// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for major garbage collection.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::gc::major::major_gc;
use crate::gc::minor::minor_gc;
use crate::gc::utils::{is_in_old_heap, is_in_young_heap};
use crate::platform::MemorySpace;
use crate::platform::MockVSpace;
use crate::process::pool::ProcessPool;
use crate::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process, WorkerId};
use crate::scheduler::Worker;
use crate::term::Term;
use crate::term::pair::Pair;

/// Create a test process, worker, pool, and mock memory.
fn setup() -> (Process, Worker, ProcessPool, MockVSpace) {
    // Large memory space for testing
    let mem = MockVSpace::new(16 * 1024 * 1024, Vaddr::new(0)); // 16 MB

    // Pool starts after some reserved space
    let pool_base = Vaddr::new(0x0010_0000);
    let pool = ProcessPool::new(pool_base, 8 * 1024 * 1024); // 8 MB pool

    // Allocate initial process memory
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
// Basic Major GC Tests
// =============================================================================

#[test]
fn major_gc_empty_heaps() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Major GC with empty heaps should succeed
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());
    let stats = result.unwrap();
    assert_eq!(stats.live_bytes, 0);
}

#[test]
fn major_gc_reclaims_dead_young_objects() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Allocate objects but don't keep references
    for _ in 0..10 {
        let _ = process
            .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
            .expect("alloc failed");
    }

    let used_before = process.htop.as_u64() - process.heap.as_u64();
    assert!(used_before > 0);

    // Major GC should reclaim all
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Young heap should be reset and empty
    let used_after = process.htop.as_u64() - process.heap.as_u64();
    assert_eq!(used_after, 0);
}

#[test]
fn major_gc_reclaims_dead_old_objects() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Allocate objects and promote them to old heap via minor GC
    let pair = process
        .alloc_term_pair(&mut mem, Term::small_int(42).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[0] = pair;

    // Minor GC promotes to old heap
    let _ = minor_gc(&mut process, &mut worker, &mut mem).expect("minor gc failed");

    // Object is now in old heap
    assert!(is_in_old_heap(&process, worker.x_regs[0].to_vaddr()));

    // Clear the root - object is now dead
    worker.x_regs[0] = Term::NIL;

    let old_used_before = process.old_htop.as_u64() - process.old_heap.as_u64();
    assert!(old_used_before > 0);

    // Major GC should reclaim it
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Old heap should be empty after major GC
    // (all live data compacts into young heap)
    let old_used_after = process.old_htop.as_u64() - process.old_heap.as_u64();
    assert_eq!(old_used_after, 0);
}

#[test]
fn major_gc_preserves_live_young_objects() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Allocate and keep reference
    let pair = process
        .alloc_term_pair(&mut mem, Term::small_int(99).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[0] = pair;

    // Major GC
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Object should still be accessible in young heap (compacted)
    let new_term = worker.x_regs[0];
    assert!(is_in_young_heap(&process, new_term.to_vaddr()));

    // Data preserved
    let new_pair: Pair = mem.read(new_term.to_vaddr());
    assert_eq!(new_pair.head, Term::small_int(99).unwrap());
    assert_eq!(new_pair.rest, Term::NIL);
}

#[test]
fn major_gc_preserves_live_old_objects() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Create object and promote to old heap
    let pair = process
        .alloc_term_pair(&mut mem, Term::small_int(42).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[0] = pair;

    // Promote via minor GC
    let _ = minor_gc(&mut process, &mut worker, &mut mem).expect("minor gc failed");

    // Verify in old heap
    assert!(is_in_old_heap(&process, worker.x_regs[0].to_vaddr()));

    // Major GC compacts everything into new young heap
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Object should now be in young heap (compacted from old)
    let new_term = worker.x_regs[0];
    assert!(is_in_young_heap(&process, new_term.to_vaddr()));

    // Data preserved
    let new_pair: Pair = mem.read(new_term.to_vaddr());
    assert_eq!(new_pair.head, Term::small_int(42).unwrap());
}

