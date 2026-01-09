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

mod function;
mod namespace;
pub mod pool;
mod value_alloc;

#[cfg(test)]
mod process_test;

use crate::Vaddr;
use crate::bytecode::Chunk;
use crate::value::Value;

/// Number of X registers (temporaries).
pub const X_REG_COUNT: usize = 256;

/// Maximum call stack depth.
pub const MAX_CALL_DEPTH: usize = 256;

/// Maximum number of interned keywords per process.
///
/// Keywords are interned so that identical keyword literals share the same address.
/// This enables O(1) equality comparison via address comparison.
///
/// Note: This is a temporary per-process table. Once realm-level interning is
/// implemented (Phase 4+), most keywords will be interned at the realm level,
/// and this table will only handle dynamically-constructed keywords.
pub const MAX_INTERNED_KEYWORDS: usize = 1024;

/// Maximum number of metadata entries per process.
///
/// Metadata is stored separately from objects to avoid inline overhead.
/// Most objects don't have metadata, so a separate table is more efficient.
pub const MAX_METADATA_ENTRIES: usize = 1024;

/// Maximum number of namespaces per process.
///
/// Namespaces are stored in a per-process registry. In the future, this will
/// move to the realm level for proper sharing across processes.
pub const MAX_NAMESPACES: usize = 256;

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

/// A saved call frame on the call stack.
///
/// When calling a function, we save the current execution context here
/// so it can be restored on return.
#[derive(Clone, Copy, Debug)]
pub struct CallFrame {
    /// Return address (instruction pointer to resume at).
    pub return_ip: usize,
    /// Address of the function being called (for debugging/closures).
    pub fn_addr: Vaddr,
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

    // Call stack
    /// Saved call frames for function returns.
    call_stack: [CallFrame; MAX_CALL_DEPTH],
    /// Number of frames on the call stack (stack top index).
    call_stack_len: usize,

    // Interning tables
    /// Interned keywords (addresses of keyword `HeapString`s on the heap).
    /// Keywords are interned so that identical keyword literals share the same address.
    pub(crate) keyword_intern: [Vaddr; MAX_INTERNED_KEYWORDS],
    /// Number of interned keywords.
    pub(crate) keyword_intern_len: usize,

    // Metadata table
    /// Metadata table: maps object addresses to metadata map addresses.
    /// Stored as parallel arrays: `metadata_keys[i]` → `metadata_values[i]`.
    pub(crate) metadata_keys: [Vaddr; MAX_METADATA_ENTRIES],
    pub(crate) metadata_values: [Vaddr; MAX_METADATA_ENTRIES],
    /// Number of metadata entries.
    pub(crate) metadata_len: usize,

    // Namespace registry
    /// Namespace registry: maps namespace name symbols to namespace addresses.
    /// Stored as parallel arrays: `namespace_names[i]` → `namespace_addrs[i]`.
    pub(crate) namespace_names: [Vaddr; MAX_NAMESPACES],
    pub(crate) namespace_addrs: [Vaddr; MAX_NAMESPACES],
    /// Number of registered namespaces.
    pub(crate) namespace_len: usize,
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
            // Call stack
            call_stack: [CallFrame {
                return_ip: 0,
                fn_addr: Vaddr::new(0),
            }; MAX_CALL_DEPTH],
            call_stack_len: 0,
            // Interning tables
            keyword_intern: [Vaddr::new(0); MAX_INTERNED_KEYWORDS],
            keyword_intern_len: 0,
            // Metadata table
            metadata_keys: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_values: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_len: 0,
            // Namespace registry
            namespace_names: [Vaddr::new(0); MAX_NAMESPACES],
            namespace_addrs: [Vaddr::new(0); MAX_NAMESPACES],
            namespace_len: 0,
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
        self.call_stack_len = 0;
        self.status = ProcessStatus::Ready;
    }

    // --- Call stack methods ---

    /// Push a call frame onto the stack.
    ///
    /// Returns `false` if the call stack is full (stack overflow).
    pub const fn push_call_frame(&mut self, return_ip: usize, fn_addr: Vaddr) -> bool {
        if self.call_stack_len >= MAX_CALL_DEPTH {
            return false;
        }
        self.call_stack[self.call_stack_len] = CallFrame { return_ip, fn_addr };
        self.call_stack_len += 1;
        true
    }

    /// Pop a call frame from the stack.
    ///
    /// Returns `None` if the call stack is empty.
    pub const fn pop_call_frame(&mut self) -> Option<CallFrame> {
        if self.call_stack_len == 0 {
            return None;
        }
        self.call_stack_len -= 1;
        Some(self.call_stack[self.call_stack_len])
    }

    /// Get the current call stack depth.
    #[must_use]
    pub const fn call_depth(&self) -> usize {
        self.call_stack_len
    }
}
