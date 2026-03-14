// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Heap growth for when GC cannot reclaim enough space.
//!
//! When a minor GC completes but the young heap still doesn't have enough
//! space for allocation, the heap needs to grow. This module provides:
//!
//! - Fibonacci-like heap size sequence (matching BEAM)
//! - Young heap growth with stack relocation
//! - Old heap growth for promotion overflow
//!
//! # Heap Size Sequence
//!
//! Heap sizes follow a Fibonacci-like sequence to provide gradual growth
//! that balances memory efficiency with GC frequency. Each size is
//! approximately the sum of the previous two sizes.
//!
//! # Young Heap Growth Algorithm
//!
//! 1. Allocate new, larger heap region
//! 2. Run GC to find live objects (and compact them)
//! 3. Copy live objects to bottom of new heap
//! 4. Copy stack to top of new heap
//! 5. Update all pointers (roots, htop, stop, heap, hend)
//! 6. Old memory is abandoned (bump allocator doesn't free)

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::process::pool::ProcessPool;
use crate::process::{Process, X_REG_COUNT, Y_REGISTER_SIZE, frame_offset};
use crate::scheduler::Worker;
use crate::term::Term;
use crate::term::header::Header;

use super::copy::Copier;
use super::utils::needs_tracing;
use super::{GcError, GcStats};

/// Fibonacci-like heap sizes in words (8 bytes each).
///
/// These match BEAM's heap size sequence for compatibility and proven
/// performance characteristics. The sequence provides gradual growth
/// that balances memory usage with GC frequency.
///
/// Values in bytes: multiply by 8 (e.g., 233 words = 1864 bytes).
pub const HEAP_SIZES: [usize; 24] = [
    233,        // ~1.8 KB
    377,        // ~3.0 KB
    610,        // ~4.9 KB
    987,        // ~7.9 KB
    1_597,      // ~12.8 KB
    2_584,      // ~20.7 KB
    4_181,      // ~33.4 KB
    6_765,      // ~54.1 KB
    10_946,     // ~87.6 KB
    17_711,     // ~141.7 KB
    28_657,     // ~229.3 KB
    46_368,     // ~371.0 KB
    75_025,     // ~600.2 KB
    121_393,    // ~971.1 KB
    196_418,    // ~1.6 MB
    317_811,    // ~2.5 MB
    514_229,    // ~4.1 MB
    832_040,    // ~6.7 MB
    1_346_269,  // ~10.8 MB
    2_178_309,  // ~17.4 MB
    3_524_578,  // ~28.2 MB
    5_702_887,  // ~45.6 MB
    9_227_465,  // ~73.8 MB
    14_930_352, // ~119.4 MB
];

/// Get the next heap size that can accommodate the required size.
///
/// Returns the smallest size from `HEAP_SIZES` that is:
/// 1. Larger than `current_words`
/// 2. At least as large as `required_words`
///
/// If no standard size fits, returns `required_words` rounded up to the
/// nearest size that is larger than `current_words` (at least 1.5x current).
#[must_use]
pub fn next_heap_size(current_words: usize, required_words: usize) -> usize {
    // Find smallest size that satisfies both constraints
    for &size in &HEAP_SIZES {
        if size > current_words && size >= required_words {
            return size;
        }
    }

    // No standard size fits - calculate a larger size
    // Use at least 1.5x current or required, whichever is larger
    let minimum = current_words
        .saturating_add(current_words / 2)
        .max(required_words);

    // Round up to next power of 2 for alignment benefits (optional)
    minimum.next_power_of_two().max(minimum)
}

