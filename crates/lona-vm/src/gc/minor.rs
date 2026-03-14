// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Minor garbage collection for young generation.
//!
//! Minor GC collects the young heap, promoting live objects to the old heap.
//! This is the most frequent type of collection.
//!
//! # Algorithm
//!
//! 1. Find all roots (X registers, Y registers, process bindings)
//! 2. Copy live young heap objects to old heap
//! 3. Update root pointers to new locations
//! 4. Use Cheney's algorithm to transitively copy referenced objects
//! 5. Reset young heap (`htop = heap`)
//!
//! # Important Notes
//!
//! - The stack (between `stop` and `hend`) does NOT move during minor GC
//! - Only objects between `heap` and `htop` are considered for copying
//! - Objects already in old heap are not copied again
//! - If old heap runs out of space, returns `NeedsMajorGc`

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::process::{Process, X_REG_COUNT, Y_REGISTER_SIZE, frame_offset};
use crate::scheduler::Worker;
use crate::term::Term;

use super::copy::{Copier, copy_term};
use super::utils::{is_in_young_heap, needs_tracing};
use super::{GcError, GcStats};

/// Perform minor GC on a process.
///
/// Copies live objects from the young heap to the old heap.
/// The young heap is reset after the collection.
///
/// # Arguments
///
/// * `process` - The process to collect
/// * `worker` - The worker running this process (contains X registers)
/// * `mem` - The memory space for reading/writing heap data
///
/// # Returns
///
/// * `Ok(GcStats)` - Collection succeeded, with statistics
/// * `Err(GcError::NeedsMajorGc)` - Old heap is full, need major GC
///
/// # Errors
///
/// Returns `GcError::NeedsMajorGc` if the old heap runs out of space during
/// promotion. The caller should then trigger a major GC.
pub fn minor_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    mem: &mut M,
) -> Result<GcStats, GcError> {
    let young_used_before = process.htop.as_u64() - process.heap.as_u64();

    // Create copier targeting old heap (from current old_htop to old_hend)
    let mut copier = Copier::new(process.old_htop, process.old_hend);

    // Process roots inline — no Vec allocation.
    // Each root category is handled directly: copy the term, update the root.

    // 1. X registers (stored in Worker, no borrow conflict with Process)
    for i in 0..X_REG_COUNT {
        let term = worker.x_regs[i];
        if needs_tracing(term) && is_in_young_heap(process, term.to_vaddr()) {
            worker.x_regs[i] = copy_term(&mut copier, process, mem, term)?;
        }
    }

    // 2. Current chunk_addr (points to HeapFun on process heap)
    if let Some(chunk_addr) = process.chunk_addr {
        let term = Term::boxed_vaddr(chunk_addr);
        if needs_tracing(term) && is_in_young_heap(process, term.to_vaddr()) {
            let new_term = copy_term(&mut copier, process, mem, term)?;
            process.chunk_addr = Some(new_term.to_vaddr());
        }
    }

    // 3. Stack frames (Y registers + frame chunk_addrs)
    // Read directly from memory — no borrow conflict since mem is separate.
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

        // Frame's chunk_addr (caller's HeapFun)
        if frame_chunk_raw != 0 {
            let term = Term::boxed_vaddr(Vaddr::new(frame_chunk_raw));
            if needs_tracing(term) && is_in_young_heap(process, term.to_vaddr()) {
                let new_term = copy_term(&mut copier, process, mem, term)?;
                let chunk_addr_ptr =
                    Vaddr::new(frame_addr.as_u64() + frame_offset::CHUNK_ADDR as u64);
                mem.write(chunk_addr_ptr, new_term.to_vaddr().as_u64());
            }
        }

        // Y registers (below frame header)
        let y_base = frame_addr.as_u64() - y_count * Y_REGISTER_SIZE as u64;
        for y_idx in 0..(y_count as usize) {
            let y_addr = Vaddr::new(y_base + y_idx as u64 * Y_REGISTER_SIZE as u64);
            let term: Term = mem.read(y_addr);
            if needs_tracing(term) && is_in_young_heap(process, term.to_vaddr()) {
                let new_term = copy_term(&mut copier, process, mem, term)?;
                mem.write(y_addr, new_term);
            }
        }

        frame_opt = if caller_frame == 0 {
            None
        } else {
            Some(Vaddr::new(caller_frame))
        };
    }

    // 4. Process bindings (dynamic var values)
    // Temporarily move bindings out to avoid borrow conflict with &Process.
    // copy_term only reads heap bounds from Process, not bindings.
    let mut bindings = core::mem::take(&mut process.bindings);
    let mut binding_err = None;
    for term in bindings.values_mut() {
        if needs_tracing(*term) && is_in_young_heap(process, term.to_vaddr()) {
            match copy_term(&mut copier, process, mem, *term) {
                Ok(new_term) => *term = new_term,
                Err(e) => {
                    binding_err = Some(e);
                    break;
                }
            }
        }
    }
    process.bindings = bindings;
    if let Some(e) = binding_err {
        return Err(e);
    }

    // Run Cheney's scan loop to copy transitively referenced objects
    copier.scan_copied_objects(process, mem)?;

    // TODO: Sweep MSO list here (after Cheney's scan, BEFORE resetting young heap).
    // This must be done while forwarding pointers are still valid. The sweep:
    // 1. Updates MSO entries when their HeapProcBin is forwarded (live)
    // 2. Decrements refcount and removes entry when HeapProcBin is dead
    // Requires: Realm binary heap infrastructure (Phase 7 completion)
    // See: PLAN_GC.md Phase 7 (sweep_mso_list_minor)

    // Update old_htop to where the copier stopped
    process.old_htop = copier.alloc_ptr;

    // Reset young heap (stack stays in place!)
    // Note: stop is NOT changed - stack remains at top of young heap block
    process.htop = process.heap;

    // Update statistics
    process.minor_gc_count += 1;
    process.minor_since_major += 1;
    let live_bytes = copier.bytes_copied();
    let reclaimed_bytes = young_used_before as usize - live_bytes;
    process.total_reclaimed += reclaimed_bytes as u64;

    // Check if we should trigger a major GC based on fullsweep_after threshold
    if process.fullsweep_after > 0 && process.minor_since_major >= process.fullsweep_after {
        // Return success but signal that major GC should be triggered on next opportunity
        // The caller can check minor_since_major >= fullsweep_after after this returns
    }

    Ok(GcStats::new(live_bytes, reclaimed_bytes))
}
