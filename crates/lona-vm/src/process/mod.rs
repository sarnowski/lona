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
mod var;

#[cfg(test)]
mod allocation_test;
#[cfg(test)]
mod binding_test;
#[cfg(test)]
mod copy_test;
#[cfg(test)]
mod execution_test;
#[cfg(test)]
mod namespace_test;
#[cfg(test)]
mod pool_test;
#[cfg(test)]
mod reduction_test;
#[cfg(test)]
mod stack_test;
#[cfg(test)]
mod value_alloc_test;

use crate::Vaddr;
use crate::bytecode::Chunk;
use crate::value::Value;

#[cfg(any(test, feature = "std"))]
use std::vec::Vec;

#[cfg(not(any(test, feature = "std")))]
use alloc::vec::Vec;

/// Number of X registers (temporaries).
pub const X_REG_COUNT: usize = 256;

/// Maximum call stack depth.
pub const MAX_CALL_DEPTH: usize = 256;

/// Maximum number of interned keywords per process.
///
/// Keywords are interned so that identical keyword literals share the same address.
/// This enables O(1) equality comparison via address comparison.
///
/// Note: This per-process table is used during compilation and REPL evaluation.
/// Realm-level interning (see `realm::Realm`) handles persistent keywords that
/// are part of `def`'d code. This table handles dynamically-constructed keywords.
pub const MAX_INTERNED_KEYWORDS: usize = 1024;

/// Maximum number of interned symbols per process.
///
/// Symbols are interned so that identical symbol literals share the same address.
/// This enables O(1) equality comparison via address comparison and is required
/// for namespace lookups (which compare symbol addresses).
///
/// Note: This per-process table is used during compilation and REPL evaluation.
/// Realm-level interning (see `realm::Realm`) handles persistent symbols that
/// are part of `def`'d code.
pub const MAX_INTERNED_SYMBOLS: usize = 1024;

/// Maximum number of metadata entries per process.
///
/// Metadata is stored separately from objects to avoid inline overhead.
/// Most objects don't have metadata, so a separate table is more efficient.
pub const MAX_METADATA_ENTRIES: usize = 1024;

/// Maximum number of namespaces per process.
///
/// Namespaces are stored in a per-process registry for REPL and compilation.
/// Realm-level namespace registry (see `realm::Realm`) handles persistent
/// namespaces shared across processes. This table is used during bootstrap
/// and for temporary namespace operations.
pub const MAX_NAMESPACES: usize = 256;

/// Maximum number of process-bound variable bindings.
///
/// Process-bound vars (like `*ns*`) can have per-process values that shadow
/// the root binding. This table maps var IDs (`VarSlot` addresses) to values.
pub const MAX_BINDINGS: usize = 256;

/// Initial young heap size (48 KB).
pub const INITIAL_YOUNG_HEAP_SIZE: usize = 48 * 1024;

/// Initial old heap size (12 KB).
pub const INITIAL_OLD_HEAP_SIZE: usize = 12 * 1024;

/// Maximum reductions per time slice.
///
/// Tuned for ~500µs execution time to fit within typical MCS budgets.
/// See `docs/architecture/process-model.md` for scheduling details.
pub const MAX_REDUCTIONS: u32 = 2000;

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

/// A saved call frame on the VM call stack.
///
/// Contains all state needed to resume the caller after the callee returns.
/// When calling a function, we save the current execution context here
/// so it can be restored on return.
#[derive(Clone, Debug)]
pub struct CallFrame {
    /// Return address (instruction pointer to resume at).
    pub return_ip: usize,
    /// The caller's bytecode chunk.
    pub chunk: Chunk,
    /// Address of the function being called (for debugging/closures).
    pub fn_addr: Vaddr,
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
    /// Process identifier.
    pub pid: u64,
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
    /// Uses `Vec` because `CallFrame` contains `Chunk` which is not `Copy`.
    call_stack: Vec<CallFrame>,

    // Interning tables
    /// Interned keywords (addresses of keyword `HeapString`s on the heap).
    /// Keywords are interned so that identical keyword literals share the same address.
    pub(crate) keyword_intern: [Vaddr; MAX_INTERNED_KEYWORDS],
    /// Number of interned keywords.
    pub(crate) keyword_intern_len: usize,
    /// Interned symbols (addresses of symbol `HeapString`s on the heap).
    /// Symbols are interned so that identical symbol literals share the same address.
    pub(crate) symbol_intern: [Vaddr; MAX_INTERNED_SYMBOLS],
    /// Number of interned symbols.
    pub(crate) symbol_intern_len: usize,

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

