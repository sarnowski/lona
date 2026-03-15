// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Root finding for garbage collection.
//!
//! GC roots are the starting points for tracing. They include:
//! - X registers (temporaries, owned by Worker)
//! - Y registers (locals on stack frames)
//! - Process bindings (dynamic var bindings)
//! - Execution state (`chunk_addr`, etc.)
//!
//! The `RootIterator` yields all root Terms that `needs_tracing()`.
//! After GC copies objects, `update_root()` updates the root to point
//! to the new location.

#[cfg(any(test, feature = "std"))]
use std::vec::Vec;

#[cfg(not(any(test, feature = "std")))]
use alloc::vec::Vec;

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::process::{Process, X_REG_COUNT, Y_REGISTER_SIZE, frame_offset};
use crate::scheduler::Worker;
use crate::term::Term;

use super::utils::needs_tracing;

/// Location of a root for updating after GC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootLocation {
    /// X register (index `0..X_REG_COUNT`).
    XRegister(usize),
    /// Y register in a specific stack frame.
    YRegister {
        /// Address of the frame header.
        frame_addr: Vaddr,
        /// Index of the Y register within the frame.
        y_index: usize,
    },
    /// Process binding (var address -> value).
    ProcessBinding(Vaddr),
    /// Current chunk address (`process.chunk_addr`).
    ChunkAddr,
    /// Cached PID term (`process.pid_term`).
    PidTerm,
    /// Caller's chunk address in a stack frame.
    FrameChunkAddr {
        /// Address of the frame header.
        frame_addr: Vaddr,
    },
    /// Saved chunk address in an eval stack frame.
    EvalChunkAddr {
        /// Index into the eval stack.
        eval_index: usize,
    },
}

/// Iterator over all GC roots in a process.
///
/// Yields `(RootLocation, Term)` pairs for each root that `needs_tracing()`.
/// Roots that don't need tracing (immediates) are skipped.
pub struct RootIterator<'a> {
    process: &'a Process,
    worker: &'a Worker,
    phase: RootPhase,
}

/// Current phase of root iteration.
enum RootPhase {
    /// Scanning X registers.
    XRegisters { index: usize },
    /// Scanning process bindings.
    ProcessBindings {
        /// Iterator state - stored as index into sorted keys.
        /// Using explicit index since we can't store iterators easily.
        index: usize,
    },
    /// Done iterating.
    Done,
}

impl<'a> RootIterator<'a> {
    /// Create a new root iterator for a process and its worker.
    ///
    /// This iterates over:
    /// 1. X registers (in the worker)
    /// 2. Process bindings (dynamic var values)
    ///
    /// Note: Y registers require `MemorySpace` access. Use `iterate_roots_with_mem`
    /// for complete root scanning including Y registers.
    #[must_use]
    pub const fn new(process: &'a Process, worker: &'a Worker) -> Self {
        Self {
            process,
            worker,
            phase: RootPhase::XRegisters { index: 0 },
        }
    }
}

impl Iterator for RootIterator<'_> {
    type Item = (RootLocation, Term);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.phase {
                RootPhase::XRegisters { index } => {
                    // Scan X registers
                    while *index < X_REG_COUNT {
                        let i = *index;
                        *index += 1;

                        let term = self.worker.x_regs[i];
                        if needs_tracing(term) {
                            return Some((RootLocation::XRegister(i), term));
                        }
                    }

                    // Move to bindings (Y registers require MemorySpace)
                    self.phase = RootPhase::ProcessBindings { index: 0 };
                }

                RootPhase::ProcessBindings { index } => {
                    // Process bindings are stored in a BTreeMap
                    // Iterate through keys
                    let keys: Vec<_> = self.process.bindings.keys().copied().collect();

                    while *index < keys.len() {
                        let i = *index;
                        *index += 1;

                        let var_addr = keys[i];
                        if let Some(&term) = self.process.bindings.get(&var_addr) {
                            if needs_tracing(term) {
                                return Some((RootLocation::ProcessBinding(var_addr), term));
                            }
                        }
                    }

                    self.phase = RootPhase::Done;
                }

                RootPhase::Done => {
                    return None;
                }
            }
        }
    }
}

