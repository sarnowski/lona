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
//! │   [pair][string]◄─htop                   stop─►[frame1][frame0]    │
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
mod term_alloc;

#[cfg(test)]
mod allocation_test;
#[cfg(test)]
mod execution_test;
#[cfg(test)]
mod frame_test;
#[cfg(test)]
mod pool_test;
#[cfg(test)]
mod process_id_test;
#[cfg(test)]
mod reduction_test;
#[cfg(test)]
mod stack_test;
#[cfg(test)]
mod term_alloc_test;

extern crate alloc;

use alloc::collections::BTreeMap;

use crate::Vaddr;
use crate::term::Term;

/// Number of X registers (temporaries).
pub const X_REG_COUNT: usize = 256;

/// Initial young heap size (4 KB, 512 words).
///
/// GC handles growth automatically via the Fibonacci sequence in `gc/growth.rs`.
/// The allocation path in `gc/alloc.rs` triggers minor GC, heap growth, and
/// major GC as needed. See `docs/architecture/garbage-collection.md`.
pub const INITIAL_YOUNG_HEAP_SIZE: usize = 4 * 1024;

/// Initial old heap size (1 KB, 128 words).
///
/// Old heap holds objects promoted during minor GC. Grows automatically
/// when promotion fills it (triggers major GC or heap growth).
pub const INITIAL_OLD_HEAP_SIZE: usize = 1024;

/// Maximum reductions per time slice.
///
/// Tuned for ~500µs execution time to fit within typical MCS budgets.
/// See `docs/architecture/process-model.md` for scheduling details.
pub const MAX_REDUCTIONS: u32 = 2000;

// ============================================================================
// Stack Frame Constants
// ============================================================================
//
// Stack frames are stored in the process stack region (grows down from `stop`).
// Each frame has a fixed-size header followed by Y registers for local variables.
//
// Frame layout (from low to high addresses):
//
//     stop (after ALLOCATE)
//     ┌─────────────────────────────────────────────────────────────────┐
//     │ Y(0)             ← stop + 0 * size_of::<Value>()    (16 bytes) │
//     │ Y(1)             ← stop + 1 * size_of::<Value>()    (16 bytes) │
//     │ ...                                                            │
//     │ Y(N-1)           ← stop + (N-1) * size_of::<Value>()(16 bytes) │
//     ├────────────────────────────────────────────────────────────────┤ ← frame_base
//     │ return_ip        ← frame_base + 0                   (u64)      │
//     │ chunk_addr       ← frame_base + 8                   (u64)      │
//     │ caller_frame_base← frame_base + 16 (0 if top level) (u64)      │
//     │ y_count          ← frame_base + 24                  (u64)      │
//     └────────────────────────────────────────────────────────────────┘
//     (higher addresses - previous frame above)
//
// Key relationships:
// - frame_base points to the header (return_ip slot)
// - Y registers are BELOW the header (at lower addresses)
// - Y registers are Term-sized (8 bytes), header fields are u64-sized (8 bytes)
// - stop = frame_base - y_count * Y_REGISTER_SIZE
// - Y(i) = stop + i * Y_REGISTER_SIZE

/// Size of one Y register slot in bytes.
/// Y registers store `Term`s, so this must equal `size_of::<Term>()`.
pub const Y_REGISTER_SIZE: usize = core::mem::size_of::<crate::term::Term>();

/// Size of stack frame header in bytes.
/// Contains: `return_ip`, `chunk_addr`, `caller_frame_base`, `y_count` (4 × u64).
pub const FRAME_HEADER_SIZE: usize = 4 * core::mem::size_of::<u64>();

/// Maximum Y registers per frame.
/// BEAM allows up to 1024, but most functions use < 16.
/// We use 64 as a reasonable limit that fits in 6 bits.
pub const MAX_Y_REGISTERS: usize = 64;

/// Minimum stack space to keep free (safety margin for GC, interrupts).
pub const STACK_REDZONE: usize = 256;

/// Offsets within frame header (from `frame_base`).
pub mod frame_offset {
    /// Offset of return instruction pointer.
    pub const RETURN_IP: usize = 0;
    /// Offset of chunk address (for reloading caller's chunk on return).
    pub const CHUNK_ADDR: usize = 8;
    /// Offset of caller's `frame_base` (0 if top level).
    pub const CALLER_FRAME_BASE: usize = 16;
    /// Offset of Y register count.
    pub const Y_COUNT: usize = 24;
}

// ============================================================================
// Process Identity Types
// ============================================================================

