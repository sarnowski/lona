// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Process memory model for BEAM-style lightweight processes.
//!
//! Each process has its own heap (for allocations) and execution state.
//! The heap uses the BEAM two-heap architecture:
//!
//! ```text
//! YOUNG HEAP (single contiguous block):
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                                                                    │
//! │   HEAP                             FREE                  STACK     │
//! │   (grows up)                      SPACE                 (grows     │
//! │                                                          down)     │
//! │                                                                    │
//! │   [cons][string]◄─htop                   stop─►[frame1][frame0]    │
//! │        ↑                                              ↓            │
//! │                                                                    │
//! └────────────────────────────────────────────────────────────────────┘
//! ▲                                                                    ▲
//! heap (low address)                                      hend (high address)
//!
//! Out of memory when: htop >= stop
//! For M2: Return error (no GC yet)
//!
//! OLD HEAP (separate block, for future GC):
//! ┌────────────────────────────────────────────────────────────────────┐
//! │   [promoted][promoted]                    │         FREE           │
//! │                                           │◄─ old_htop             │
//! └────────────────────────────────────────────────────────────────────┘
//! ▲                                                                    ▲
//! old_heap                                                        old_hend
//!
//! For M2: Allocated but empty (no promotion without GC)
//! ```

pub mod pool;

#[cfg(test)]
mod process_test;

use crate::Vaddr;
use crate::bytecode::Chunk;
use crate::platform::MemorySpace;
use crate::value::{HeapString, Pair, Value};

/// Number of X registers (temporaries).
pub const X_REG_COUNT: usize = 256;

/// Initial young heap size (48 KB).
pub const INITIAL_YOUNG_HEAP_SIZE: usize = 48 * 1024;

/// Initial old heap size (12 KB).
pub const INITIAL_OLD_HEAP_SIZE: usize = 12 * 1024;

/// Process execution status.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessStatus {
    /// Process is ready to run.
    Ready = 0,
    /// Process is currently running.
    Running = 1,
    /// Process has completed execution.
    Completed = 2,
    /// Process encountered an error.
    Error = 3,
}

/// A lightweight process with BEAM-style memory layout.
///
/// Each process owns its heap and execution state. The VM operates on
/// a process reference rather than owning the state itself.
#[repr(C)]
pub struct Process {
    // Identity
    /// Process identifier.
    pub pid: u64,
    /// Current execution status.
    pub status: ProcessStatus,

    // Young heap (stack grows down, heap grows up)
    /// Base (low address) of the young heap.
    pub heap: Vaddr,
    /// End (high address) of the young heap.
    pub hend: Vaddr,
    /// Heap top pointer (grows UP toward hend).
    pub htop: Vaddr,
    /// Stack pointer (grows DOWN toward heap).
    pub stop: Vaddr,

    // Old heap (for future GC)
    /// Base of the old heap.
    pub old_heap: Vaddr,
    /// End of the old heap.
    pub old_hend: Vaddr,
    /// Old heap allocation pointer.
    pub old_htop: Vaddr,

    // Execution state
    /// Instruction pointer (index into bytecode).
    pub ip: usize,
    /// X registers (temporaries).
    pub x_regs: [Value; X_REG_COUNT],
    /// Current bytecode chunk being executed.
    pub chunk: Option<Chunk>,
}

impl Process {
    /// Create a new process with the given heap regions.
    ///
    /// # Arguments
    /// * `pid` - Process identifier
    /// * `young_base` - Base address of young heap (low address)
    /// * `young_size` - Size of young heap in bytes
    /// * `old_base` - Base address of old heap
    /// * `old_size` - Size of old heap in bytes
    #[must_use]
    pub const fn new(
        pid: u64,
        young_base: Vaddr,
        young_size: usize,
        old_base: Vaddr,
        old_size: usize,
    ) -> Self {
        let young_end = Vaddr::new(young_base.as_u64() + young_size as u64);
        let old_end = Vaddr::new(old_base.as_u64() + old_size as u64);

        Self {
            pid,
            status: ProcessStatus::Ready,
            // Young heap: htop starts at base (grows up), stop starts at end (grows down)
            heap: young_base,
            hend: young_end,
            htop: young_base,
            stop: young_end,
            // Old heap: empty, htop at base
            old_heap: old_base,
            old_hend: old_end,
            old_htop: old_base,
            // Execution state
            ip: 0,
            x_regs: [Value::Nil; X_REG_COUNT],
            chunk: None,
        }
    }

    /// Allocate bytes from the young heap (grows up).
    ///
    /// Returns `None` if there isn't enough space.
    pub const fn alloc(&mut self, size: usize, align: usize) -> Option<Vaddr> {
        if size == 0 {
            return Some(self.htop);
        }

        // Align htop up
        let mask = (align as u64).wrapping_sub(1);
        let aligned = (self.htop.as_u64() + mask) & !mask;
        let new_htop = aligned + size as u64;

        // Check collision with stack
        if new_htop > self.stop.as_u64() {
            return None; // OOM - in future, trigger GC
        }

        let result = Vaddr::new(aligned);
        self.htop = Vaddr::new(new_htop);
        Some(result)
    }

