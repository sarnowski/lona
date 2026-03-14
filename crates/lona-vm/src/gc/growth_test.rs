// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for heap growth.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::gc::growth::{HEAP_SIZES, grow_old_heap, grow_young_heap_with_gc, next_heap_size};
use crate::gc::utils::is_in_young_heap;
use crate::platform::MemorySpace;
use crate::platform::MockVSpace;
use crate::process::pool::ProcessPool;
use crate::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process, WorkerId};
use crate::scheduler::Worker;
use crate::term::Term;
use crate::term::pair::Pair;

/// Create a test process, worker, pool, and mock memory.
fn setup() -> (Process, Worker, ProcessPool, MockVSpace) {
    // Large memory space for testing heap growth
    let mem = MockVSpace::new(16 * 1024 * 1024, Vaddr::new(0)); // 16 MB

    // Pool starts after some reserved space
    let pool_base = Vaddr::new(0x0010_0000);
    let pool = ProcessPool::new(pool_base, 8 * 1024 * 1024); // 8 MB pool

    // Allocate initial process memory from pool
    let young_base = Vaddr::new(0x1000);
    let old_base = Vaddr::new(0x0008_0000);
    let process = Process::new(
        young_base,
        INITIAL_YOUNG_HEAP_SIZE,
        old_base,
        INITIAL_OLD_HEAP_SIZE,
    );

    let worker = Worker::new(WorkerId(0));

    (process, worker, pool, mem)
}

// =============================================================================
// Heap Size Sequence Tests
// =============================================================================

#[test]
fn heap_sizes_are_fibonacci_like() {
    // Verify each size is approximately the sum of the two previous
    // (allowing for rounding in the actual Fibonacci sequence)
    for i in 2..HEAP_SIZES.len() {
        let expected_approx = HEAP_SIZES[i - 1] + HEAP_SIZES[i - 2];
        // Allow 1% tolerance for rounding
        let tolerance = expected_approx / 100;
        assert!(
            HEAP_SIZES[i] >= expected_approx - tolerance
                && HEAP_SIZES[i] <= expected_approx + tolerance,
            "HEAP_SIZES[{i}] = {} is not approximately {} + {} = {}",
            HEAP_SIZES[i],
            HEAP_SIZES[i - 1],
            HEAP_SIZES[i - 2],
            expected_approx
        );
    }
}

#[test]
fn heap_sizes_are_increasing() {
    for i in 1..HEAP_SIZES.len() {
        assert!(
            HEAP_SIZES[i] > HEAP_SIZES[i - 1],
            "HEAP_SIZES not strictly increasing at index {i}"
        );
    }
}

#[test]
fn next_heap_size_returns_larger_size() {
    let current = 500;
    let required = 600;
    let next = next_heap_size(current, required);

    assert!(next >= required, "next_heap_size must be >= required");
    assert!(next > current, "next_heap_size must be > current");
}

#[test]
fn next_heap_size_finds_smallest_fitting_size() {
    // Required 300 words should give us 377 (first size >= 300 that's > current)
    let next = next_heap_size(200, 300);
    assert_eq!(next, 377);
}

#[test]
fn next_heap_size_skips_sizes_not_larger_than_current() {
    // If current is 610, required is 300, we need > 610
    let next = next_heap_size(610, 300);
    assert!(next > 610);
    assert_eq!(next, 987); // Next Fibonacci after 610
}

#[test]
fn next_heap_size_handles_large_required() {
    // Required larger than any standard size
    let required = HEAP_SIZES[HEAP_SIZES.len() - 1] + 1000;
    let next = next_heap_size(0, required);

    // Should return at least the required size
    assert!(next >= required);
}

#[test]
fn next_heap_size_zero_current() {
    let next = next_heap_size(0, 100);
    assert_eq!(next, 233); // First size that fits 100
}

// =============================================================================
// Old Heap Growth Tests
// =============================================================================

#[test]
fn grow_old_heap_increases_capacity() {
    let (mut process, _worker, mut pool, mut mem) = setup();

    let old_capacity_before = process.old_hend.as_u64() - process.old_heap.as_u64();

    // Grow old heap
    let required = old_capacity_before as usize + 1000;
    let result = grow_old_heap(&mut process, &mut pool, &mut mem, required);

    assert!(result.is_ok());

    let old_capacity_after = process.old_hend.as_u64() - process.old_heap.as_u64();
    assert!(old_capacity_after > old_capacity_before);
    assert!(old_capacity_after >= required as u64);
}

#[test]
fn grow_old_heap_preserves_existing_data() {
    let (mut process, _worker, mut pool, mut mem) = setup();

    // Write some data to old heap
    let test_value = 0xDEAD_BEEF_u64;
    mem.write(process.old_heap, test_value);

    // Advance old_htop to mark data as live
    process.old_htop = Vaddr::new(process.old_heap.as_u64() + 8);

    let old_htop_offset = process.old_htop.as_u64() - process.old_heap.as_u64();

    // Grow old heap
    let required = 10000;
    let result = grow_old_heap(&mut process, &mut pool, &mut mem, required);

    assert!(result.is_ok());

    // Verify data was preserved (at same offset from new base)
    let preserved: u64 = mem.read(process.old_heap);
    assert_eq!(preserved, test_value);

    // Verify old_htop offset is preserved
    let new_htop_offset = process.old_htop.as_u64() - process.old_heap.as_u64();
    assert_eq!(new_htop_offset, old_htop_offset);
}

