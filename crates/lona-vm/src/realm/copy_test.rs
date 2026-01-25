// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for deep copy to realm functionality.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::items_after_statements
)]

use crate::Vaddr;
use crate::platform::{MemorySpace, MockVSpace};
use crate::process::Process;
use crate::realm::{Realm, VisitedTracker, deep_copy_term_to_realm};
use crate::term::Term;
use crate::term::heap::{HeapPair, HeapString, HeapTuple};

/// Create a test setup with process, realm, and memory.
fn setup() -> (Process, Realm, MockVSpace) {
    // Memory layout:
    // 0x1000-0x2000: Process young heap
    // 0x2000-0x3000: Process old heap
    // 0x4000-0x8000: Realm code region
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x1000));
    let proc = Process::new(Vaddr::new(0x1000), 0x1000, Vaddr::new(0x2000), 0x1000);
    let realm = Realm::new(Vaddr::new(0x4000), 0x4000);
    (proc, realm, mem)
}

#[test]
fn test_visited_tracker_basic() {
    let mut tracker = VisitedTracker::new();

    // Initially empty
    assert!(tracker.check(Vaddr::new(0x1000)).is_none());

    // Record a mapping
    assert!(tracker.record(Vaddr::new(0x1000), Vaddr::new(0x4000)));

    // Should find it now
    assert_eq!(tracker.check(Vaddr::new(0x1000)), Some(Vaddr::new(0x4000)));

    // Different address should not be found
    assert!(tracker.check(Vaddr::new(0x1100)).is_none());
}

#[test]
fn test_deep_copy_immediates() {
    let (_, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Nil
    let result = deep_copy_term_to_realm(Term::NIL, &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(Term::NIL));

    // Bool
    let result = deep_copy_term_to_realm(Term::bool(true), &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(Term::bool(true)));

    // Int
    let int_term = Term::small_int(42).unwrap();
    let result = deep_copy_term_to_realm(int_term, &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(int_term));

    // NativeFn
    let native = Term::native_fn(5);
    let result = deep_copy_term_to_realm(native, &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(native));

    // Unbound
    let result = deep_copy_term_to_realm(Term::UNBOUND, &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(Term::UNBOUND));
}

#[test]
fn test_deep_copy_string() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Allocate string on process heap
    let src = proc.alloc_term_string(&mut mem, "hello").unwrap();

    // Copy to realm
    let dst = deep_copy_term_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a string at different address
    assert!(dst.is_boxed());
    let src_addr = src.to_vaddr();
    let dst_addr = dst.to_vaddr();
    assert_ne!(src_addr, dst_addr);

    // Contents should match
    let src_header: HeapString = mem.read(src_addr);
    let dst_header: HeapString = mem.read(dst_addr);
    assert_eq!(src_header.len(), dst_header.len());

    let src_data = src_addr.add(HeapString::HEADER_SIZE as u64);
    let dst_data = dst_addr.add(HeapString::HEADER_SIZE as u64);
    let src_bytes = mem.slice(src_data, src_header.len());
    let dst_bytes = mem.slice(dst_data, dst_header.len());
    assert_eq!(src_bytes, dst_bytes);
    assert_eq!(src_bytes, b"hello");

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);
    assert!(dst_addr.as_u64() < 0x8000);
}