/// Grow the old heap to accommodate more promoted objects.
///
/// This allocates a new, larger old heap region from the pool,
/// copies existing data, and updates process pointers.
///
/// # Arguments
///
/// * `process` - The process whose old heap needs growing
/// * `pool` - The process pool for allocating new memory
/// * `mem` - Memory space for copying data
/// * `required_bytes` - Minimum size needed for the new old heap
///
/// # Errors
///
/// Returns `GcError::OutOfMemory` if the pool cannot allocate enough memory.
pub fn grow_old_heap<M: MemorySpace>(
    process: &mut Process,
    pool: &mut ProcessPool,
    mem: &mut M,
    required_bytes: usize,
) -> Result<GcStats, GcError> {
    let old_base = process.old_heap;
    let old_htop = process.old_htop;
    let current_size = (process.old_hend.as_u64() - old_base.as_u64()) as usize;
    let used_size = (old_htop.as_u64() - old_base.as_u64()) as usize;

    // Calculate new size
    let current_words = current_size / 8;
    let required_words = required_bytes / 8;
    let new_words = next_heap_size(current_words, required_words);
    let new_size = new_words * 8;

    // Allocate new old heap region (with IPC growth if pool is exhausted)
    let new_base = pool
        .allocate_with_growth(new_size, 8)
        .ok_or(GcError::OutOfMemory)?;
    let new_hend = Vaddr::new(new_base.as_u64() + new_size as u64);

    // Copy existing data from old heap to new heap
    for offset in 0..used_size {
        let src = Vaddr::new(old_base.as_u64() + offset as u64);
        let dst = Vaddr::new(new_base.as_u64() + offset as u64);
        let byte: u8 = mem.read(src);
        mem.write(dst, byte);
    }

    // Update process pointers
    process.old_heap = new_base;
    process.old_htop = Vaddr::new(new_base.as_u64() + used_size as u64);
    process.old_hend = new_hend;

    // Note: Old memory is not freed (bump allocator doesn't support free)
    // It will be reclaimed when the process terminates

    Ok(GcStats::new(used_size, 0))
}

/// Grow the young heap, performing GC to compact live data first.
///
/// This is the primary heap growth function. It:
/// 1. Allocates a new, larger young heap
/// 2. Runs GC to find and copy live objects to the new heap
/// 3. Copies the stack to the top of the new heap
/// 4. Updates all root pointers
///
/// # Arguments
///
/// * `process` - The process whose young heap needs growing
/// * `worker` - The worker (contains X registers)
/// * `pool` - The process pool for allocating new memory
/// * `mem` - Memory space for copying data
/// * `required_bytes` - Minimum size needed for the new young heap
///
/// # Errors
///
/// Returns `GcError::OutOfMemory` if the pool cannot allocate enough memory.
pub fn grow_young_heap_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    pool: &mut ProcessPool,
    mem: &mut M,
    required_bytes: usize,
) -> Result<GcStats, GcError> {
    let old_heap = process.heap;
    let old_hend = process.hend;
    let current_size = (old_hend.as_u64() - old_heap.as_u64()) as usize;

    // Calculate new size
    let current_words = current_size / 8;
    let required_words = required_bytes / 8;
    let new_words = next_heap_size(current_words, required_words);
    let new_size = new_words * 8;

    // Allocate new young heap region (with IPC growth if pool is exhausted)
    let new_heap = pool
        .allocate_with_growth(new_size, 8)
        .ok_or(GcError::OutOfMemory)?;
    let new_hend = Vaddr::new(new_heap.as_u64() + new_size as u64);

    // Calculate stack size (stack is at top of heap, grows down)
    let stack_size = (old_hend.as_u64() - process.stop.as_u64()) as usize;

    // New stack will be at top of new heap
    let new_stop = Vaddr::new(new_hend.as_u64() - stack_size as u64);

    // Copy stack to new location first (before we modify anything)
    for offset in 0..stack_size {
        let src = Vaddr::new(process.stop.as_u64() + offset as u64);
        let dst = Vaddr::new(new_stop.as_u64() + offset as u64);
        let byte: u8 = mem.read(src);
        mem.write(dst, byte);
    }

    // Create copier targeting the new young heap (objects at bottom)
    let mut copier = Copier::new(new_heap, new_stop);

    // Copy all roots inline (no Vec allocation)
    copy_roots_for_growth(
        &mut copier,
        process,
        worker,
        mem,
        old_heap,
        old_hend,
        new_hend,
    )?;

    // Run Cheney's scan loop
    scan_copied_objects_for_growth(&mut copier, mem, old_heap, old_hend)?;

    // Update Y registers in copied stack frames
    // Stack frame pointers also need updating since the stack moved
    update_stack_frame_pointers(process, mem, old_hend, new_hend);

    // Update process pointers
    process.heap = new_heap;
    process.htop = copier.alloc_ptr;
    process.stop = new_stop;
    process.hend = new_hend;

    // Note: Old memory is not freed (bump allocator doesn't support free)

    let live_bytes = copier.bytes_copied();
    Ok(GcStats::new(live_bytes, 0))
}