    // Process-bound var bindings
    /// Var IDs (`VarSlot` addresses) with process-local bindings.
    pub(crate) binding_var_ids: [Vaddr; MAX_BINDINGS],
    /// Process-local values for those vars.
    pub(crate) binding_values: [Value; MAX_BINDINGS],
    /// Number of bindings.
    pub(crate) binding_len: usize,
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
    // Vec::new() is not const in alloc crate (no_std mode)
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec::new() is not const in no_std"
    )]
    #[must_use]
    pub fn new(
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
            // Execution state
            ip: 0,
            x_regs: [Value::Nil; X_REG_COUNT],
            chunk: None,
            // Call stack
            call_stack: Vec::new(),
            // Interning tables
            keyword_intern: [Vaddr::new(0); MAX_INTERNED_KEYWORDS],
            keyword_intern_len: 0,
            symbol_intern: [Vaddr::new(0); MAX_INTERNED_SYMBOLS],
            symbol_intern_len: 0,
            // Metadata table
            metadata_keys: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_values: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_len: 0,
            // Namespace registry
            namespace_names: [Vaddr::new(0); MAX_NAMESPACES],
            namespace_addrs: [Vaddr::new(0); MAX_NAMESPACES],
            namespace_len: 0,
            // Process-bound var bindings
            binding_var_ids: [Vaddr::new(0); MAX_BINDINGS],
            binding_values: [Value::Nil; MAX_BINDINGS],
            binding_len: 0,
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
    pub fn reset(&mut self) {
        self.ip = 0;
        self.x_regs = [Value::Nil; X_REG_COUNT];
        self.call_stack.clear();
        self.status = ProcessStatus::Ready;
    }

    // --- Call stack methods ---

    /// Push a call frame before entering a function.
    ///
    /// Takes ownership of the current chunk (moves it to the stack).
    ///
    /// # Errors
    ///
    /// Returns `Err(StackOverflow)` if:
    /// - Call stack is full (reached `MAX_CALL_DEPTH`)
    /// - No current chunk exists (process has no code)
    pub fn push_call_frame(&mut self, fn_addr: Vaddr) -> Result<(), StackOverflow> {
        if self.call_stack.len() >= MAX_CALL_DEPTH {
            return Err(StackOverflow);
        }

        // Take the current chunk - it goes on the stack
        let Some(caller_chunk) = self.chunk.take() else {
            return Err(StackOverflow);
        };

        self.call_stack.push(CallFrame {
            return_ip: self.ip,
            chunk: caller_chunk,
            fn_addr,
        });
        Ok(())
    }

    /// Pop a call frame after returning from a function.
    ///
    /// Restores the caller's chunk and IP.
    /// Returns `false` if at top level (call stack empty).
    pub fn pop_call_frame(&mut self) -> bool {
        let Some(frame) = self.call_stack.pop() else {
            return false;
        };

        self.ip = frame.return_ip;
        self.chunk = Some(frame.chunk);
        true
    }

    /// Check if at top level (no active calls).
    #[must_use]
    pub fn at_top_level(&self) -> bool {
        self.call_stack.is_empty()
    }

    /// Get the current call stack depth.
    #[must_use]
    pub fn call_depth(&self) -> usize {
        self.call_stack.len()
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

    // --- Process-bound var bindings ---

    /// Get process-local binding for a var, if any.
    ///
    /// Returns the bound value if the var has a process-local binding,
    /// `None` otherwise.
    #[must_use]
    pub fn get_binding(&self, var_id: Vaddr) -> Option<Value> {
        for i in 0..self.binding_len {
            if self.binding_var_ids[i] == var_id {
                return Some(self.binding_values[i]);
            }
        }
        None
    }

    /// Set process-local binding for a var.
    ///
    /// If the var already has a binding, updates it.
    /// Otherwise, adds a new binding.
    ///
    /// Returns `None` if the binding table is full.
    pub fn set_binding(&mut self, var_id: Vaddr, value: Value) -> Option<()> {
        // Check if already bound - update in place
        for i in 0..self.binding_len {
            if self.binding_var_ids[i] == var_id {
                self.binding_values[i] = value;
                return Some(());
            }
        }

        // Add new binding
        if self.binding_len >= MAX_BINDINGS {
            return None; // Table full
        }

        self.binding_var_ids[self.binding_len] = var_id;
        self.binding_values[self.binding_len] = value;
        self.binding_len += 1;
        Some(())
    }

    /// Check if a var has a process-local binding.
    #[must_use]
    pub fn has_binding(&self, var_id: Vaddr) -> bool {
        for i in 0..self.binding_len {
            if self.binding_var_ids[i] == var_id {
                return true;
            }
        }
        false
    }

    /// Bootstrap this process with initial bindings from a realm.
    ///
    /// Sets up the `*ns*` binding to point to `lona.core` namespace.
    /// This should be called after `realm::bootstrap()` has initialized the realm.
    ///
    /// # Arguments
    /// * `ns_var` - The `*ns*` var from the realm (returned by `realm::bootstrap`)
    /// * `core_ns` - The `lona.core` namespace (returned by `realm::bootstrap`)
    pub fn bootstrap(&mut self, ns_var: Value, core_ns: Value) {
        if let Value::Var(var_id) = ns_var {
            // Set *ns* to lona.core for this process
            let _ = self.set_binding(var_id, core_ns);
        }
    }
}