#[test]
fn test_deep_copy_symbol() {
    let (_, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Symbols are now immediate values interned at realm level
    let src = realm.intern_symbol(&mut mem, "foo").unwrap();

    // Deep copy of immediate value is identity
    let dst = deep_copy_term_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be the same immediate value
    assert!(dst.is_symbol());
    assert_eq!(src, dst);

    // Interning same symbol again should return same value
    let src2 = realm.intern_symbol(&mut mem, "foo").unwrap();
    assert_eq!(src, src2);
}

#[test]
fn test_deep_copy_keyword() {
    let (_, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Keywords are now immediate values interned at realm level
    let src = realm.intern_keyword(&mut mem, "bar").unwrap();

    // Deep copy of immediate value is identity
    let dst = deep_copy_term_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be the same immediate value
    assert!(dst.is_keyword());
    assert_eq!(src, dst);

    // Interning same keyword again should return same value
    let src2 = realm.intern_keyword(&mut mem, "bar").unwrap();
    assert_eq!(src, src2);
}

#[test]
fn test_deep_copy_pair() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create a simple pair: (1 . 2)
    let one = Term::small_int(1).unwrap();
    let two = Term::small_int(2).unwrap();
    let src = proc.alloc_term_pair(&mut mem, one, two).unwrap();

    // Copy to realm
    let dst = deep_copy_term_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a list at different address
    assert!(dst.is_list());
    let src_addr = src.to_vaddr();
    let dst_addr = dst.to_vaddr();
    assert_ne!(src_addr, dst_addr);

    // Contents should match
    let src_pair: HeapPair = mem.read(src_addr);
    let dst_pair: HeapPair = mem.read(dst_addr);
    assert_eq!(src_pair.head, dst_pair.head);
    assert_eq!(src_pair.tail, dst_pair.tail);

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_nested_pair() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create nested pair: (1 2 3) = (1 . (2 . (3 . nil)))
    let one = Term::small_int(1).unwrap();
    let two = Term::small_int(2).unwrap();
    let three = Term::small_int(3).unwrap();

    let inner = proc.alloc_term_pair(&mut mem, three, Term::NIL).unwrap();
    let middle = proc.alloc_term_pair(&mut mem, two, inner).unwrap();
    let src = proc.alloc_term_pair(&mut mem, one, middle).unwrap();

    // Copy to realm
    let dst = deep_copy_term_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Walk the copied list and verify structure
    let p1_addr = dst.to_vaddr();
    let p1: HeapPair = mem.read(p1_addr);
    assert_eq!(p1.head, one);

    let p2_addr = p1.tail.to_vaddr();
    let p2: HeapPair = mem.read(p2_addr);
    assert_eq!(p2.head, two);

    let p3_addr = p2.tail.to_vaddr();
    let p3: HeapPair = mem.read(p3_addr);
    assert_eq!(p3.head, three);
    assert!(p3.tail.is_nil());

    // All pair addresses should be in realm region
    assert!(p1_addr.as_u64() >= 0x4000);
    assert!(p2_addr.as_u64() >= 0x4000);
    assert!(p3_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_tuple() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create tuple [1 2 3]
    let one = Term::small_int(1).unwrap();
    let two = Term::small_int(2).unwrap();
    let three = Term::small_int(3).unwrap();
    let elements = [one, two, three];
    let src = proc.alloc_term_tuple(&mut mem, &elements).unwrap();

    // Copy to realm
    let dst = deep_copy_term_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a boxed value at different address
    assert!(dst.is_boxed());
    let src_addr = src.to_vaddr();
    let dst_addr = dst.to_vaddr();
    assert_ne!(src_addr, dst_addr);

    // Check header
    let dst_header: HeapTuple = mem.read(dst_addr);
    assert_eq!(dst_header.len(), 3);

    // Check elements
    let elem_base = dst_addr.add(HeapTuple::HEADER_SIZE as u64);
    for (i, expected) in elements.iter().enumerate() {
        let elem: Term = mem.read(elem_base.add((i * core::mem::size_of::<Term>()) as u64));
        assert_eq!(elem, *expected);
    }

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_tuple_with_nested_values() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create tuple with string element: [1 "hello" 3]
    let one = Term::small_int(1).unwrap();
    let three = Term::small_int(3).unwrap();
    let s = proc.alloc_term_string(&mut mem, "hello").unwrap();
    let elements = [one, s, three];
    let src = proc.alloc_term_tuple(&mut mem, &elements).unwrap();

    // Copy to realm
    let dst = deep_copy_term_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Check the string element was deep copied
    let dst_addr = dst.to_vaddr();
    let elem_base = dst_addr.add(HeapTuple::HEADER_SIZE as u64);
    let copied_str: Term = mem.read(elem_base.add(core::mem::size_of::<Term>() as u64));

    assert!(copied_str.is_boxed());
    let str_addr = copied_str.to_vaddr();

    // String should be in realm region
    assert!(str_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_shared_structure() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create a string that's shared in two places
    let shared_str = proc.alloc_term_string(&mut mem, "shared").unwrap();

    // Create two pairs that share the same string
    let pair1 = proc
        .alloc_term_pair(&mut mem, shared_str, Term::NIL)
        .unwrap();
    let pair2 = proc
        .alloc_term_pair(&mut mem, shared_str, Term::NIL)
        .unwrap();

    // Create a tuple containing both pairs
    let src = proc.alloc_term_tuple(&mut mem, &[pair1, pair2]).unwrap();

    // Copy to realm
    let dst = deep_copy_term_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Extract the copied pairs
    let tuple_addr = dst.to_vaddr();
    let elem_base = tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
    let copied_pair1: Term = mem.read(elem_base);
    let copied_pair2: Term = mem.read(elem_base.add(core::mem::size_of::<Term>() as u64));

    // Extract the strings from each pair
    let p1_addr = copied_pair1.to_vaddr();
    let p2_addr = copied_pair2.to_vaddr();
    let p1: HeapPair = mem.read(p1_addr);
    let p2: HeapPair = mem.read(p2_addr);

    // Both pairs should have the SAME string address (shared structure preserved)
    assert_eq!(p1.head, p2.head);

    // And it should be in realm region
    let str_addr = p1.head.to_vaddr();
    assert!(str_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_oom() {
    // Create a very small realm that will run out of memory
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x1000));
    let proc = Process::new(Vaddr::new(0x1000), 0x1000, Vaddr::new(0x2000), 0x1000);
    // Realm with only 8 bytes - definitely too small for any heap allocation
    // HeapString header alone is 8 bytes, plus we need alignment
    let mut realm = Realm::new(Vaddr::new(0x4000), 8);
    let mut mem = mem;
    let mut proc = proc;
    let mut visited = VisitedTracker::new();

    // Allocate a string on process heap - needs header (8 bytes) + content
    let s = proc.alloc_term_string(&mut mem, "hello world").unwrap();

    // Try to copy to realm - should fail due to OOM
    // String needs 8 bytes header + 11 bytes content = 19 bytes minimum
    let result = deep_copy_term_to_realm(s, &mut realm, &mut mem, &mut visited);
    assert!(
        result.is_none(),
        "deep copy should fail when realm is out of memory"
    );
}