/// Copy all roots that point to the old young heap, updating them in place.
///
/// Stack frame roots need address translation because the stack has been
/// copied to a new location (`old_hend` -> `new_hend`).
fn copy_roots_for_growth<M: MemorySpace>(
    copier: &mut Copier,
    process: &mut Process,
    worker: &mut Worker,
    mem: &mut M,
    old_heap: Vaddr,
    old_hend: Vaddr,
    new_hend: Vaddr,
) -> Result<(), GcError> {
    // 1. X registers
    for i in 0..X_REG_COUNT {
        let term = worker.x_regs[i];
        if needs_tracing(term) {
            let addr = term.to_vaddr();
            if addr.as_u64() >= old_heap.as_u64() && addr.as_u64() < old_hend.as_u64() {
                worker.x_regs[i] = copy_term_for_growth(copier, mem, term, old_heap, old_hend)?;
            }
        }
    }

    // 2. chunk_addr
    if let Some(chunk_addr) = process.chunk_addr {
        let term = Term::boxed_vaddr(chunk_addr);
        if needs_tracing(term) {
            let addr = term.to_vaddr();
            if addr.as_u64() >= old_heap.as_u64() && addr.as_u64() < old_hend.as_u64() {
                let new_term = copy_term_for_growth(copier, mem, term, old_heap, old_hend)?;
                process.chunk_addr = Some(new_term.to_vaddr());
            }
        }
    }

    // 3. Stack frames (read from old locations, write to new translated locations)
    copy_stack_frame_roots_for_growth(copier, process, mem, old_heap, old_hend, new_hend)?;

    // 4. Process bindings
    let mut bindings = core::mem::take(&mut process.bindings);
    let mut binding_err = None;
    for term in bindings.values_mut() {
        if needs_tracing(*term) {
            let addr = term.to_vaddr();
            if addr.as_u64() >= old_heap.as_u64() && addr.as_u64() < old_hend.as_u64() {
                match copy_term_for_growth(copier, mem, *term, old_heap, old_hend) {
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

/// Copy stack frame roots with address translation for relocated stack.
///
/// Reads from old frame addresses, writes to translated new frame addresses.
fn copy_stack_frame_roots_for_growth<M: MemorySpace>(
    copier: &mut Copier,
    process: &Process,
    mem: &mut M,
    old_heap: Vaddr,
    old_hend: Vaddr,
    new_hend: Vaddr,
) -> Result<(), GcError> {
    let mut frame_opt = process.frame_base;
    while let Some(old_frame_addr) = frame_opt {
        // Translate frame address to new stack location
        let offset_from_end = old_hend.as_u64() - old_frame_addr.as_u64();
        let new_frame_addr = Vaddr::new(new_hend.as_u64() - offset_from_end);

        let y_count: u64 = mem.read(Vaddr::new(
            old_frame_addr.as_u64() + frame_offset::Y_COUNT as u64,
        ));
        let caller_frame: u64 = mem.read(Vaddr::new(
            old_frame_addr.as_u64() + frame_offset::CALLER_FRAME_BASE as u64,
        ));
        let frame_chunk_raw: u64 = mem.read(Vaddr::new(
            old_frame_addr.as_u64() + frame_offset::CHUNK_ADDR as u64,
        ));

        // Frame's chunk_addr
        if frame_chunk_raw != 0 {
            let term = Term::boxed_vaddr(Vaddr::new(frame_chunk_raw));
            if needs_tracing(term) {
                let addr = term.to_vaddr();
                if addr.as_u64() >= old_heap.as_u64() && addr.as_u64() < old_hend.as_u64() {
                    let new_term = copy_term_for_growth(copier, mem, term, old_heap, old_hend)?;
                    let chunk_addr_ptr =
                        Vaddr::new(new_frame_addr.as_u64() + frame_offset::CHUNK_ADDR as u64);
                    mem.write(chunk_addr_ptr, new_term.to_vaddr().as_u64());
                }
            }
        }

        // Y registers (below frame header, use translated addresses for writes)
        let old_y_base = old_frame_addr.as_u64() - y_count * Y_REGISTER_SIZE as u64;
        let new_y_base = new_frame_addr.as_u64() - y_count * Y_REGISTER_SIZE as u64;
        for y_idx in 0..(y_count as usize) {
            let old_y_addr = Vaddr::new(old_y_base + y_idx as u64 * Y_REGISTER_SIZE as u64);
            let new_y_addr = Vaddr::new(new_y_base + y_idx as u64 * Y_REGISTER_SIZE as u64);
            let term: Term = mem.read(old_y_addr);
            if needs_tracing(term) {
                let addr = term.to_vaddr();
                if addr.as_u64() >= old_heap.as_u64() && addr.as_u64() < old_hend.as_u64() {
                    let new_term = copy_term_for_growth(copier, mem, term, old_heap, old_hend)?;
                    mem.write(new_y_addr, new_term);
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

/// Copy a term from old young heap to new young heap.
fn copy_term_for_growth<M: MemorySpace>(
    copier: &mut Copier,
    mem: &mut M,
    term: Term,
    old_heap: Vaddr,
    old_hend: Vaddr,
) -> Result<Term, GcError> {
    if !needs_tracing(term) {
        return Ok(term);
    }

    let addr = term.to_vaddr();
    // Only copy if in old young heap
    if addr.as_u64() < old_heap.as_u64() || addr.as_u64() >= old_hend.as_u64() {
        return Ok(term);
    }

    // Use the copy module's copy functions
    if term.is_list() {
        copy_pair_for_growth(copier, mem, term)
    } else if term.is_boxed() {
        copy_boxed_for_growth(copier, mem, term)
    } else {
        Ok(term)
    }
}

/// Copy a pair from old heap to new heap.
fn copy_pair_for_growth<M: MemorySpace>(
    copier: &mut Copier,
    mem: &mut M,
    term: Term,
) -> Result<Term, GcError> {
    use crate::term::pair::Pair;

    let addr = term.to_vaddr();
    let pair: Pair = mem.read(addr);

    // Check if already forwarded
    if pair.is_forwarded() {
        let new_addr = pair.forward_address();
        return Ok(Term::list_vaddr(Vaddr::new(new_addr as u64)));
    }

    // Allocate in new heap
    let new_addr = copier.allocate(Pair::SIZE).ok_or(GcError::OutOfMemory)?;

    // Copy the pair
    mem.write(new_addr, pair);

    // Leave forwarding pointer
    let mut forwarded = pair;
    unsafe {
        forwarded.set_forward(new_addr.as_u64() as *const Pair);
    }
    mem.write(addr, forwarded);

    Ok(Term::list_vaddr(new_addr))
}

/// Copy a boxed object from old heap to new heap.
fn copy_boxed_for_growth<M: MemorySpace>(
    copier: &mut Copier,
    mem: &mut M,
    term: Term,
) -> Result<Term, GcError> {
    let addr = term.to_vaddr();
    let header: Header = mem.read(addr);

    // Check if already forwarded
    if header.is_forward() {
        let new_addr = header.forward_address();
        return Ok(Term::boxed_vaddr(Vaddr::new(new_addr as u64)));
    }

    // Calculate size
    let size = header.object_size();

    // Allocate in new heap
    let new_addr = copier.allocate(size).ok_or(GcError::OutOfMemory)?;

    // Copy the object
    for offset in 0..size {
        let src = Vaddr::new(addr.as_u64() + offset as u64);
        let dst = Vaddr::new(new_addr.as_u64() + offset as u64);
        let byte: u8 = mem.read(src);
        mem.write(dst, byte);
    }

    // Leave forwarding header
    let forward_header = Header::forward(new_addr.as_u64() as *const u8);
    mem.write(addr, forward_header);

    Ok(Term::boxed_vaddr(new_addr))
}

/// Scan copied objects in the new heap, copying any referenced objects.
fn scan_copied_objects_for_growth<M: MemorySpace>(
    copier: &mut Copier,
    mem: &mut M,
    old_heap: Vaddr,
    old_hend: Vaddr,
) -> Result<(), GcError> {
    use crate::term::pair::Pair;

    while copier.scan_ptr.as_u64() < copier.alloc_ptr.as_u64() {
        let first_word: u64 = mem.read(copier.scan_ptr);
        let first_term = unsafe { Term::from_raw(first_word) };

        if first_term.is_header() {
            // Boxed object
            let header = Header::from_raw(first_word);
            let obj_size = header.object_size();
            scan_boxed_for_growth(copier, mem, header, old_heap, old_hend)?;
            copier.scan_ptr = Vaddr::new(copier.scan_ptr.as_u64() + obj_size as u64);
        } else {
            // Pair
            scan_pair_for_growth(copier, mem, old_heap, old_hend)?;
            copier.scan_ptr = Vaddr::new(copier.scan_ptr.as_u64() + Pair::SIZE as u64);
        }
    }
    Ok(())
}

/// Scan a pair and copy any referenced objects.
fn scan_pair_for_growth<M: MemorySpace>(
    copier: &mut Copier,
    mem: &mut M,
    old_heap: Vaddr,
    old_hend: Vaddr,
) -> Result<(), GcError> {
    use crate::term::pair::Pair;

    let pair_addr = copier.scan_ptr;
    let pair: Pair = mem.read(pair_addr);

    // Update head if needed
    if needs_tracing(pair.head) {
        let addr = pair.head.to_vaddr();
        if addr.as_u64() >= old_heap.as_u64() && addr.as_u64() < old_hend.as_u64() {
            let new_head = copy_term_for_growth(copier, mem, pair.head, old_heap, old_hend)?;
            mem.write(pair_addr, new_head);
        }
    }

    // Update rest if needed
    let rest_addr = Vaddr::new(pair_addr.as_u64() + 8);
    if needs_tracing(pair.rest) {
        let addr = pair.rest.to_vaddr();
        if addr.as_u64() >= old_heap.as_u64() && addr.as_u64() < old_hend.as_u64() {
            let new_rest = copy_term_for_growth(copier, mem, pair.rest, old_heap, old_hend)?;
            mem.write(rest_addr, new_rest);
        }
    }

    Ok(())
}

/// Scan a boxed object and copy any referenced objects.
fn scan_boxed_for_growth<M: MemorySpace>(
    copier: &mut Copier,
    mem: &mut M,
    header: Header,
    old_heap: Vaddr,
    old_hend: Vaddr,
) -> Result<(), GcError> {
    use crate::term::tag::object;

    let obj_addr = copier.scan_ptr;

    match header.object_tag() {
        object::TUPLE => {
            let arity = header.arity() as usize;
            for i in 0..arity {
                scan_field_for_growth(copier, mem, obj_addr, 8 + i * 8, old_heap, old_hend)?;
            }
        }
        object::VECTOR => {
            let length: u64 = mem.read(Vaddr::new(obj_addr.as_u64() + 8));
            for i in 0..(length as usize) {
                scan_field_for_growth(copier, mem, obj_addr, 16 + i * 8, old_heap, old_hend)?;
            }
        }
        object::MAP => {
            scan_field_for_growth(copier, mem, obj_addr, 8, old_heap, old_hend)?;
        }
        object::CLOSURE => {
            scan_field_for_growth(copier, mem, obj_addr, 8, old_heap, old_hend)?;
            let capture_count = header.arity() as usize;
            for i in 0..capture_count {
                scan_field_for_growth(copier, mem, obj_addr, 16 + i * 8, old_heap, old_hend)?;
            }
        }
        object::FUN => {
            let code_len: u16 = mem.read(Vaddr::new(obj_addr.as_u64() + 12));
            let const_count: u16 = mem.read(Vaddr::new(obj_addr.as_u64() + 14));
            let constants_offset = (16 + code_len as usize + 7) & !7;
            for i in 0..(const_count as usize) {
                scan_field_for_growth(
                    copier,
                    mem,
                    obj_addr,
                    constants_offset + i * 8,
                    old_heap,
                    old_hend,
                )?;
            }
        }
        object::NAMESPACE => {
            scan_field_for_growth(copier, mem, obj_addr, 16, old_heap, old_hend)?;
        }
        object::VAR => {
            scan_field_for_growth(copier, mem, obj_addr, 16, old_heap, old_hend)?;
            scan_field_for_growth(copier, mem, obj_addr, 24, old_heap, old_hend)?;
        }
        _ => {}
    }

    Ok(())
}

/// Scan and update a single field if it points to the old young heap.
fn scan_field_for_growth<M: MemorySpace>(
    copier: &mut Copier,
    mem: &mut M,
    obj_addr: Vaddr,
    offset: usize,
    old_heap: Vaddr,
    old_hend: Vaddr,
) -> Result<(), GcError> {
    let field_addr = Vaddr::new(obj_addr.as_u64() + offset as u64);
    let term: Term = mem.read(field_addr);

    if needs_tracing(term) {
        let addr = term.to_vaddr();
        if addr.as_u64() >= old_heap.as_u64() && addr.as_u64() < old_hend.as_u64() {
            let new_term = copy_term_for_growth(copier, mem, term, old_heap, old_hend)?;
            mem.write(field_addr, new_term);
        }
    }

    Ok(())
}

/// Update stack frame pointers after the stack has been relocated.
///
/// When the stack moves (during heap growth or major GC), all frame base
/// pointers must be updated to point to their new locations:
/// - `process.frame_base` (the innermost frame)
/// - `caller_frame_base` in each stack frame header
///
/// The stack layout preserves relative offsets from `hend`, so we compute
/// new addresses by: `new_addr = new_hend - (old_hend - old_addr)`.
pub fn update_stack_frame_pointers<M: MemorySpace>(
    process: &mut Process,
    mem: &mut M,
    old_hend: Vaddr,
    new_hend: Vaddr,
) {
    // Update frame_base if it exists
    if let Some(old_frame_base) = process.frame_base {
        let offset = old_hend.as_u64() - old_frame_base.as_u64();
        let new_frame_base = Vaddr::new(new_hend.as_u64() - offset);
        process.frame_base = Some(new_frame_base);

        // Walk frames and update caller_frame_base pointers
        let mut frame_addr = new_frame_base;
        loop {
            let caller_frame_ptr =
                Vaddr::new(frame_addr.as_u64() + frame_offset::CALLER_FRAME_BASE as u64);
            let caller_frame: u64 = mem.read(caller_frame_ptr);

            if caller_frame == 0 {
                break; // Top-level frame
            }

            // Update the pointer
            let old_caller_offset = old_hend.as_u64() - caller_frame;
            let new_caller = new_hend.as_u64() - old_caller_offset;
            mem.write(caller_frame_ptr, new_caller);

            frame_addr = Vaddr::new(new_caller);
        }
    }
}