/// Iterate over all roots including Y registers (requires `MemorySpace`).
///
/// This is the complete root scanning function that includes stack frames.
/// The callback receives each root location and term.
pub fn iterate_roots_with_mem<M, F>(process: &Process, worker: &Worker, mem: &M, mut callback: F)
where
    M: MemorySpace,
    F: FnMut(RootLocation, Term),
{
    // 1. X registers
    for (i, &term) in worker.x_regs.iter().enumerate() {
        if needs_tracing(term) {
            callback(RootLocation::XRegister(i), term);
        }
    }

    // 2. Current chunk address (process.chunk_addr)
    if let Some(chunk_addr) = process.chunk_addr {
        // The chunk_addr points to a HeapFun on the process heap
        let term = Term::boxed_vaddr(chunk_addr);
        if needs_tracing(term) {
            callback(RootLocation::ChunkAddr, term);
        }
    }

    // 3. Y registers and frame chunk addresses (stack frames)
    trace_frame_chain(process.frame_base, mem, &mut callback);

    // 4. Cached PID term
    if let Some(pid_term) = process.pid_term {
        if needs_tracing(pid_term) {
            callback(RootLocation::PidTerm, pid_term);
        }
    }

    // 5. Eval stack: saved chunk addresses and pre-eval stack frames
    for eval_index in 0..process.eval_depth {
        let eval_frame = &process.eval_stack[eval_index];

        if let Some(chunk_addr) = eval_frame.saved_chunk_addr {
            let term = Term::boxed_vaddr(chunk_addr);
            if needs_tracing(term) {
                callback(RootLocation::EvalChunkAddr { eval_index }, term);
            }
        }

        // Trace stack frames unreachable from current frame_base during eval
        trace_frame_chain(eval_frame.saved_frame_base, mem, &mut callback);
    }

    // 6. Process bindings
    for (&var_addr, &term) in &process.bindings {
        if needs_tracing(term) {
            callback(RootLocation::ProcessBinding(var_addr), term);
        }
    }
}

/// Update a root after GC has moved the object.
///
/// This updates the root location to point to the new address.
/// X registers are updated in the worker, bindings in the process.
/// Trace a chain of stack frames starting from `frame_base`, calling the
/// callback for each Y register and frame chunk address that needs tracing.
fn trace_frame_chain<M, F>(mut frame_opt: Option<Vaddr>, mem: &M, callback: &mut F)
where
    M: MemorySpace,
    F: FnMut(RootLocation, Term),
{
    while let Some(frame_addr) = frame_opt {
        let y_count: u64 = mem.read(Vaddr::new(
            frame_addr.as_u64() + frame_offset::Y_COUNT as u64,
        ));
        let caller_frame: u64 = mem.read(Vaddr::new(
            frame_addr.as_u64() + frame_offset::CALLER_FRAME_BASE as u64,
        ));
        let frame_chunk_addr: u64 = mem.read(Vaddr::new(
            frame_addr.as_u64() + frame_offset::CHUNK_ADDR as u64,
        ));

        if frame_chunk_addr != 0 {
            let term = Term::boxed_vaddr(Vaddr::new(frame_chunk_addr));
            if needs_tracing(term) {
                callback(RootLocation::FrameChunkAddr { frame_addr }, term);
            }
        }

        let y_base = frame_addr.as_u64() - y_count * Y_REGISTER_SIZE as u64;
        for y_idx in 0..(y_count as usize) {
            let y_addr = Vaddr::new(y_base + y_idx as u64 * Y_REGISTER_SIZE as u64);
            let term: Term = mem.read(y_addr);
            if needs_tracing(term) {
                callback(
                    RootLocation::YRegister {
                        frame_addr,
                        y_index: y_idx,
                    },
                    term,
                );
            }
        }

        frame_opt = if caller_frame == 0 {
            None
        } else {
            Some(Vaddr::new(caller_frame))
        };
    }
}

