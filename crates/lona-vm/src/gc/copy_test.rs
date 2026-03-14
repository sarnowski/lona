// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for GC object copying (Cheney's algorithm).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::gc::copy::{Copier, copy_term, scan_object_fields};
use crate::gc::utils::{is_in_old_heap, is_in_young_heap, needs_tracing};
use crate::platform::MemorySpace;
use crate::platform::MockVSpace;
use crate::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process};
use crate::term::Term;
use crate::term::header::Header;
use crate::term::pair::Pair;

/// Create a test process and memory space.
fn setup() -> (Process, MockVSpace) {
    let young_base = Vaddr::new(0x1000);
    let old_base = Vaddr::new(0x0010_0000);
    let process = Process::new(
        young_base,
        INITIAL_YOUNG_HEAP_SIZE,
        old_base,
        INITIAL_OLD_HEAP_SIZE,
    );
    let mem = MockVSpace::new(2 * 1024 * 1024, Vaddr::new(0)); // 2 MB mock memory
    (process, mem)
}

// =============================================================================
// Copier Creation Tests
// =============================================================================

#[test]
fn copier_new_initializes_pointers() {
    let (_process, _mem) = setup();
    let to_start = Vaddr::new(0x0020_0000);
    let to_end = Vaddr::new(0x0030_0000);

    let copier = Copier::new(to_start, to_end);

    assert_eq!(copier.to_space_start, to_start);
    assert_eq!(copier.to_space_end, to_end);
    assert_eq!(copier.alloc_ptr, to_start);
    assert_eq!(copier.scan_ptr, to_start);
}

#[test]
fn copier_bytes_copied_initially_zero() {
    let to_start = Vaddr::new(0x0020_0000);
    let to_end = Vaddr::new(0x0030_0000);

    let copier = Copier::new(to_start, to_end);

    assert_eq!(copier.bytes_copied(), 0);
}

// =============================================================================
// Copy Term Tests - Immediates
// =============================================================================

#[test]
fn copy_term_immediate_unchanged() {
    let (process, mut mem) = setup();
    let to_start = Vaddr::new(0x0020_0000);
    let to_end = Vaddr::new(0x0030_0000);
    let mut copier = Copier::new(to_start, to_end);

    // Immediates don't need copying - should return unchanged
    let int_term = Term::small_int(42).unwrap();
    let result = copy_term(&mut copier, &process, &mut mem, int_term);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), int_term);
    assert_eq!(copier.bytes_copied(), 0); // Nothing copied
}

#[test]
fn copy_term_nil_unchanged() {
    let (process, mut mem) = setup();
    let to_start = Vaddr::new(0x0020_0000);
    let to_end = Vaddr::new(0x0030_0000);
    let mut copier = Copier::new(to_start, to_end);

    let result = copy_term(&mut copier, &process, &mut mem, Term::NIL);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Term::NIL);
}

#[test]
fn copy_term_symbol_unchanged() {
    let (process, mut mem) = setup();
    let to_start = Vaddr::new(0x0020_0000);
    let to_end = Vaddr::new(0x0030_0000);
    let mut copier = Copier::new(to_start, to_end);

    let sym_term = Term::symbol(123);
    let result = copy_term(&mut copier, &process, &mut mem, sym_term);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), sym_term);
}

// =============================================================================
// Copy Term Tests - Pairs
// =============================================================================

#[test]
fn copy_term_pair_to_new_location() {
    let (mut process, mut mem) = setup();

    // Allocate a pair on the young heap
    let pair_term = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");

    assert!(is_in_young_heap(&process, pair_term.to_vaddr()));

    // Copy to old heap
    let mut copier = Copier::new(process.old_heap, process.old_hend);

    let result = copy_term(&mut copier, &process, &mut mem, pair_term);

    assert!(result.is_ok());
    let new_term = result.unwrap();

    // New term should point to old heap
    assert!(is_in_old_heap(&process, new_term.to_vaddr()));

    // New term should have same structure
    assert!(new_term.is_list());

    // Read pair from new location
    let new_pair: Pair = mem.read(new_term.to_vaddr());
    assert_eq!(new_pair.head, Term::small_int(1).unwrap());
    assert_eq!(new_pair.rest, Term::NIL);
}