/// Process identifier with generation counter for ABA safety.
///
/// When a slot is reused, generation increments to invalidate stale references.
/// This prevents the ABA problem where a freed slot is reallocated and an old
/// reference incorrectly accesses the new process.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProcessId {
    /// Slot index in process table.
    index: u32,
    /// Incremented on slot reuse.
    generation: u32,
}

impl ProcessId {
    /// The null process ID (invalid reference).
    pub const NULL: Self = Self {
        index: u32::MAX,
        generation: 0,
    };

    /// Create a new process ID.
    #[inline]
    #[must_use]
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get the slot index.
    #[inline]
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index as usize
    }

    /// Get the generation counter.
    #[inline]
    #[must_use]
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Check if this is the null process ID.
    #[inline]
    #[must_use]
    pub const fn is_null(&self) -> bool {
        self.index == u32::MAX
    }
}

impl core::fmt::Debug for ProcessId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_null() {
            write!(f, "ProcessId::NULL")
        } else {
            write!(f, "ProcessId({}, gen={})", self.index, self.generation)
        }
    }
}

/// Worker identifier (0 to MAX_WORKERS-1).
///
/// Each worker has its own run queue and can execute one process at a time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WorkerId(pub u8);

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

/// Stack overflow error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackOverflow;

/// A lightweight process with BEAM-style memory layout.
///
/// Each process owns its heap and execution state. The VM operates on
/// a process reference rather than owning the state itself.
#[repr(C)]
pub struct Process {
    // Identity
    /// Process identifier with generation counter.
    pub pid: ProcessId,
    /// Parent process identifier (NULL for root processes).
    pub parent_pid: ProcessId,
    /// Worker this process is assigned to.
    pub worker_id: WorkerId,
    /// Current execution status.
    pub status: ProcessStatus,

    // Reduction counting
    /// Remaining reductions before yield.
    pub reductions: u32,
    /// Total reductions executed by this process (for monitoring).
    pub total_reductions: u64,

    // Young heap (stack grows down, heap grows up)
    /// Base (low address) of the young heap.
    pub heap: Vaddr,
    /// End (high address) of the young heap.
    pub hend: Vaddr,
    /// Heap top pointer (grows UP toward hend).
    pub htop: Vaddr,
    /// Stack pointer (grows DOWN toward heap).
    pub stop: Vaddr,

    // Old heap (for GC)
    /// Base of the old heap.
    pub old_heap: Vaddr,
    /// End of the old heap.
    pub old_hend: Vaddr,
    /// Old heap allocation pointer.
    pub old_htop: Vaddr,

    // GC configuration
    /// Trigger major GC after this many minor GCs (0 = never auto-trigger).
    pub fullsweep_after: u32,

    // GC statistics
    /// Number of minor GCs performed.
    pub minor_gc_count: u64,
    /// Number of major (fullsweep) GCs performed.
    pub major_gc_count: u64,
    /// Total bytes reclaimed by GC.
    pub total_reclaimed: u64,
    /// Minor GCs since last major GC (for `fullsweep_after` tracking).
    pub minor_since_major: u32,

    // Execution state
    /// Instruction pointer (index into bytecode).
    pub ip: usize,
    /// Address of current `HeapFun` being executed.
    ///
    /// The VM reads bytecode and constants directly from this address
    /// via `HeapFun::read_instruction` and `HeapFun::read_constant`,
    /// avoiding per-call Vec allocation.
    pub chunk_addr: Option<Vaddr>,

    // Stack-based frame tracking
    /// Base of the innermost (current) stack frame.
    /// Points to the `return_ip` slot of the current frame header.
    /// `None` if at top level (no active call).
    pub frame_base: Option<Vaddr>,
    /// Number of Y registers in the current frame (cached for fast access).
    pub current_y_count: usize,
    /// Current call depth (number of frames on stack).
    frame_depth: usize,

    // Process-bound var bindings
    /// Bindings (var address -> value).
    pub(crate) bindings: BTreeMap<Vaddr, Term>,
}