///
/// Note: For Y register roots, use `update_root_with_mem` instead.
pub fn update_root(
    process: &mut Process,
    worker: &mut Worker,
    location: &RootLocation,
    new_term: Term,
) {
    match location {
        RootLocation::XRegister(index) => {
            worker.x_regs[*index] = new_term;
        }
        RootLocation::YRegister { .. } | RootLocation::FrameChunkAddr { .. } => {
            // These need MemorySpace - this should use update_root_with_mem
            // Silently ignore here since it would be a programming error to call
            // this function with these location types without memory access.
            debug_assert!(
                false,
                "Use update_root_with_mem for Y register and frame chunk_addr updates"
            );
        }
        RootLocation::ProcessBinding(var_addr) => {
            process.bindings.insert(*var_addr, new_term);
        }
        RootLocation::ChunkAddr => {
            // Update process.chunk_addr
            process.chunk_addr = Some(new_term.to_vaddr());
        }
        RootLocation::PidTerm => {
            process.pid_term = Some(new_term);
        }
        RootLocation::EvalChunkAddr { eval_index } => {
            process.eval_stack[*eval_index].saved_chunk_addr = Some(new_term.to_vaddr());
        }
    }
}

/// Update a root after GC, including Y registers (requires `MemorySpace`).
///
/// This is the complete update function that can handle all root types.
pub fn update_root_with_mem<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    mem: &mut M,
    location: &RootLocation,
    new_term: Term,
) {
    match location {
        RootLocation::XRegister(index) => {
            worker.x_regs[*index] = new_term;
        }
        RootLocation::YRegister {
            frame_addr,
            y_index,
        } => {
            update_root_y(process, mem, *frame_addr, *y_index, new_term);
        }
        RootLocation::ProcessBinding(var_addr) => {
            process.bindings.insert(*var_addr, new_term);
        }
        RootLocation::ChunkAddr => {
            process.chunk_addr = Some(new_term.to_vaddr());
        }
        RootLocation::FrameChunkAddr { frame_addr } => {
            // Update chunk_addr slot in frame header
            let chunk_addr_ptr = Vaddr::new(frame_addr.as_u64() + frame_offset::CHUNK_ADDR as u64);
            mem.write(chunk_addr_ptr, new_term.to_vaddr().as_u64());
        }
        RootLocation::PidTerm => {
            process.pid_term = Some(new_term);
        }
        RootLocation::EvalChunkAddr { eval_index } => {
            process.eval_stack[*eval_index].saved_chunk_addr = Some(new_term.to_vaddr());
        }
    }
}

/// Update a Y register root after GC (requires `MemorySpace`).
pub fn update_root_y<M: MemorySpace>(
    _process: &Process,
    mem: &mut M,
    frame_addr: Vaddr,
    y_index: usize,
    new_term: Term,
) {
    // Read y_count from frame header
    let y_count: u64 = mem.read(Vaddr::new(
        frame_addr.as_u64() + frame_offset::Y_COUNT as u64,
    ));

    // Y registers are BELOW the frame header
    let y_base = frame_addr.as_u64() - y_count * Y_REGISTER_SIZE as u64;
    let y_addr = Vaddr::new(y_base + y_index as u64 * Y_REGISTER_SIZE as u64);

    mem.write(y_addr, new_term);
}

/// Collect all roots into a vector (for testing/debugging).
///
/// This is a convenience function that collects all roots.
/// For production GC, use `iterate_roots_with_mem` to avoid allocation.
#[cfg(any(test, feature = "std"))]
pub fn collect_roots<M: MemorySpace>(
    process: &Process,
    worker: &Worker,
    mem: &M,
) -> std::vec::Vec<(RootLocation, Term)> {
    let mut roots = std::vec::Vec::new();
    iterate_roots_with_mem(process, worker, mem, |loc, term| {
        roots.push((loc, term));
    });
    roots
}