#[test]
fn copy_term_pair_leaves_forwarding_pointer() {
    let (mut process, mut mem) = setup();

    // Allocate a pair on the young heap
    let pair_term = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");

    let original_addr = pair_term.to_vaddr();

    // Copy to old heap
    let mut copier = Copier::new(process.old_heap, process.old_hend);
    let new_term = copy_term(&mut copier, &process, &mut mem, pair_term).unwrap();

    // Original location should now have forwarding pointer
    let forwarded_pair: Pair = mem.read(original_addr);
    assert!(forwarded_pair.is_forwarded());
    assert_eq!(
        forwarded_pair.forward_address() as u64,
        new_term.to_vaddr().as_u64()
    );
}

#[test]
fn copy_term_pair_already_forwarded_returns_forwarded_address() {
    let (mut process, mut mem) = setup();

    // Allocate a pair on the young heap
    let pair_term = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");

    // Copy once
    let mut copier = Copier::new(process.old_heap, process.old_hend);
    let first_copy = copy_term(&mut copier, &process, &mut mem, pair_term).unwrap();

    let bytes_after_first = copier.bytes_copied();

    // Copy again - should return same address without additional copying
    let second_copy = copy_term(&mut copier, &process, &mut mem, pair_term).unwrap();

    assert_eq!(first_copy, second_copy);
    assert_eq!(copier.bytes_copied(), bytes_after_first); // No additional bytes
}

// =============================================================================
// Copy Term Tests - Boxed Objects (Tuple)
// =============================================================================

#[test]
fn copy_term_tuple_to_new_location() {
    let (mut process, mut mem) = setup();

    // Allocate a tuple on the young heap
    let elements = [
        Term::small_int(1).unwrap(),
        Term::small_int(2).unwrap(),
        Term::small_int(3).unwrap(),
    ];
    let tuple_term = process
        .alloc_term_tuple(&mut mem, &elements)
        .expect("alloc failed");

    assert!(is_in_young_heap(&process, tuple_term.to_vaddr()));

    // Copy to old heap
    let mut copier = Copier::new(process.old_heap, process.old_hend);
    let new_term = copy_term(&mut copier, &process, &mut mem, tuple_term).unwrap();

    // New term should point to old heap
    assert!(is_in_old_heap(&process, new_term.to_vaddr()));

    // Read header and verify
    let header: Header = mem.read(new_term.to_vaddr());
    assert_eq!(header.arity(), 3);

    // Read elements and verify
    for (i, &expected) in elements.iter().enumerate() {
        let elem_addr = Vaddr::new(new_term.to_vaddr().as_u64() + 8 + (i as u64) * 8);
        let elem: Term = mem.read(elem_addr);
        assert_eq!(elem, expected);
    }
}

#[test]
fn copy_term_boxed_leaves_forwarding_header() {
    let (mut process, mut mem) = setup();

    // Allocate a tuple on the young heap
    let elements = [Term::small_int(42).unwrap()];
    let tuple_term = process
        .alloc_term_tuple(&mut mem, &elements)
        .expect("alloc failed");

    let original_addr = tuple_term.to_vaddr();

    // Copy to old heap
    let mut copier = Copier::new(process.old_heap, process.old_hend);
    let new_term = copy_term(&mut copier, &process, &mut mem, tuple_term).unwrap();

    // Original location should now have forwarding header
    let header: Header = mem.read(original_addr);
    assert!(header.is_forward());
    assert_eq!(
        header.forward_address() as u64,
        new_term.to_vaddr().as_u64()
    );
}

// =============================================================================
// Copy Term Tests - Out of Space
// =============================================================================

#[test]
fn copy_term_out_of_space_returns_error() {
    let (mut process, mut mem) = setup();

    // Allocate a tuple on the young heap
    let elements = [Term::small_int(1).unwrap(); 100]; // Large tuple
    let tuple_term = process
        .alloc_term_tuple(&mut mem, &elements)
        .expect("alloc failed");

    // Create copier with tiny to-space (8 bytes, not enough for the tuple)
    let to_start = Vaddr::new(0x0020_0000);
    let to_end = Vaddr::new(0x0020_0008); // Only 8 bytes
    let mut copier = Copier::new(to_start, to_end);

    let result = copy_term(&mut copier, &process, &mut mem, tuple_term);

    assert!(result.is_err());
}

