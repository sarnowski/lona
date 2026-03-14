// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Major garbage collection for full heap compaction.
//!
//! Major GC (fullsweep) collects both young and old generations, compacting
//! all live data into a fresh young heap. The old heap is reset to empty.
//!
//! # Algorithm
//!
//! 1. Estimate live data size and allocate fresh heaps
//! 2. Copy stack to top of new young heap
//! 3. Copy all roots from both generations
//! 4. Use Cheney's algorithm to transitively copy referenced objects
//! 5. Swap to new heaps, free old heaps
//! 6. Reset old heap (empty - all live data is in young heap)
//!
//! # When to Use
//!
//! - When old heap fills up during minor GC promotion
//! - After `fullsweep_after` minor GCs (configurable per-process)
//! - Explicitly requested via `(garbage-collect :full)` intrinsic

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::process::pool::ProcessPool;
use crate::process::{Process, X_REG_COUNT, Y_REGISTER_SIZE, frame_offset};
use crate::scheduler::Worker;
use crate::term::Term;

use super::copy::{Copier, copy_term};
use super::growth::{next_heap_size, update_stack_frame_pointers};
use super::utils::{is_in_old_heap, is_in_young_heap, needs_tracing};
use super::{GcError, GcStats};

/// Perform major GC on a process.
///
/// Compacts all live data from both young and old heaps into a fresh young heap.
/// The old heap is reset to empty after collection.
///
/// # Arguments
///
/// * `process` - The process to collect
/// * `worker` - The worker running this process (contains X registers)
/// * `pool` - The process pool for allocating new heaps
/// * `mem` - The memory space for reading/writing heap data
///
/// # Returns
///
/// * `Ok(GcStats)` - Collection succeeded, with statistics
/// * `Err(GcError::OutOfMemory)` - Cannot allocate new heaps
///
/// # Errors
///
/// Returns `GcError::OutOfMemory` if the process pool cannot allocate
/// enough memory for the new heaps.
pub fn major_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    pool: &mut ProcessPool,
    mem: &mut M,
) -> Result<GcStats, GcError> {
    // Calculate current live data estimate
    let young_used = (process.htop.as_u64() - process.heap.as_u64()) as usize;
    let old_used = (process.old_htop.as_u64() - process.old_heap.as_u64()) as usize;
    let stack_size = (process.hend.as_u64() - process.stop.as_u64()) as usize;

    // Estimate: assume we keep roughly half of allocated data plus stack
    let estimated_live = usize::midpoint(young_used, old_used) + stack_size;

    // Get next heap size that can hold the estimated live data with room to grow
    let young_capacity_words = (process.hend.as_u64() - process.heap.as_u64()) as usize / 8;
    let required_words = (estimated_live * 2).div_ceil(8); // Double for growth room
    let new_young_size_words = next_heap_size(young_capacity_words, required_words);
    let new_young_size = new_young_size_words * 8;

    // Allocate new young heap (with IPC growth if pool is exhausted)
    let new_young_block = pool
        .allocate_with_growth(new_young_size, 8)
        .ok_or(GcError::OutOfMemory)?;

    let new_young_base = new_young_block;
    let new_young_end = Vaddr::new(new_young_block.as_u64() + new_young_size as u64);

    // Copy stack to top of new young heap first
    // Save old hend for frame pointer adjustment
    let old_hend = process.hend;
    let new_stop = copy_stack(process, mem, new_young_end);

    // Update frame base and frame chain pointers to point into new stack location
    update_stack_frame_pointers(process, mem, old_hend, new_young_end);

    // Create copier targeting new young heap (below stack)
    let mut copier = Copier::new(new_young_base, new_stop);

    // Copy all roots inline (no Vec allocation)
    copy_all_roots(&mut copier, process, worker, mem)?;

    // Run Cheney's scan loop
    copier.scan_copied_objects(process, mem)?;

    // TODO: Sweep MSO list here (after Cheney's scan, BEFORE swapping heaps).
    // During major GC, all HeapProcBin objects are either forwarded or dead.
    // The sweep updates MSO entries for forwarded ProcBins and decrements
    // refcounts for dead ones (potentially freeing RefcBinary objects).
    // Requires: Realm binary heap infrastructure (Phase 7 completion)
    // See: PLAN_GC.md Phase 7 (sweep_mso_list_major)

    // Record old heap addresses for freeing
    let old_young_base = process.heap;
    let old_young_size = (process.hend.as_u64() - process.heap.as_u64()) as usize;
    let old_old_base = process.old_heap;
    let old_old_size = (process.old_hend.as_u64() - process.old_heap.as_u64()) as usize;

    // Calculate bytes copied
    let live_bytes = copier.bytes_copied();

    // Update process to use new heaps
    process.heap = new_young_base;
    process.hend = new_young_end;
    process.htop = copier.alloc_ptr;
    process.stop = new_stop;

    // Reset old heap (keep same allocation, just reset pointers)
    process.old_htop = process.old_heap;

    // Note: ProcessPool is a bump allocator that doesn't support freeing.
    // Old memory regions are abandoned and will be reclaimed when the process terminates.
    // This is acceptable because GC compacts live data, reducing overall memory usage.
    let _ = (old_young_base, old_young_size, old_old_base, old_old_size);

    // Update statistics
    process.major_gc_count += 1;
    let total_before = young_used + old_used;
    let reclaimed_bytes = total_before.saturating_sub(live_bytes);
    process.total_reclaimed += reclaimed_bytes as u64;

    // Reset minor GC counter (for fullsweep_after tracking)
    process.minor_since_major = 0;

    Ok(GcStats::new(live_bytes, reclaimed_bytes))
}

