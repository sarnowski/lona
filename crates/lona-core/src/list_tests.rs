// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the List type.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::integer::Integer;
use crate::value::Value;

use super::List;

/// Helper to create an integer value.
fn int(value: i64) -> Value {
    Value::Integer(Integer::from_i64(value))
}

// =========================================================================
// Construction Tests
// =========================================================================

#[test]
fn empty_list() {
    let list = List::empty();
    assert!(list.is_empty());
    assert_eq!(list.len(), 0);
    assert!(list.first().is_none());
}

#[test]
fn cons_single_element() {
    let list = List::empty().cons(int(1));
    assert!(!list.is_empty());
    assert_eq!(list.len(), 1);
    assert_eq!(list.first(), Some(&int(1)));
}

#[test]
fn cons_multiple_elements() {
    let list = List::empty().cons(int(3)).cons(int(2)).cons(int(1));
    assert_eq!(list.len(), 3);
    assert_eq!(list.first(), Some(&int(1)));
}

#[test]
fn from_vec_empty() {
    let list = List::from_vec(Vec::new());
    assert!(list.is_empty());
}

#[test]
fn from_vec_elements() {
    let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
    assert_eq!(list.len(), 3);
    assert_eq!(list.first(), Some(&int(1)));
}

// =========================================================================
// Access Tests
// =========================================================================

#[test]
fn first_empty() {
    let list = List::empty();
    assert!(list.first().is_none());
}

#[test]
fn first_non_empty() {
    let list = List::empty().cons(int(42));
    assert_eq!(list.first(), Some(&int(42)));
}

#[test]
fn rest_empty() {
    let list = List::empty();
    let rest = list.rest();
    assert!(rest.is_empty());
}

#[test]
fn rest_single_element() {
    let list = List::empty().cons(int(1));
    let rest = list.rest();
    assert!(rest.is_empty());
}

#[test]
fn rest_multiple_elements() {
    let list = List::empty().cons(int(3)).cons(int(2)).cons(int(1));
    let rest = list.rest();
    assert_eq!(rest.len(), 2);
    assert_eq!(rest.first(), Some(&int(2)));
}

// =========================================================================
// Iterator Tests
// =========================================================================

#[test]
fn iter_empty() {
    let list = List::empty();
    let collected: Vec<_> = list.iter().collect();
    assert!(collected.is_empty());
}

#[test]
fn iter_elements() {
    let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
    let collected: Vec<_> = list.iter().cloned().collect();
    assert_eq!(collected, alloc::vec![int(1), int(2), int(3)]);
}

#[test]
fn into_iterator() {
    let list = List::from_vec(alloc::vec![int(1), int(2)]);
    let mut count = 0_usize;
    for _val in &list {
        count = count.saturating_add(1);
    }
    assert_eq!(count, 2);
}

// =========================================================================
// Display Tests
// =========================================================================

#[test]
fn display_empty() {
    let list = List::empty();
    assert_eq!(list.to_string(), "()");
}

#[test]
fn display_single() {
    let list = List::empty().cons(int(42));
    assert_eq!(list.to_string(), "(42)");
}

#[test]
fn display_multiple() {
    let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
    assert_eq!(list.to_string(), "(1 2 3)");
}

// =========================================================================
// Equality Tests
// =========================================================================

#[test]
fn equality_empty() {
    let l1 = List::empty();
    let l2 = List::empty();
    assert_eq!(l1, l2);
}

#[test]
fn equality_same_elements() {
    let l1 = List::from_vec(alloc::vec![int(1), int(2)]);
    let l2 = List::from_vec(alloc::vec![int(1), int(2)]);
    assert_eq!(l1, l2);
}

#[test]
fn equality_different_elements() {
    let l1 = List::from_vec(alloc::vec![int(1), int(2)]);
    let l2 = List::from_vec(alloc::vec![int(1), int(3)]);
    assert_ne!(l1, l2);
}

#[test]
fn equality_different_lengths() {
    let l1 = List::from_vec(alloc::vec![int(1), int(2)]);
    let l2 = List::from_vec(alloc::vec![int(1)]);
    assert_ne!(l1, l2);
}

// =========================================================================
// Clone/Sharing Tests
// =========================================================================

#[test]
fn clone_shares_tail() {
    let original = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
    let cloned = original.clone();

    // Both should have the same elements
    assert_eq!(original, cloned);
    assert_eq!(original.len(), cloned.len());
}

#[test]
fn structural_sharing() {
    let base = List::from_vec(alloc::vec![int(2), int(3)]);
    let extended = base.cons(int(1));

    // The base list is unaffected
    assert_eq!(base.len(), 2);
    assert_eq!(extended.len(), 3);

    // The tail of extended should equal base
    assert_eq!(extended.rest(), base);
}

// =========================================================================
// Default Test
// =========================================================================

#[test]
fn default_is_empty() {
    let list: List = List::default();
    assert!(list.is_empty());
}

// =========================================================================
// Nested List Tests
// =========================================================================

#[test]
fn nested_lists() {
    let inner = List::from_vec(alloc::vec![int(1), int(2)]);
    let outer = List::empty().cons(Value::List(inner.clone()));
    assert_eq!(outer.len(), 1);

    if let Some(Value::List(inner_list)) = outer.first() {
        assert_eq!(inner_list.len(), 2);
    } else {
        panic!("Expected List value");
    }
}