#[test]
fn major_gc_compacts_all_live_data_to_young_heap() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Create multiple objects and scatter them between young and old heaps
    let pair1 = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[0] = pair1;

    // Promote first pair to old heap
    let _ = minor_gc(&mut process, &mut worker, &mut mem).expect("minor gc failed");

    // Create more objects in young heap
    let pair2 = process
        .alloc_term_pair(&mut mem, Term::small_int(2).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[1] = pair2;

    // Verify setup: one in old, one in young
    assert!(is_in_old_heap(&process, worker.x_regs[0].to_vaddr()));
    assert!(is_in_young_heap(&process, worker.x_regs[1].to_vaddr()));

    // Major GC compacts all
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Both should now be in young heap
    assert!(is_in_young_heap(&process, worker.x_regs[0].to_vaddr()));
    assert!(is_in_young_heap(&process, worker.x_regs[1].to_vaddr()));

    // Old heap should be empty
    assert_eq!(process.old_htop, process.old_heap);
}

#[test]
fn major_gc_handles_transitive_references() {
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

    // Promote all to old heap
    let _ = minor_gc(&mut process, &mut worker, &mut mem).expect("minor gc failed");

    // Major GC
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Walk the chain and verify all in young heap with correct data
    let new_pair1 = worker.x_regs[0];
    assert!(is_in_young_heap(&process, new_pair1.to_vaddr()));

    let p1: Pair = mem.read(new_pair1.to_vaddr());
    assert!(is_in_young_heap(&process, p1.head.to_vaddr()));

    let p2: Pair = mem.read(p1.head.to_vaddr());
    assert!(is_in_young_heap(&process, p2.head.to_vaddr()));

    let p3: Pair = mem.read(p2.head.to_vaddr());
    assert_eq!(p3.head, Term::small_int(3).unwrap());
}

#[test]
fn major_gc_handles_shared_objects() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Create shared object referenced by two parents
    let shared = process
        .alloc_term_pair(&mut mem, Term::small_int(42).unwrap(), Term::NIL)
        .expect("alloc failed");
    let pair1 = process
        .alloc_term_pair(&mut mem, shared, Term::NIL)
        .expect("alloc failed");
    let pair2 = process
        .alloc_term_pair(&mut mem, shared, Term::NIL)
        .expect("alloc failed");

    worker.x_regs[0] = pair1;
    worker.x_regs[1] = pair2;

    // Major GC
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Both parents should point to the same shared object
    let new_pair1: Pair = mem.read(worker.x_regs[0].to_vaddr());
    let new_pair2: Pair = mem.read(worker.x_regs[1].to_vaddr());
    assert_eq!(new_pair1.head, new_pair2.head);
}

#[test]
fn major_gc_handles_cyclic_structures() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Create self-referencing pair
    let pair_addr = process.alloc(16, 8).expect("alloc failed");
    let pair_term = Term::list_vaddr(pair_addr);

    // Write pair with rest pointing to itself
    let pair = Pair::new(Term::small_int(1).unwrap(), pair_term);
    mem.write(pair_addr, pair);

    worker.x_regs[0] = pair_term;

    // Major GC should not infinite loop
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // New pair should still be cyclic
    let new_pair_term = worker.x_regs[0];
    let new_pair: Pair = mem.read(new_pair_term.to_vaddr());
    assert_eq!(new_pair.rest, new_pair_term);
}

// =============================================================================
// Stack Preservation Tests
// =============================================================================

#[test]
fn major_gc_preserves_stack() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Create a stack frame
    let frame_size = 32u64;
    process.stop = Vaddr::new(process.hend.as_u64() - frame_size);

    // Write frame data
    let frame_marker = 0xCAFE_BABE_u64;
    mem.write(process.stop, frame_marker);

    let stack_size_before = process.hend.as_u64() - process.stop.as_u64();

    // Major GC
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Stack size should be preserved
    let stack_size_after = process.hend.as_u64() - process.stop.as_u64();
    assert_eq!(stack_size_after, stack_size_before);

    // Stack data should be preserved
    let preserved: u64 = mem.read(process.stop);
    assert_eq!(preserved, frame_marker);
}