/// Copy all roots that point to young or old heap, updating them in place.
fn copy_all_roots<M: MemorySpace>(
    copier: &mut Copier,
    process: &mut Process,
    worker: &mut Worker,
    mem: &mut M,
) -> Result<(), GcError> {
    // 1. X registers (stored in Worker)
    for i in 0..X_REG_COUNT {
        let term = worker.x_regs[i];
        if needs_tracing(term) {
            let addr = term.to_vaddr();
            if is_in_young_heap(process, addr) || is_in_old_heap(process, addr) {
                worker.x_regs[i] = copy_term(copier, process, mem, term)?;
            }
        }
    }

    // 2. Current chunk_addr
    if let Some(chunk_addr) = process.chunk_addr {
        let term = Term::boxed_vaddr(chunk_addr);
        if needs_tracing(term) {
            let addr = term.to_vaddr();
            if is_in_young_heap(process, addr) || is_in_old_heap(process, addr) {
                let new_term = copy_term(copier, process, mem, term)?;
                process.chunk_addr = Some(new_term.to_vaddr());
            }
        }
    }

    // 3. Stack frames (Y registers + frame chunk_addrs)
    copy_stack_frame_roots(copier, process, mem)?;

    // 4. Process bindings
    let mut bindings = core::mem::take(&mut process.bindings);
    let mut binding_err = None;
    for term in bindings.values_mut() {
        if needs_tracing(*term) {
            let addr = term.to_vaddr();
            if is_in_young_heap(process, addr) || is_in_old_heap(process, addr) {
                match copy_term(copier, process, mem, *term) {
                    Ok(new_term) => *term = new_term,
                    Err(e) => {
                        binding_err = Some(e);
                        break;
                    }
                }
            }
        }
    }
    process.bindings = bindings;
    if let Some(e) = binding_err {
        return Err(e);
    }

    Ok(())
}

/// Copy stack frame roots (Y registers and frame `chunk_addr` slots).
fn copy_stack_frame_roots<M: MemorySpace>(
    copier: &mut Copier,
    process: &Process,
    mem: &mut M,
) -> Result<(), GcError> {
    let mut frame_opt = process.frame_base;
    while let Some(frame_addr) = frame_opt {
        let y_count: u64 = mem.read(Vaddr::new(
            frame_addr.as_u64() + frame_offset::Y_COUNT as u64,
        ));
        let caller_frame: u64 = mem.read(Vaddr::new(
            frame_addr.as_u64() + frame_offset::CALLER_FRAME_BASE as u64,
        ));
        let frame_chunk_raw: u64 = mem.read(Vaddr::new(
            frame_addr.as_u64() + frame_offset::CHUNK_ADDR as u64,
        ));

        // Frame's chunk_addr
        if frame_chunk_raw != 0 {
            let term = Term::boxed_vaddr(Vaddr::new(frame_chunk_raw));
            if needs_tracing(term) {
                let addr = term.to_vaddr();
                if is_in_young_heap(process, addr) || is_in_old_heap(process, addr) {
                    let new_term = copy_term(copier, process, mem, term)?;
                    let chunk_addr_ptr =
                        Vaddr::new(frame_addr.as_u64() + frame_offset::CHUNK_ADDR as u64);
                    mem.write(chunk_addr_ptr, new_term.to_vaddr().as_u64());
                }
            }
        }

        // Y registers (below frame header)
        let y_base = frame_addr.as_u64() - y_count * Y_REGISTER_SIZE as u64;
        for y_idx in 0..(y_count as usize) {
            let y_addr = Vaddr::new(y_base + y_idx as u64 * Y_REGISTER_SIZE as u64);
            let term: Term = mem.read(y_addr);
            if needs_tracing(term) {
                let addr = term.to_vaddr();
                if is_in_young_heap(process, addr) || is_in_old_heap(process, addr) {
                    let new_term = copy_term(copier, process, mem, term)?;
                    mem.write(y_addr, new_term);
                }
            }
        }

        frame_opt = if caller_frame == 0 {
            None
        } else {
            Some(Vaddr::new(caller_frame))
        };
    }
    Ok(())
}

/// Copy the stack from old young heap to top of new young heap.
///
/// Stack grows downward from `hend` to `stop`. This copies the stack
/// bytes to the corresponding position in the new heap.
///
/// Returns the new `stop` value in the new heap.
fn copy_stack<M: MemorySpace>(process: &Process, mem: &mut M, new_hend: Vaddr) -> Vaddr {
    let stack_size = process.hend.as_u64() - process.stop.as_u64();

    if stack_size == 0 {
        return new_hend;
    }

    let new_stop = Vaddr::new(new_hend.as_u64() - stack_size);

    // Copy stack bytes
    for offset in 0..stack_size {
        let src = Vaddr::new(process.stop.as_u64() + offset);
        let dst = Vaddr::new(new_stop.as_u64() + offset);
        let byte: u8 = mem.read(src);
        mem.write(dst, byte);
    }

    new_stop
}