// =============================================================================
// Scan Object Fields Tests
// =============================================================================

#[test]
fn scan_object_fields_pair_yields_head_and_rest() {
    let (mut process, mut mem) = setup();

    // Allocate a pair with heap pointer children
    let child_pair = process
        .alloc_term_pair(&mut mem, Term::small_int(99).unwrap(), Term::NIL)
        .expect("alloc failed");

    let parent_pair = process
        .alloc_term_pair(&mut mem, child_pair, Term::small_int(1).unwrap())
        .expect("alloc failed");

    // Collect fields that need tracing
    let mut fields = vec![];
    scan_object_fields(&mem, parent_pair, |field_addr, term| {
        if needs_tracing(term) {
            fields.push((field_addr, term));
        }
    });

    // Should find head (child_pair) but not rest (small_int)
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].1, child_pair);
}

#[test]
fn scan_object_fields_tuple_yields_all_elements() {
    let (mut process, mut mem) = setup();

    // Allocate children
    let child1 = process
        .alloc_term_pair(&mut mem, Term::NIL, Term::NIL)
        .expect("alloc failed");
    let child2 = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(1).unwrap()])
        .expect("alloc failed");

    // Allocate parent tuple with heap pointers and immediates
    let tuple = process
        .alloc_term_tuple(&mut mem, &[child1, Term::small_int(42).unwrap(), child2])
        .expect("alloc failed");

    // Collect fields
    let mut fields = vec![];
    scan_object_fields(&mem, tuple, |field_addr, term| {
        if needs_tracing(term) {
            fields.push((field_addr, term));
        }
    });

    // Should find child1 and child2, but not the small_int
    assert_eq!(fields.len(), 2);
    let terms: Vec<_> = fields.iter().map(|(_, t)| *t).collect();
    assert!(terms.contains(&child1));
    assert!(terms.contains(&child2));
}

#[test]
fn scan_object_fields_map_yields_entries() {
    let (mut process, mut mem) = setup();

    // Allocate a map with an entries list
    let entries = process
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .expect("alloc failed");

    let map_term = process
        .alloc_term_map(&mut mem, entries, 1)
        .expect("alloc failed");

    // Collect fields
    let mut fields = vec![];
    scan_object_fields(&mem, map_term, |field_addr, term| {
        if needs_tracing(term) {
            fields.push((field_addr, term));
        }
    });

    // Should find the entries list
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].1, entries);
}

#[test]
fn scan_object_fields_closure_yields_function_and_captures() {
    let (mut process, mut mem) = setup();

    // Allocate a function (minimal for testing)
    let func = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(1).unwrap()]) // Fake function
        .expect("alloc failed");

    // Allocate a capture
    let capture = process
        .alloc_term_pair(&mut mem, Term::NIL, Term::NIL)
        .expect("alloc failed");

    // Allocate closure
    let closure = process
        .alloc_term_closure(&mut mem, func, &[capture])
        .expect("alloc failed");

    // Collect fields
    let mut fields = vec![];
    scan_object_fields(&mem, closure, |field_addr, term| {
        if needs_tracing(term) {
            fields.push((field_addr, term));
        }
    });

    // Should find function and capture
    assert_eq!(fields.len(), 2);
    let terms: Vec<_> = fields.iter().map(|(_, t)| *t).collect();
    assert!(terms.contains(&func));
    assert!(terms.contains(&capture));
}

// =============================================================================
// Cheney's Scan Loop Tests
// =============================================================================

