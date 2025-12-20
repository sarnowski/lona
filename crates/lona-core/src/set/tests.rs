// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the set implementation.

use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use core::cmp::Ordering;

use crate::integer::Integer;
use crate::string::HeapStr;
use crate::value::Value;

use super::Set;

/// Helper to create an integer value.
fn int(value: i64) -> Value {
    Value::Integer(Integer::from_i64(value))
}

/// Helper to create a string value.
fn string(text: &str) -> Value {
    Value::String(HeapStr::new(text))
}

// =============================================================================
// Construction Tests
// =============================================================================

#[test]
fn empty_set() {
    let set = Set::empty();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[test]
fn from_values() {
    let set = Set::from_values(vec![int(1), int(2), int(3)]);
    assert!(!set.is_empty());
    assert_eq!(set.len(), 3);
}

#[test]
fn from_values_empty() {
    let set = Set::from_values(Vec::<Value>::new());
    assert!(set.is_empty());
}

#[test]
fn from_values_with_duplicates() {
    let set = Set::from_values(vec![int(1), int(2), int(1)]);
    assert_eq!(set.len(), 2);
}

// =============================================================================
// Access Tests
// =============================================================================

#[test]
fn contains_existing() {
    let set = Set::from_values(vec![int(1), int(2), int(3)]);
    assert!(set.contains(&int(1)));
    assert!(set.contains(&int(2)));
    assert!(set.contains(&int(3)));
}

#[test]
fn contains_missing() {
    let set = Set::from_values(vec![int(1), int(2)]);
    assert!(!set.contains(&int(3)));
}

#[test]
fn contains_empty() {
    let set = Set::empty();
    assert!(!set.contains(&int(1)));
}

// =============================================================================
// Mutation Tests (Copy-on-Write)
// =============================================================================

#[test]
fn insert_new_element() {
    let set = Set::empty();
    let new_set = set.insert(int(1));

    assert!(set.is_empty());
    assert_eq!(new_set.len(), 1);
    assert!(new_set.contains(&int(1)));
}

#[test]
fn insert_existing_element() {
    let set = Set::from_values(vec![int(1)]);
    let new_set = set.insert(int(1));

    // Size unchanged since element already exists
    assert_eq!(new_set.len(), 1);
    assert!(new_set.contains(&int(1)));
}

#[test]
fn insert_multiple() {
    let set = Set::empty().insert(int(1)).insert(int(2)).insert(int(3));

    assert_eq!(set.len(), 3);
}

#[test]
fn remove_existing() {
    let set = Set::from_values(vec![int(1), int(2), int(3)]);
    let new_set = set.remove(&int(2));

    // Original unchanged
    assert_eq!(set.len(), 3);
    assert!(set.contains(&int(2)));

    // New set has element removed
    assert_eq!(new_set.len(), 2);
    assert!(!new_set.contains(&int(2)));
    assert!(new_set.contains(&int(1)));
    assert!(new_set.contains(&int(3)));
}

#[test]
fn remove_missing() {
    let set = Set::from_values(vec![int(1)]);
    let new_set = set.remove(&int(999));

    // Same length, nothing removed
    assert_eq!(new_set.len(), 1);
}

// =============================================================================
// Iterator Tests
// =============================================================================

#[test]
fn iter() {
    let set = Set::from_values(vec![int(1), int(2), int(3)]);
    let elements: Vec<_> = set.iter().collect();
    assert_eq!(elements.len(), 3);
}

#[test]
fn iter_empty() {
    let set = Set::empty();
    let elements: Vec<_> = set.iter().collect();
    assert!(elements.is_empty());
}

// =============================================================================
// Display Tests
// =============================================================================

#[test]
fn display_empty() {
    let set = Set::empty();
    assert_eq!(set.to_string(), "#{}");
}

#[test]
fn display_single() {
    let set = Set::from_values(vec![int(42)]);
    assert_eq!(set.to_string(), "#{42}");
}

// =============================================================================
// Equality Tests
// =============================================================================

#[test]
fn equality_empty() {
    let s1 = Set::empty();
    let s2 = Set::empty();
    assert_eq!(s1, s2);
}

#[test]
fn equality_same_elements() {
    let s1 = Set::from_values(vec![int(1), int(2), int(3)]);
    let s2 = Set::from_values(vec![int(3), int(2), int(1)]);
    assert_eq!(s1, s2);
}

#[test]
fn equality_different_elements() {
    let s1 = Set::from_values(vec![int(1), int(2)]);
    let s2 = Set::from_values(vec![int(1), int(3)]);
    assert_ne!(s1, s2);
}

#[test]
fn equality_different_sizes() {
    let s1 = Set::from_values(vec![int(1)]);
    let s2 = Set::from_values(vec![int(1), int(2)]);
    assert_ne!(s1, s2);
}

// =============================================================================
// Ordering Tests
// =============================================================================

#[test]
fn ordering_by_size() {
    let s1 = Set::from_values(vec![int(1)]);
    let s2 = Set::from_values(vec![int(1), int(2)]);
    assert_eq!(s1.cmp(&s2), Ordering::Less);
}

#[test]
fn ordering_same_size_different_elements() {
    let s1 = Set::from_values(vec![int(1)]);
    let s2 = Set::from_values(vec![int(2)]);
    assert_eq!(s1.cmp(&s2), Ordering::Less);
}

#[test]
fn ordering_equal() {
    let s1 = Set::from_values(vec![int(1), int(2)]);
    let s2 = Set::from_values(vec![int(2), int(1)]);
    assert_eq!(s1.cmp(&s2), Ordering::Equal);
}

// =============================================================================
// Hash Tests
// =============================================================================

#[test]
fn hash_equal_sets() {
    use crate::fnv::FnvHasher;
    use core::hash::{Hash, Hasher};

    let s1 = Set::from_values(vec![int(1), int(2), int(3)]);
    let s2 = Set::from_values(vec![int(3), int(2), int(1)]);

    let mut h1 = FnvHasher::default();
    let mut h2 = FnvHasher::default();
    s1.hash(&mut h1);
    s2.hash(&mut h2);

    assert_eq!(h1.finish(), h2.finish());
}

// =============================================================================
// Clone Tests
// =============================================================================

#[test]
fn clone_shares_data() {
    let s1 = Set::from_values(vec![int(1), int(2)]);
    let s2 = s1.clone();

    assert_eq!(s1, s2);
}

// =============================================================================
// Default Test
// =============================================================================

#[test]
fn default_is_empty() {
    let set: Set = Set::default();
    assert!(set.is_empty());
}

// =============================================================================
// Mixed Type Tests
// =============================================================================

#[test]
fn mixed_types() {
    let set = Set::from_values(vec![
        Value::Nil,
        Value::Bool(true),
        int(42),
        string("hello"),
    ]);
    assert_eq!(set.len(), 4);
    assert!(set.contains(&Value::Nil));
    assert!(set.contains(&Value::Bool(true)));
    assert!(set.contains(&int(42)));
    assert!(set.contains(&string("hello")));
}

// =============================================================================
// Structural Sharing Tests
// =============================================================================

#[test]
fn structural_sharing_on_insert() {
    let s1 = Set::from_values(vec![int(1)]);
    let s2 = s1.insert(int(2));

    // s1 unchanged
    assert_eq!(s1.len(), 1);
    assert!(s1.contains(&int(1)));
    assert!(!s1.contains(&int(2)));

    // s2 has both
    assert_eq!(s2.len(), 2);
    assert!(s2.contains(&int(1)));
    assert!(s2.contains(&int(2)));
}

#[test]
fn structural_sharing_on_remove() {
    let s1 = Set::from_values(vec![int(1), int(2)]);
    let s2 = s1.remove(&int(1));

    // s1 unchanged
    assert_eq!(s1.len(), 2);
    assert!(s1.contains(&int(1)));

    // s2 has removal
    assert_eq!(s2.len(), 1);
    assert!(!s2.contains(&int(1)));
    assert!(s2.contains(&int(2)));
}

// =============================================================================
// Large Set Tests
// =============================================================================

#[test]
fn large_set_operations() {
    let mut set = Set::empty();
    let count = 100_i64;

    for i in 0..count {
        set = set.insert(int(i));
    }

    assert_eq!(set.len(), count as usize);

    for i in 0..count {
        assert!(set.contains(&int(i)));
    }
}