impl Process {
    /// Create a new process with the given heap regions.
    ///
    /// The process starts with NULL pid and `parent_pid`, and `worker_id` 0.
    /// The scheduler assigns the actual `ProcessId` when spawning.
    ///
    /// # Arguments
    /// * `young_base` - Base address of young heap (low address)
    /// * `young_size` - Size of young heap in bytes
    /// * `old_base` - Base address of old heap
    /// * `old_size` - Size of old heap in bytes
    #[must_use]
    pub const fn new(
        young_base: Vaddr,
        young_size: usize,
        old_base: Vaddr,
        old_size: usize,
    ) -> Self {
        let young_end = Vaddr::new(young_base.as_u64() + young_size as u64);
        let old_end = Vaddr::new(old_base.as_u64() + old_size as u64);

        Self {
            pid: ProcessId::NULL,
            parent_pid: ProcessId::NULL,
            worker_id: WorkerId(0),
            status: ProcessStatus::Ready,
            // Reduction counting - starts at 0, must call reset_reductions() before run
            reductions: 0,
            total_reductions: 0,
            // Young heap: htop starts at base (grows up), stop starts at end (grows down)
            heap: young_base,
            hend: young_end,
            htop: young_base,
            stop: young_end,
            // Old heap: empty, htop at base
            old_heap: old_base,
            old_hend: old_end,
            old_htop: old_base,
            // GC configuration (0 = never auto-trigger major GC)
            fullsweep_after: 0,
            // GC statistics
            minor_gc_count: 0,
            major_gc_count: 0,
            total_reclaimed: 0,
            minor_since_major: 0,
            // Execution state
            ip: 0,
            chunk_addr: None,
            // Stack-based frame tracking
            frame_base: None,
            current_y_count: 0,
            frame_depth: 0,
            // Process-bound var bindings
            bindings: BTreeMap::new(),
        }
    }

    /// Allocate bytes from the young heap (grows up).
    ///
    /// Returns `None` if there isn't enough space.
    ///
    /// Callers must request appropriate alignment for the type being allocated.
    /// For Term types, use 8-byte alignment. For strings, 4-byte is sufficient.
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

    /// Set the `HeapFun` address to execute from.
    ///
    /// The VM reads bytecode and constants directly from this address.
    pub const fn set_chunk_addr(&mut self, addr: Vaddr) {
        self.chunk_addr = Some(addr);
        self.ip = 0;
    }

    /// Reset execution state for a new evaluation.
    ///
    /// Note: X registers are now owned by Worker, not Process.
    /// The caller is responsible for resetting `Worker.x_regs` if needed.
    pub const fn reset(&mut self) {
        self.ip = 0;
        self.chunk_addr = None;
        // Reset stack-based frame tracking
        self.frame_base = None;
        self.current_y_count = 0;
        self.frame_depth = 0;
        // Reset stack pointer to top of young heap
        self.stop = self.hend;
        self.status = ProcessStatus::Ready;
    }

    /// Reset heap allocation pointer, clearing all heap allocations.
    ///
    /// This is useful for E2E tests to prevent heap exhaustion.
    /// WARNING: Invalidates all previously allocated values!
    pub const fn reset_heap(&mut self) {
        self.htop = self.heap;
    }

    /// Check if at top level (no active call frames).
    #[must_use]
    pub const fn at_top_level(&self) -> bool {
        self.frame_base.is_none()
    }

    /// Get the current call stack depth.
    #[must_use]
    pub const fn call_depth(&self) -> usize {
        self.frame_depth
    }

    // --- Stack-based frame methods ---

    /// Allocate a new stack frame for a function call.
    ///
    /// Creates a frame header in the stack region. The frame starts with
    /// no Y registers; use `extend_frame_y_regs` to add them.
    ///
    /// # Arguments
    /// * `mem` - Memory space for writing frame header
    /// * `return_ip` - Instruction pointer to resume at on return
    /// * `chunk_addr` - Address of caller's `CompiledFn` (for reloading chunk on return)
    ///
    /// # Errors
    ///
    /// Returns `Err(StackOverflow)` if insufficient stack space.
    pub fn allocate_frame<M: crate::platform::MemorySpace>(
        &mut self,
        mem: &mut M,
        return_ip: usize,
        chunk_addr: Vaddr,
    ) -> Result<(), StackOverflow> {
        // Check stack space (with redzone)
        let new_stop = self
            .stop
            .as_u64()
            .checked_sub(FRAME_HEADER_SIZE as u64)
            .ok_or(StackOverflow)?;

        if new_stop < self.htop.as_u64() + STACK_REDZONE as u64 {
            return Err(StackOverflow);
        }

        // Save caller's frame_base (0 if at top level)
        let caller_frame_base = self.frame_base.map_or(0, Vaddr::as_u64);

        // Allocate frame
        self.stop = Vaddr::new(new_stop);
        self.frame_base = Some(self.stop);
        self.current_y_count = 0;
        self.frame_depth += 1;

        // Write frame header
        let base = self.stop.as_u64();
        mem.write(
            Vaddr::new(base + frame_offset::RETURN_IP as u64),
            return_ip as u64,
        );
        mem.write(
            Vaddr::new(base + frame_offset::CHUNK_ADDR as u64),
            chunk_addr.as_u64(),
        );
        mem.write(
            Vaddr::new(base + frame_offset::CALLER_FRAME_BASE as u64),
            caller_frame_base,
        );
        mem.write(Vaddr::new(base + frame_offset::Y_COUNT as u64), 0u64);

        Ok(())
    }

