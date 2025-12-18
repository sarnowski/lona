// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the persistent vector implementation.

use alloc::vec::Vec;

use super::PersistentVec;

// =============================================================================
// Construction Tests
// =============================================================================

#[test]
fn new_creates_empty_vec() {
    let pvec: PersistentVec<i32> = PersistentVec::new();
    assert!(pvec.is_empty());
    assert_eq!(pvec.len(), 0);
}

#[test]
fn from_vec_empty() {
    let pvec: PersistentVec<i32> = PersistentVec::from_vec(Vec::new());
    assert!(pvec.is_empty());
}

#[test]
fn from_vec_elements() {
    let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    assert_eq!(pvec.len(), 3);
    assert_eq!(pvec.get(0), Some(&1));
    assert_eq!(pvec.get(1), Some(&2));
    assert_eq!(pvec.get(2), Some(&3));
}

// =============================================================================
// Push Tests
// =============================================================================

#[test]
fn push_single_element() {
    let pvec: PersistentVec<i32> = PersistentVec::new();
    let pvec = pvec.push(42);
    assert_eq!(pvec.len(), 1);
    assert_eq!(pvec.get(0), Some(&42));
}

#[test]
fn push_multiple_elements() {
    let mut pvec: PersistentVec<i32> = PersistentVec::new();
    for i in 0..10 {
        pvec = pvec.push(i);
    }
    assert_eq!(pvec.len(), 10);
    for i in 0..10 {
        assert_eq!(pvec.get(i), Some(&(i as i32)));
    }
}

#[test]
fn push_fills_tail_then_trie() {
    let mut pvec: PersistentVec<i32> = PersistentVec::new();
    // Push more than WIDTH elements to force trie creation
    for i in 0..50 {
        pvec = pvec.push(i);
    }
    assert_eq!(pvec.len(), 50);
    for i in 0..50 {
        assert_eq!(pvec.get(i), Some(&(i as i32)));
    }
}

#[test]
fn push_large_vector() {
    let mut pvec: PersistentVec<i32> = PersistentVec::new();
    let count: i32 = 1000;
    for i in 0..count {
        pvec = pvec.push(i);
    }
    assert_eq!(pvec.len(), count as usize);
    for i in 0..count {
        assert_eq!(pvec.get(i as usize), Some(&i));
    }
}

// =============================================================================
// Get Tests
// =============================================================================

#[test]
fn get_out_of_bounds() {
    let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    assert_eq!(pvec.get(5), None);
}

#[test]
fn get_empty() {
    let pvec: PersistentVec<i32> = PersistentVec::new();
    assert_eq!(pvec.get(0), None);
}

// =============================================================================
// Assoc Tests
// =============================================================================

#[test]
fn assoc_in_tail() {
    let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    let new_pvec = pvec.assoc(1, 42).unwrap();

    // Original unchanged
    assert_eq!(pvec.get(1), Some(&2));
    // New has updated value
    assert_eq!(new_pvec.get(1), Some(&42));
    // Other elements unchanged
    assert_eq!(new_pvec.get(0), Some(&1));
    assert_eq!(new_pvec.get(2), Some(&3));
}

#[test]
fn assoc_in_trie() {
    let mut pvec: PersistentVec<i32> = PersistentVec::new();
    for i in 0..50 {
        pvec = pvec.push(i);
    }

    let new_pvec = pvec.assoc(5, 999).unwrap();

    // Original unchanged
    assert_eq!(pvec.get(5), Some(&5));
    // New has updated value
    assert_eq!(new_pvec.get(5), Some(&999));
}

#[test]
fn assoc_out_of_bounds() {
    let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    assert!(pvec.assoc(10, 42).is_none());
}

// =============================================================================
// Pop Tests
// =============================================================================

#[test]
fn pop_empty() {
    let pvec: PersistentVec<i32> = PersistentVec::new();
    assert!(pvec.pop().is_none());
}

#[test]
fn pop_single_element() {
    let pvec = PersistentVec::from_vec(alloc::vec![42]);
    let popped = pvec.pop().unwrap();

    assert!(popped.is_empty());
    // Original unchanged
    assert_eq!(pvec.len(), 1);
}