// =============================================================================
// Young Heap Growth Tests
// =============================================================================

#[test]
fn grow_young_heap_increases_capacity() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    let young_capacity_before = process.hend.as_u64() - process.heap.as_u64();

    // Grow young heap
    let required = young_capacity_before as usize + 1000;
    let result = grow_young_heap_with_gc(&mut process, &mut worker, &mut pool, &mut mem, required);

    assert!(result.is_ok());

    let young_capacity_after = process.hend.as_u64() - process.heap.as_u64();
    assert!(young_capacity_after > young_capacity_before);
}

#[test]
fn grow_young_heap_preserves_live_objects() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Allocate a pair and keep reference in X register
    let pair_term = process
        .alloc_term_pair(&mut mem, Term::small_int(42).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[0] = pair_term;

    // Grow young heap
    let required = INITIAL_YOUNG_HEAP_SIZE * 2;
    let result = grow_young_heap_with_gc(&mut process, &mut worker, &mut pool, &mut mem, required);

    assert!(result.is_ok());

    // X register should have updated pointer
    let new_term = worker.x_regs[0];
    assert!(is_in_young_heap(&process, new_term.to_vaddr()));

    // Data should be preserved
    let new_pair: Pair = mem.read(new_term.to_vaddr());
    assert_eq!(new_pair.head, Term::small_int(42).unwrap());
    assert_eq!(new_pair.rest, Term::NIL);
}

#[test]
fn grow_young_heap_preserves_stack() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Simulate a stack frame by writing data at the stack location
    // Stack grows down from hend, so stop < hend
    let frame_size = 32u64;
    process.stop = Vaddr::new(process.hend.as_u64() - frame_size);

    // Write frame data
    let frame_marker = 0xCAFE_BABE_u64;
    mem.write(process.stop, frame_marker);

    let stack_size_before = process.hend.as_u64() - process.stop.as_u64();

    // Grow young heap
    let required = INITIAL_YOUNG_HEAP_SIZE * 2;
    let result = grow_young_heap_with_gc(&mut process, &mut worker, &mut pool, &mut mem, required);

    assert!(result.is_ok());

    // Stack size should be preserved
    let stack_size_after = process.hend.as_u64() - process.stop.as_u64();
    assert_eq!(stack_size_after, stack_size_before);

    // Stack data should be preserved
    let preserved: u64 = mem.read(process.stop);
    assert_eq!(preserved, frame_marker);
}

#[test]
fn grow_young_heap_reclaims_dead_objects() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Allocate objects but don't keep references (they become garbage)
    for _ in 0..10 {
        let _ = process
            .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
            .expect("alloc failed");
    }

    // Only keep one
    let kept = process
        .alloc_term_pair(&mut mem, Term::small_int(42).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[0] = kept;

    let used_before = process.htop.as_u64() - process.heap.as_u64();

    // Grow (which includes GC)
    let required = INITIAL_YOUNG_HEAP_SIZE * 2;
    let result = grow_young_heap_with_gc(&mut process, &mut worker, &mut pool, &mut mem, required);

    assert!(result.is_ok());

    // Only the kept pair should have been copied
    let used_after = process.htop.as_u64() - process.heap.as_u64();
    assert!(
        used_after < used_before,
        "Dead objects should have been reclaimed"
    );
    assert_eq!(used_after, 16); // One pair = 16 bytes
}

#[test]
fn grow_young_heap_updates_transitive_references() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Create chain: pair1 -> pair2 -> pair3
    let pair3 = process
        .alloc_term_pair(&mut mem, Term::small_int(3).unwrap(), Term::NIL)
        .expect("alloc failed");
    let pair2 = process
        .alloc_term_pair(&mut mem, pair3, Term::NIL)
        .expect("alloc failed");
    let pair1 = process
        .alloc_term_pair(&mut mem, pair2, Term::NIL)
        .expect("alloc failed");

    worker.x_regs[0] = pair1;

    // Grow
    let required = INITIAL_YOUNG_HEAP_SIZE * 2;
    let result = grow_young_heap_with_gc(&mut process, &mut worker, &mut pool, &mut mem, required);

    assert!(result.is_ok());

    // Walk the chain and verify all pointers are in new young heap
    let new_pair1 = worker.x_regs[0];
    assert!(is_in_young_heap(&process, new_pair1.to_vaddr()));

    let p1: Pair = mem.read(new_pair1.to_vaddr());
    assert!(is_in_young_heap(&process, p1.head.to_vaddr()));

    let p2: Pair = mem.read(p1.head.to_vaddr());
    assert!(is_in_young_heap(&process, p2.head.to_vaddr()));

    let p3: Pair = mem.read(p2.head.to_vaddr());
    assert_eq!(p3.head, Term::small_int(3).unwrap());
}

// =============================================================================
// Pool Allocation Failure Tests
// =============================================================================

#[test]
fn grow_heap_fails_when_pool_exhausted() {
    let (mut process, _worker, mut pool, mut mem) = setup();

    // Exhaust the pool by allocating most of it
    while pool.remaining() > 1024 {
        let _ = pool.allocate(1024, 8);
    }

    // Try to grow - should fail
    let required = 1024 * 1024; // 1 MB
    let result = grow_old_heap(&mut process, &mut pool, &mut mem, required);

    assert!(result.is_err());
}
