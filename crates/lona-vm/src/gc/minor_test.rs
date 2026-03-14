// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for minor garbage collection.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::gc::GcError;
use crate::gc::minor::minor_gc;
use crate::gc::utils::{is_in_old_heap, is_in_young_heap};
use crate::platform::MemorySpace;
use crate::platform::MockVSpace;
use crate::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process, WorkerId};
use crate::scheduler::Worker;
use crate::term::Term;
use crate::term::pair::Pair;

/// Create a test process, worker, and mock memory.
fn setup() -> (Process, Worker, MockVSpace) {
    let young_base = Vaddr::new(0x1000);
    let old_base = Vaddr::new(0x0010_0000);
    let process = Process::new(
        young_base,
        INITIAL_YOUNG_HEAP_SIZE,
        old_base,
        INITIAL_OLD_HEAP_SIZE,
    );
    let worker = Worker::new(WorkerId(0));
    let mem = MockVSpace::new(2 * 1024 * 1024, Vaddr::new(0)); // 2 MB mock memory
    (process, worker, mem)
}

// =============================================================================
// Basic Minor GC Tests
// =============================================================================

#[test]
fn minor_gc_empty_heap() {
    let (mut process, mut worker, mut mem) = setup();

    // GC with empty young heap should succeed
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());
    let stats = result.unwrap();
    assert_eq!(stats.live_bytes, 0);
}

#[test]
fn minor_gc_no_roots_reclaims_all() {
    let (mut process, mut worker, mut mem) = setup();

    // Allocate some objects but don't keep references to them
    let _ = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");
    let _ = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(42).unwrap()])
        .expect("alloc failed");

    // GC should reclaim everything since there are no roots
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());
    let stats = result.unwrap();
    // Nothing should have been promoted
    assert_eq!(stats.live_bytes, 0);
    // Young heap should be reset
    assert_eq!(process.htop, process.heap);
}

#[test]
fn minor_gc_x_register_root_promotes() {
    let (mut process, mut worker, mut mem) = setup();

    // Allocate a pair and keep reference in X register
    let pair_term = process
        .alloc_term_pair(&mut mem, Term::small_int(99).unwrap(), Term::NIL)
        .expect("alloc failed");

    worker.x_regs[0] = pair_term;

    // Verify it's in young heap before GC
    assert!(is_in_young_heap(&process, pair_term.to_vaddr()));

    // Run minor GC
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());
    let stats = result.unwrap();
    assert!(stats.live_bytes > 0); // Something was promoted

    // X register should now point to old heap
    let new_term = worker.x_regs[0];
    assert!(is_in_old_heap(&process, new_term.to_vaddr()));

    // Verify data is preserved
    let new_pair: Pair = mem.read(new_term.to_vaddr());
    assert_eq!(new_pair.head, Term::small_int(99).unwrap());
    assert_eq!(new_pair.rest, Term::NIL);
}

#[test]
fn minor_gc_binding_root_promotes() {
    let (mut process, mut worker, mut mem) = setup();

    // Allocate a tuple
    let tuple_term = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(42).unwrap()])
        .expect("alloc failed");

    // Keep reference in process bindings
    let var_addr = Vaddr::new(0x9000); // Some fake var address
    process.bindings.insert(var_addr, tuple_term);

    // Run minor GC
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());

    // Binding should now point to old heap
    let new_term = *process.bindings.get(&var_addr).unwrap();
    assert!(is_in_old_heap(&process, new_term.to_vaddr()));
}

#[test]
fn minor_gc_transitive_promotion() {
    let (mut process, mut worker, mut mem) = setup();

    // Create a chain: pair1 -> pair2 -> pair3
    let pair3 = process
        .alloc_term_pair(&mut mem, Term::small_int(3).unwrap(), Term::NIL)
        .expect("alloc failed");
    let pair2 = process
        .alloc_term_pair(&mut mem, pair3, Term::NIL)
        .expect("alloc failed");
    let pair1 = process
        .alloc_term_pair(&mut mem, pair2, Term::NIL)
        .expect("alloc failed");

    // Root only points to pair1
    worker.x_regs[0] = pair1;

    // Run minor GC
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());

    // All three pairs should now be in old heap
    let new_pair1 = worker.x_regs[0];
    assert!(is_in_old_heap(&process, new_pair1.to_vaddr()));

    let p1: Pair = mem.read(new_pair1.to_vaddr());
    assert!(is_in_old_heap(&process, p1.head.to_vaddr()));

    let p2: Pair = mem.read(p1.head.to_vaddr());
    assert!(is_in_old_heap(&process, p2.head.to_vaddr()));

    let p3: Pair = mem.read(p2.head.to_vaddr());
    assert_eq!(p3.head, Term::small_int(3).unwrap());
}

#[test]
fn minor_gc_shared_object_not_duplicated() {
    let (mut process, mut worker, mut mem) = setup();

    // Create shared: both pair1 and pair2 point to shared
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

    // Run minor GC
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());

    // Get new addresses
    let new_pair1: Pair = mem.read(worker.x_regs[0].to_vaddr());
    let new_pair2: Pair = mem.read(worker.x_regs[1].to_vaddr());

    // Both should point to the SAME new shared object
    assert_eq!(new_pair1.head, new_pair2.head);
}

