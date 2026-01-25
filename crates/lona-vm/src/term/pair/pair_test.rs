// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for pair cells.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// ============================================================================
// Size and Layout Tests
// ============================================================================

#[test]
fn pair_is_16_bytes() {
    assert_eq!(core::mem::size_of::<Pair>(), 16);
}

#[test]
fn pair_has_8_byte_alignment() {
    assert_eq!(core::mem::align_of::<Pair>(), 8);
}

// ============================================================================
// Pair Construction Tests
// ============================================================================

#[test]
fn pair_new_stores_head_and_rest() {
    let head = Term::small_int(1).unwrap();
    let rest = Term::small_int(2).unwrap();
    let pair = Pair::new(head, rest);

    assert_eq!(pair.head, head);
    assert_eq!(pair.rest, rest);
}

#[test]
fn pair_with_nil_rest() {
    let head = Term::small_int(42).unwrap();
    let pair = Pair::new(head, Term::NIL);

    assert_eq!(pair.head, head);
    assert!(pair.rest.is_nil());
}

// ============================================================================
// List Pointer Tests
// ============================================================================

#[test]
fn list_pointer_round_trip() {
    let pair = Pair::new(Term::small_int(1).unwrap(), Term::NIL);

    // Get pointer to pair (simulating heap allocation)
    let ptr = &raw const pair;

    // Create list term
    let term = Term::list(ptr);

    // Verify it's a list
    assert!(term.is_list());
    assert!(!term.is_nil());
    assert!(!term.is_boxed());
    assert!(!term.is_immediate());

    // Extract pointer back
    let extracted = term.as_pair_ptr().unwrap();
    assert_eq!(extracted, ptr);
}

#[test]
fn list_type_name() {
    let pair = Pair::new(Term::NIL, Term::NIL);
    let term = Term::list(&raw const pair);
    assert_eq!(term.type_name(), "pair");
}

#[test]
fn non_list_returns_none_for_as_pair_ptr() {
    assert!(Term::NIL.as_pair_ptr().is_none());
    assert!(Term::TRUE.as_pair_ptr().is_none());
    assert!(Term::small_int(42).unwrap().as_pair_ptr().is_none());
    assert!(Term::symbol(0).as_pair_ptr().is_none());
}

// ============================================================================
// Head/Rest Access Tests
// ============================================================================

#[test]
fn head_and_rest_access() {
    let head_val = Term::small_int(100).unwrap();
    let rest_val = Term::small_int(200).unwrap();
    let pair = Pair::new(head_val, rest_val);
    let term = Term::list(&raw const pair);

    unsafe {
        assert_eq!(term.head(), Some(head_val));
        assert_eq!(term.rest(), Some(rest_val));
    }
}

#[test]
fn head_rest_on_non_list_returns_none() {
    unsafe {
        assert!(Term::NIL.head().is_none());
        assert!(Term::NIL.rest().is_none());
        assert!(Term::small_int(42).unwrap().head().is_none());
        assert!(Term::small_int(42).unwrap().rest().is_none());
    }
}

// ============================================================================
// Empty List Tests
// ============================================================================

#[test]
fn nil_is_empty_list() {
    assert!(Term::NIL.is_empty_list());
    assert!(Term::NIL.is_nil());
}

#[test]
fn list_pointer_is_not_empty_list() {
    let pair = Pair::new(Term::NIL, Term::NIL);
    let term = Term::list(&raw const pair);

    assert!(!term.is_empty_list());
    assert!(!term.is_nil());
}

// ============================================================================
// Forwarding Tests
// ============================================================================

#[test]
fn new_pair_is_not_forwarded() {
    let pair = Pair::new(Term::small_int(1).unwrap(), Term::NIL);
    assert!(!pair.is_forwarded());
}

#[test]
fn set_forward_marks_as_forwarded() {
    let mut pair = Pair::new(Term::small_int(1).unwrap(), Term::NIL);

    // Create a "new location" for the forwarding test
    let new_pair = Pair::new(Term::small_int(2).unwrap(), Term::NIL);
    let new_addr = &raw const new_pair;

    // SAFETY: new_addr is a valid pointer to a Pair
    unsafe { pair.set_forward(new_addr) };

    assert!(pair.is_forwarded());
}

#[test]
fn forward_address_extraction() {
    let mut pair = Pair::new(Term::small_int(1).unwrap(), Term::NIL);

    let new_pair = Pair::new(Term::small_int(2).unwrap(), Term::NIL);
    let new_addr = &raw const new_pair;

    // SAFETY: new_addr is a valid pointer to a Pair
    unsafe { pair.set_forward(new_addr) };

    assert_eq!(pair.forward_address(), new_addr);
}

#[test]
fn forwarding_overwrites_original_content() {
    let mut pair = Pair::new(Term::small_int(42).unwrap(), Term::small_int(99).unwrap());

    let new_pair = Pair::new(Term::NIL, Term::NIL);
    let new_addr = &raw const new_pair;

    // SAFETY: new_addr is a valid pointer to a Pair
    unsafe { pair.set_forward(new_addr) };

    // Original values are gone - head is now a header marker
    assert!(pair.head.is_header());
    // rest now contains the forwarding address
    assert_eq!(pair.rest.as_raw(), new_addr as u64);
}

// ============================================================================
// Nested List Tests
// ============================================================================

#[test]
fn nested_list_structure() {
    // Build list (1 2 3) as (1 . (2 . (3 . nil)))
    let pair3 = Pair::new(Term::small_int(3).unwrap(), Term::NIL);
    let pair2 = Pair::new(Term::small_int(2).unwrap(), Term::list(&raw const pair3));
    let pair1 = Pair::new(Term::small_int(1).unwrap(), Term::list(&raw const pair2));

    let list = Term::list(&raw const pair1);

    unsafe {
        // First element is 1
        assert_eq!(list.head().unwrap().as_small_int(), Some(1));

        // Rest is a list
        let rest1 = list.rest().unwrap();
        assert!(rest1.is_list());
        assert_eq!(rest1.head().unwrap().as_small_int(), Some(2));

        // Rest of rest is a list
        let rest2 = rest1.rest().unwrap();
        assert!(rest2.is_list());
        assert_eq!(rest2.head().unwrap().as_small_int(), Some(3));

        // Rest of rest of rest is nil
        let rest3 = rest2.rest().unwrap();
        assert!(rest3.is_nil());
    }
}

// ============================================================================
// Mutable Pointer Tests
// ============================================================================

#[test]
#[allow(unused_mut)]
fn mutable_pair_ptr_access() {
    let mut pair = Pair::new(Term::small_int(1).unwrap(), Term::NIL);
    let term = Term::list(&raw const pair);

    // Get mutable pointer
    let ptr_mut = term.as_pair_ptr_mut().unwrap();

    // Modify through pointer
    unsafe {
        (*ptr_mut).head = Term::small_int(999).unwrap();
    }

    // Verify modification
    assert_eq!(pair.head.as_small_int(), Some(999));
}