    /// Deallocate the current frame and restore caller's context.
    ///
    /// Returns `None` if at top level, otherwise returns `(return_ip, chunk_addr)`.
    pub fn deallocate_frame<M: crate::platform::MemorySpace>(
        &mut self,
        mem: &M,
    ) -> Option<(usize, Vaddr)> {
        let frame_base = self.frame_base?;

        // Read frame header
        let base = frame_base.as_u64();
        let return_ip: u64 = mem.read(Vaddr::new(base + frame_offset::RETURN_IP as u64));
        let chunk_addr: u64 = mem.read(Vaddr::new(base + frame_offset::CHUNK_ADDR as u64));
        let caller_frame_base: u64 =
            mem.read(Vaddr::new(base + frame_offset::CALLER_FRAME_BASE as u64));

        // Restore caller's context
        self.frame_depth = self.frame_depth.saturating_sub(1);

        if caller_frame_base == 0 {
            // Top level - no caller frame
            self.frame_base = None;
            self.current_y_count = 0;
            self.stop = self.hend;
        } else {
            // Restore caller's frame
            let caller_fb = Vaddr::new(caller_frame_base);
            self.frame_base = Some(caller_fb);

            // Read caller's y_count from its header
            let caller_y_count: u64 =
                mem.read(Vaddr::new(caller_frame_base + frame_offset::Y_COUNT as u64));
            self.current_y_count = caller_y_count as usize;

            // Restore stop: frame_base - y_count * slot_size
            self.stop = Vaddr::new(caller_frame_base - caller_y_count * Y_REGISTER_SIZE as u64);
        }

        Some((return_ip as usize, Vaddr::new(chunk_addr)))
    }

    /// Extend the current frame with Y registers.
    ///
    /// Allocates space for Y registers below the frame header.
    /// Y registers are NOT initialized (use `extend_frame_y_regs_zero` for that).
    ///
    /// # Errors
    ///
    /// Returns `Err(StackOverflow)` if:
    /// - No frame exists
    /// - Too many Y registers requested
    /// - Insufficient stack space
    pub fn extend_frame_y_regs<M: crate::platform::MemorySpace>(
        &mut self,
        mem: &mut M,
        y_count: usize,
    ) -> Result<(), StackOverflow> {
        let frame_base = self.frame_base.ok_or(StackOverflow)?;

        if y_count > MAX_Y_REGISTERS {
            return Err(StackOverflow);
        }

        // Calculate new stop position
        let y_space = y_count * Y_REGISTER_SIZE;
        let new_stop = frame_base
            .as_u64()
            .checked_sub(y_space as u64)
            .ok_or(StackOverflow)?;

        if new_stop < self.htop.as_u64() + STACK_REDZONE as u64 {
            return Err(StackOverflow);
        }

        // Update state
        self.stop = Vaddr::new(new_stop);
        self.current_y_count = y_count;

        // Update y_count in frame header
        mem.write(
            Vaddr::new(frame_base.as_u64() + frame_offset::Y_COUNT as u64),
            y_count as u64,
        );

        Ok(())
    }

    /// Extend the current frame with Y registers and initialize them to nil.
    ///
    /// This is GC-safe: all Y registers are valid from the start.
    ///
    /// # Errors
    ///
    /// Returns `Err(StackOverflow)` if extension fails.
    pub fn extend_frame_y_regs_zero<M: crate::platform::MemorySpace>(
        &mut self,
        mem: &mut M,
        y_count: usize,
    ) -> Result<(), StackOverflow> {
        self.extend_frame_y_regs(mem, y_count)?;

        // Initialize Y registers to nil
        for i in 0..y_count {
            let y_addr = Vaddr::new(self.stop.as_u64() + i as u64 * Y_REGISTER_SIZE as u64);
            mem.write(y_addr, crate::term::Term::NIL);
        }

        Ok(())
    }