#[test]
fn minor_gc_cyclic_structure() {
    let (mut process, mut worker, mut mem) = setup();

    // Create a self-referencing pair
    let pair_addr = process.alloc(16, 8).expect("alloc failed");
    let pair_term = Term::list_vaddr(pair_addr);

    // Write pair with rest pointing to itself
    let pair = Pair::new(Term::small_int(1).unwrap(), pair_term);
    mem.write(pair_addr, pair);

    worker.x_regs[0] = pair_term;

    // Run minor GC - should not infinite loop
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());

    // New pair should still be cyclic
    let new_pair_term = worker.x_regs[0];
    let new_pair: Pair = mem.read(new_pair_term.to_vaddr());
    assert_eq!(new_pair.rest, new_pair_term);
}

#[test]
fn minor_gc_resets_young_heap() {
    let (mut process, mut worker, mut mem) = setup();

    // Allocate some objects
    let _ = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");
    let _ = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(42).unwrap()])
        .expect("alloc failed");

    // htop should have advanced
    assert!(process.htop.as_u64() > process.heap.as_u64());

    // Run minor GC
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());

    // htop should be reset to heap base
    assert_eq!(process.htop, process.heap);
}

#[test]
fn minor_gc_updates_statistics() {
    let (mut process, mut worker, mut mem) = setup();

    let initial_count = process.minor_gc_count;

    // Allocate and root an object
    let pair_term = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");
    worker.x_regs[0] = pair_term;

    // Run minor GC
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());
    assert_eq!(process.minor_gc_count, initial_count + 1);
}

// =============================================================================
// Old Heap Overflow Tests
// =============================================================================

#[test]
fn minor_gc_old_heap_overflow_returns_error() {
    // Create a process with a very small old heap
    let young_base = Vaddr::new(0x1000);
    let old_base = Vaddr::new(0x0010_0000);
    let mut process = Process::new(
        young_base,
        INITIAL_YOUNG_HEAP_SIZE,
        old_base,
        64, // Tiny old heap: only 64 bytes
    );
    let mut worker = Worker::new(WorkerId(0));
    let mut mem = MockVSpace::new(2 * 1024 * 1024, Vaddr::new(0));

    // Allocate objects that exceed old heap capacity
    // Each tuple: 8 (header) + 8 * N (elements) bytes
    let elements = [Term::small_int(1).unwrap(); 8]; // 72 bytes total
    let tuple = process
        .alloc_term_tuple(&mut mem, &elements)
        .expect("alloc failed");
    worker.x_regs[0] = tuple;

    // Minor GC should fail with NeedsMajorGc
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), GcError::NeedsMajorGc);
}

// =============================================================================
// Old Heap Object Not Copied Tests
// =============================================================================

#[test]
fn minor_gc_old_heap_objects_not_copied() {
    let (mut process, mut worker, mut mem) = setup();

    // Manually place an object in old heap (simulate promotion)
    let old_addr = process.old_htop;
    let header = crate::term::header::Header::tuple(1);
    mem.write(old_addr, header);
    mem.write(
        Vaddr::new(old_addr.as_u64() + 8),
        Term::small_int(42).unwrap(),
    );
    process.old_htop = Vaddr::new(old_addr.as_u64() + 16);

    // Create a Term pointing to the old heap object
    let old_term = Term::boxed_vaddr(old_addr);

    // Put old heap reference in X register
    worker.x_regs[0] = old_term;

    let old_htop_before = process.old_htop;

    // Run minor GC
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());

    // Old heap top should not have changed (nothing promoted)
    assert_eq!(process.old_htop, old_htop_before);

    // X register should still point to same old heap address
    assert_eq!(worker.x_regs[0], old_term);
}

// =============================================================================
// Immediate Terms Not Affected Tests
// =============================================================================

#[test]
fn minor_gc_immediates_unchanged() {
    let (mut process, mut worker, mut mem) = setup();

    // Fill X registers with various immediates
    worker.x_regs[0] = Term::NIL;
    worker.x_regs[1] = Term::TRUE;
    worker.x_regs[2] = Term::FALSE;
    worker.x_regs[3] = Term::small_int(12345).unwrap();
    worker.x_regs[4] = Term::symbol(100);
    worker.x_regs[5] = Term::keyword(200);

    // Run minor GC
    let result = minor_gc(&mut process, &mut worker, &mut mem);

    assert!(result.is_ok());

    // All immediates should be unchanged
    assert_eq!(worker.x_regs[0], Term::NIL);
    assert_eq!(worker.x_regs[1], Term::TRUE);
    assert_eq!(worker.x_regs[2], Term::FALSE);
    assert_eq!(worker.x_regs[3], Term::small_int(12345).unwrap());
    assert_eq!(worker.x_regs[4], Term::symbol(100));
    assert_eq!(worker.x_regs[5], Term::keyword(200));
}