    /// Push bytes onto the stack (grows down).
    ///
    /// Returns `None` if there isn't enough space.
    pub const fn stack_push(&mut self, size: usize, align: usize) -> Option<Vaddr> {
        // Align stop down
        let mask = (align as u64).wrapping_sub(1);
        let new_stop = (self.stop.as_u64() - size as u64) & !mask;

        // Check collision with heap
        if new_stop < self.htop.as_u64() {
            return None; // OOM
        }

        self.stop = Vaddr::new(new_stop);
        Some(self.stop)
    }

    /// Pop bytes from the stack (grows down).
    pub fn stack_pop(&mut self, size: usize) {
        let new_stop = self.stop.as_u64() + size as u64;
        // Don't grow past hend
        self.stop = Vaddr::new(new_stop.min(self.hend.as_u64()));
    }

    /// Returns remaining free space (between htop and stop).
    #[must_use]
    pub const fn free_space(&self) -> usize {
        self.stop.as_u64().saturating_sub(self.htop.as_u64()) as usize
    }

    /// Returns the number of bytes used in the young heap.
    #[must_use]
    pub const fn heap_used(&self) -> usize {
        self.htop.as_u64().saturating_sub(self.heap.as_u64()) as usize
    }

    /// Returns the number of bytes used in the stack.
    #[must_use]
    pub const fn stack_used(&self) -> usize {
        self.hend.as_u64().saturating_sub(self.stop.as_u64()) as usize
    }

    /// Set the bytecode chunk to execute.
    pub fn set_chunk(&mut self, chunk: Chunk) {
        self.chunk = Some(chunk);
        self.ip = 0;
    }

    /// Reset execution state for a new evaluation.
    pub const fn reset(&mut self) {
        self.ip = 0;
        self.x_regs = [Value::Nil; X_REG_COUNT];
        self.status = ProcessStatus::Ready;
    }

    // --- Value allocation helpers ---

    /// Allocate a string on the young heap.
    ///
    /// Returns a `Value::String` pointing to the allocated string, or `None` if OOM.
    pub fn alloc_string<M: MemorySpace>(&mut self, mem: &mut M, s: &str) -> Option<Value> {
        let len = s.len();
        let total_size = HeapString::alloc_size(len);

        // Allocate space (align to 4 bytes for the header)
        let addr = self.alloc(total_size, 4)?;

        // Write header
        let header = HeapString { len: len as u32 };
        mem.write(addr, header);

        // Write string data
        let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
        let dest = mem.slice_mut(data_addr, len);
        dest.copy_from_slice(s.as_bytes());

        Some(Value::string(addr))
    }

    /// Allocate a pair on the young heap.
    ///
    /// Returns a `Value::Pair` pointing to the allocated pair, or `None` if OOM.
    pub fn alloc_pair<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        first: Value,
        rest: Value,
    ) -> Option<Value> {
        // Allocate space (align to 8 bytes for Value fields)
        let addr = self.alloc(Pair::SIZE, 8)?;

        // Write the pair
        let pair = Pair::new(first, rest);
        mem.write(addr, pair);

        Some(Value::pair(addr))
    }

    /// Allocate a symbol on the young heap (same as string but tagged differently).
    ///
    /// Returns a `Value::Symbol` pointing to the allocated symbol, or `None` if OOM.
    pub fn alloc_symbol<M: MemorySpace>(&mut self, mem: &mut M, name: &str) -> Option<Value> {
        let len = name.len();
        let total_size = HeapString::alloc_size(len);

        // Allocate space (align to 4 bytes for the header)
        let addr = self.alloc(total_size, 4)?;

        // Write header
        let header = HeapString { len: len as u32 };
        mem.write(addr, header);

        // Write string data
        let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
        let dest = mem.slice_mut(data_addr, len);
        dest.copy_from_slice(name.as_bytes());

        Some(Value::symbol(addr))
    }

    /// Read a heap-allocated string.
    ///
    /// Returns `None` if the value is not a string or symbol.
    #[must_use]
    pub fn read_string<'a, M: MemorySpace>(&self, mem: &'a M, value: Value) -> Option<&'a str> {
        let (Value::String(addr) | Value::Symbol(addr)) = value else {
            return None;
        };

        let header: HeapString = mem.read(addr);
        let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
        let bytes = mem.slice(data_addr, header.len as usize);

        // We wrote valid UTF-8 when creating the string, but return None on error
        core::str::from_utf8(bytes).ok()
    }

    /// Read a pair from the heap.
    ///
    /// Returns `None` if the value is not a pair.
    #[must_use]
    pub fn read_pair<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<Pair> {
        let Value::Pair(addr) = value else {
            return None;
        };

        Some(mem.read(addr))
    }
}