    /// Shrink the current frame by releasing Y registers.
    ///
    /// # Arguments
    /// * `y_count` - Number of Y registers to release (must match current count)
    ///
    /// # Errors
    ///
    /// Returns `Err(StackOverflow)` if no frame exists or count mismatch.
    pub fn shrink_frame_y_regs<M: crate::platform::MemorySpace>(
        &mut self,
        mem: &mut M,
        y_count: usize,
    ) -> Result<(), StackOverflow> {
        let frame_base = self.frame_base.ok_or(StackOverflow)?;

        if y_count != self.current_y_count {
            return Err(StackOverflow);
        }

        // Move stop back to frame_base
        self.stop = frame_base;
        self.current_y_count = 0;

        // Update y_count in frame header
        mem.write(
            Vaddr::new(frame_base.as_u64() + frame_offset::Y_COUNT as u64),
            0u64,
        );

        Ok(())
    }

    /// Get Y register value from current frame.
    ///
    /// Returns `None` if index out of bounds or no frame exists.
    #[inline]
    #[must_use]
    pub fn get_y<M: crate::platform::MemorySpace>(
        &self,
        mem: &M,
        index: usize,
    ) -> Option<crate::term::Term> {
        if self.frame_base.is_none() || index >= self.current_y_count {
            return None;
        }

        let y_addr = Vaddr::new(self.stop.as_u64() + index as u64 * Y_REGISTER_SIZE as u64);
        Some(mem.read(y_addr))
    }

    /// Set Y register value in current frame.
    ///
    /// Returns `false` if index out of bounds or no frame exists.
    #[inline]
    pub fn set_y<M: crate::platform::MemorySpace>(
        &self,
        mem: &mut M,
        index: usize,
        value: crate::term::Term,
    ) -> bool {
        if self.frame_base.is_none() || index >= self.current_y_count {
            return false;
        }

        let y_addr = Vaddr::new(self.stop.as_u64() + index as u64 * Y_REGISTER_SIZE as u64);
        mem.write(y_addr, value);
        true
    }

    // --- Reduction counting methods ---

    /// Reset reduction budget for a new time slice.
    pub const fn reset_reductions(&mut self) {
        self.reductions = MAX_REDUCTIONS;
    }

    /// Consume reductions. Returns false if budget exhausted.
    ///
    /// If the cost exceeds remaining budget, consumes all remaining reductions
    /// and returns false. The `total_reductions` counter is always updated
    /// with the actual amount consumed.
    pub fn consume_reductions(&mut self, cost: u32) -> bool {
        if self.reductions >= cost {
            self.reductions -= cost;
            self.total_reductions = self.total_reductions.wrapping_add(u64::from(cost));
            true
        } else {
            let remaining = self.reductions;
            self.reductions = 0;
            self.total_reductions = self.total_reductions.wrapping_add(u64::from(remaining));
            false
        }
    }

    /// Check if budget is exhausted.
    #[must_use]
    pub const fn should_yield(&self) -> bool {
        self.reductions == 0
    }

    // --- Chunk management methods ---

    /// Serialize a compiled Chunk onto the process heap and set `chunk_addr`.
    ///
    /// This is used by the REPL and compiler to convert a `Chunk` (with Vecs)
    /// into a `HeapFun` on the process heap. After this call, the VM reads
    /// bytecode directly from the heap address.
    ///
    /// Returns `true` if successful, `false` if out of memory.
    pub fn write_chunk_to_heap<M: crate::platform::MemorySpace>(
        &mut self,
        mem: &mut M,
        chunk: &crate::bytecode::Chunk,
    ) -> bool {
        let Some(fn_term) = self.alloc_term_compiled_fn(
            mem,
            0,     // arity (not used for REPL/top-level code)
            false, // variadic
            0,     // num_locals
            chunk.code(),
            chunk.constants(),
        ) else {
            return false;
        };

        self.chunk_addr = Some(fn_term.to_vaddr());
        self.ip = 0;
        true
    }

    /// Bootstrap this process with initial bindings from a realm.
    ///
    /// Sets up the `*ns*` binding to point to `lona.core` namespace.
    /// This should be called after `realm::bootstrap()` has initialized the realm.
    ///
    /// # Arguments
    /// * `ns_var` - The `*ns*` var from the realm (returned by `realm::bootstrap`)
    /// * `core_ns` - The `lona.core` namespace (returned by `realm::bootstrap`)
    pub fn bootstrap(&mut self, ns_var: Term, core_ns: Term) {
        if ns_var.is_boxed() {
            // Set *ns* to lona.core for this process
            let var_addr = ns_var.to_vaddr();
            self.bindings.insert(var_addr, core_ns);
        }
    }
}