#[test]
fn pop_multiple_elements_in_tail() {
    let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    let popped = pvec.pop().unwrap();

    assert_eq!(popped.len(), 2);
    assert_eq!(popped.get(0), Some(&1));
    assert_eq!(popped.get(1), Some(&2));
    // Original unchanged
    assert_eq!(pvec.len(), 3);
}

#[test]
fn pop_across_tail_boundary() {
    // Create a vector with more than 32 elements
    let mut pvec: PersistentVec<i32> = PersistentVec::new();
    for i in 0..35 {
        pvec = pvec.push(i);
    }

    // Pop should work correctly across the boundary
    let popped = pvec.pop().unwrap();
    assert_eq!(popped.len(), 34);
    for i in 0..34 {
        assert_eq!(popped.get(i), Some(&(i as i32)));
    }
}

#[test]
fn pop_preserves_original() {
    let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    let v2 = v1.pop().unwrap();

    // v1 unchanged
    assert_eq!(v1.len(), 3);
    assert_eq!(v1.get(2), Some(&3));

    // v2 has one less element
    assert_eq!(v2.len(), 2);
    assert_eq!(v2.get(2), None);
}

#[test]
fn pop_then_push() {
    let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    let v2 = v1.pop().unwrap();
    let v3 = v2.push(99);

    assert_eq!(v3.len(), 3);
    assert_eq!(v3.get(0), Some(&1));
    assert_eq!(v3.get(1), Some(&2));
    assert_eq!(v3.get(2), Some(&99));
}

// =============================================================================
// Structural Sharing Tests
// =============================================================================

#[test]
fn push_preserves_original() {
    let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    let v2 = v1.push(4);

    // v1 unchanged
    assert_eq!(v1.len(), 3);
    assert_eq!(v1.get(0), Some(&1));
    assert_eq!(v1.get(1), Some(&2));
    assert_eq!(v1.get(2), Some(&3));

    // v2 has new element
    assert_eq!(v2.len(), 4);
    assert_eq!(v2.get(3), Some(&4));
}

#[test]
fn assoc_preserves_original() {
    let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    let v2 = v1.assoc(1, 42).unwrap();

    // v1 unchanged
    assert_eq!(v1.get(1), Some(&2));
    // v2 has updated value
    assert_eq!(v2.get(1), Some(&42));
}

// =============================================================================
// Iterator Tests
// =============================================================================

#[test]
fn iter_empty() {
    let pvec: PersistentVec<i32> = PersistentVec::new();
    let collected: Vec<_> = pvec.iter().collect();
    assert!(collected.is_empty());
}

#[test]
fn iter_elements() {
    let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    let collected: Vec<_> = pvec.iter().cloned().collect();
    assert_eq!(collected, alloc::vec![1, 2, 3]);
}

#[test]
fn iter_large() {
    let mut pvec: PersistentVec<i32> = PersistentVec::new();
    for i in 0..100 {
        pvec = pvec.push(i);
    }
    let collected: Vec<_> = pvec.iter().cloned().collect();
    let expected: Vec<i32> = (0..100).collect();
    assert_eq!(collected, expected);
}

#[test]
fn iter_size_hint() {
    let pvec = PersistentVec::from_vec(alloc::vec![1, 2, 3, 4, 5]);
    let iter = pvec.iter();
    assert_eq!(iter.size_hint(), (5, Some(5)));
}

// =============================================================================
// Clone Tests
// =============================================================================

#[test]
fn clone_shares_structure() {
    let v1 = PersistentVec::from_vec(alloc::vec![1, 2, 3]);
    let v2 = v1.clone();

    assert_eq!(v1.len(), v2.len());
    for i in 0..v1.len() {
        assert_eq!(v1.get(i), v2.get(i));
    }
}

// =============================================================================
// Default Test
// =============================================================================

#[test]
fn default_is_empty() {
    let pvec: PersistentVec<i32> = PersistentVec::default();
    assert!(pvec.is_empty());
}