#[test]
fn cheney_scan_copies_referenced_objects() {
    let (mut process, mut mem) = setup();

    // Create a graph: parent pair -> child pair -> grandchild pair
    let grandchild = process
        .alloc_term_pair(&mut mem, Term::small_int(3).unwrap(), Term::NIL)
        .expect("alloc failed");
    let child = process
        .alloc_term_pair(&mut mem, grandchild, Term::NIL)
        .expect("alloc failed");
    let parent = process
        .alloc_term_pair(&mut mem, child, Term::NIL)
        .expect("alloc failed");

    // Copy parent only first
    let mut copier = Copier::new(process.old_heap, process.old_hend);
    let new_parent = copy_term(&mut copier, &process, &mut mem, parent).unwrap();

    // After copying parent, scan_ptr < alloc_ptr
    // scan_copied_objects should copy child and grandchild
    copier.scan_copied_objects(&process, &mut mem).unwrap();

    // All objects should now be forwarded
    let parent_pair: Pair = mem.read(parent.to_vaddr());
    assert!(parent_pair.is_forwarded());

    let child_pair: Pair = mem.read(child.to_vaddr());
    assert!(child_pair.is_forwarded());

    let gc_pair: Pair = mem.read(grandchild.to_vaddr());
    assert!(gc_pair.is_forwarded());

    // Verify structure is preserved by reading from new location
    let new_parent_pair: Pair = mem.read(new_parent.to_vaddr());
    assert!(new_parent_pair.head.is_list()); // Points to new child

    let new_child_addr = new_parent_pair.head.to_vaddr();
    let new_child_pair: Pair = mem.read(new_child_addr);
    assert!(new_child_pair.head.is_list()); // Points to new grandchild

    let new_gc_addr = new_child_pair.head.to_vaddr();
    let new_gc_pair: Pair = mem.read(new_gc_addr);
    assert_eq!(new_gc_pair.head, Term::small_int(3).unwrap());
}

#[test]
fn cheney_scan_handles_shared_references() {
    let (mut process, mut mem) = setup();

    // Create shared structure: both parent1 and parent2 reference the same child
    let shared_child = process
        .alloc_term_pair(&mut mem, Term::small_int(42).unwrap(), Term::NIL)
        .expect("alloc failed");
    let parent1 = process
        .alloc_term_pair(&mut mem, shared_child, Term::NIL)
        .expect("alloc failed");
    let parent2 = process
        .alloc_term_pair(&mut mem, shared_child, Term::NIL)
        .expect("alloc failed");

    // Copy both parents
    let mut copier = Copier::new(process.old_heap, process.old_hend);
    let new_parent1 = copy_term(&mut copier, &process, &mut mem, parent1).unwrap();
    let new_parent2 = copy_term(&mut copier, &process, &mut mem, parent2).unwrap();

    // Scan
    copier.scan_copied_objects(&process, &mut mem).unwrap();

    // Both new parents should point to the SAME new child
    let new_p1: Pair = mem.read(new_parent1.to_vaddr());
    let new_p2: Pair = mem.read(new_parent2.to_vaddr());

    assert_eq!(new_p1.head, new_p2.head);
    assert!(is_in_old_heap(&process, new_p1.head.to_vaddr()));
}

#[test]
fn cheney_scan_handles_cyclic_structures() {
    let (mut process, mut mem) = setup();

    // Create a cycle: pair.rest points back to itself
    // First allocate with NIL, then update to create cycle
    let pair_addr = process.alloc(16, 8).expect("alloc failed");

    // Write initial pair
    let pair = Pair::new(Term::small_int(1).unwrap(), Term::NIL);
    mem.write(pair_addr, pair);

    // Create Term for the pair
    let pair_term = Term::list_vaddr(pair_addr);

    // Now update rest to point back to itself (creating a cycle)
    let cyclic_pair = Pair::new(Term::small_int(1).unwrap(), pair_term);
    mem.write(pair_addr, cyclic_pair);

    // Copy and scan
    let mut copier = Copier::new(process.old_heap, process.old_hend);
    let new_pair_term = copy_term(&mut copier, &process, &mut mem, pair_term).unwrap();
    copier.scan_copied_objects(&process, &mut mem).unwrap();

    // Verify cycle is preserved
    let new_pair: Pair = mem.read(new_pair_term.to_vaddr());
    assert_eq!(new_pair.head, Term::small_int(1).unwrap());
    // rest should point to itself (new location)
    assert_eq!(new_pair.rest, new_pair_term);
}