// =============================================================================
// Statistics Tests
// =============================================================================

#[test]
fn major_gc_updates_statistics() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    let initial_count = process.major_gc_count;

    // Major GC
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());
    assert_eq!(process.major_gc_count, initial_count + 1);
}

// =============================================================================
// Old Heap Reset Tests
// =============================================================================

#[test]
fn major_gc_resets_old_heap() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Create object and promote to old heap
    let pair = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[0] = pair;

    // Minor GC promotes
    let _ = minor_gc(&mut process, &mut worker, &mut mem).expect("minor gc failed");

    // Old heap should have data
    assert!(process.old_htop.as_u64() > process.old_heap.as_u64());

    // Major GC
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Old heap should be empty (all data compacted to young heap)
    assert_eq!(process.old_htop, process.old_heap);
}

// =============================================================================
// Process Binding Tests
// =============================================================================

#[test]
fn major_gc_preserves_process_bindings() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Allocate and bind
    let tuple = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(42).unwrap()])
        .expect("alloc failed");

    let var_addr = Vaddr::new(0x9000);
    process.bindings.insert(var_addr, tuple);

    // Promote to old heap
    let _ = minor_gc(&mut process, &mut worker, &mut mem).expect("minor gc failed");

    // Major GC
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Binding should be updated and in young heap
    let new_term = *process.bindings.get(&var_addr).unwrap();
    assert!(is_in_young_heap(&process, new_term.to_vaddr()));
}

// =============================================================================
// Immediate Terms Tests
// =============================================================================

#[test]
fn major_gc_immediates_unchanged() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Fill X registers with immediates
    worker.x_regs[0] = Term::NIL;
    worker.x_regs[1] = Term::TRUE;
    worker.x_regs[2] = Term::FALSE;
    worker.x_regs[3] = Term::small_int(12345).unwrap();
    worker.x_regs[4] = Term::symbol(100);

    // Major GC
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // All should be unchanged
    assert_eq!(worker.x_regs[0], Term::NIL);
    assert_eq!(worker.x_regs[1], Term::TRUE);
    assert_eq!(worker.x_regs[2], Term::FALSE);
    assert_eq!(worker.x_regs[3], Term::small_int(12345).unwrap());
    assert_eq!(worker.x_regs[4], Term::symbol(100));
}

// =============================================================================
// Mixed Young/Old Heap Tests
// =============================================================================

#[test]
fn major_gc_handles_cross_generation_references() {
    let (mut process, mut worker, mut pool, mut mem) = setup();

    // Create an object in young heap
    let young_pair = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[0] = young_pair;

    // Promote to old heap
    let _ = minor_gc(&mut process, &mut worker, &mut mem).expect("minor gc failed");
    let old_pair = worker.x_regs[0];
    assert!(is_in_old_heap(&process, old_pair.to_vaddr()));

    // Create new young object that references the old object
    let new_young_pair = process
        .alloc_term_pair(&mut mem, old_pair, Term::NIL)
        .expect("alloc failed");
    worker.x_regs[1] = new_young_pair;

    // Major GC should handle cross-generation reference
    let result = major_gc(&mut process, &mut worker, &mut pool, &mut mem);

    assert!(result.is_ok());

    // Both should be in young heap now
    assert!(is_in_young_heap(&process, worker.x_regs[0].to_vaddr()));
    assert!(is_in_young_heap(&process, worker.x_regs[1].to_vaddr()));

    // Reference should be preserved
    let new_parent: Pair = mem.read(worker.x_regs[1].to_vaddr());
    assert_eq!(new_parent.head, worker.x_regs[0]);
}
