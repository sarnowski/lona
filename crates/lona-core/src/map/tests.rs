// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the map implementation.

use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use core::cmp::Ordering;

use crate::integer::Integer;
use crate::string::HeapStr;
use crate::value::Value;

use super::{Map, ValueKey};

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
fn empty_map() {
    let map = Map::empty();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn from_pairs() {
    let map = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
    assert!(!map.is_empty());
    assert_eq!(map.len(), 2);
}

#[test]
fn from_pairs_empty() {
    let map = Map::from_pairs(Vec::<(Value, Value)>::new());
    assert!(map.is_empty());
}

// =============================================================================
// Access Tests
// =============================================================================

#[test]
fn get_existing_key() {
    let map = Map::from_pairs(vec![(string("key"), int(42))]);
    assert_eq!(map.get(&string("key")), Some(&int(42)));
}

#[test]
fn get_missing_key() {
    let map = Map::from_pairs(vec![(string("key"), int(42))]);
    assert_eq!(map.get(&string("other")), None);
}

#[test]
fn get_empty() {
    let map = Map::empty();
    assert_eq!(map.get(&string("key")), None);
}

#[test]
fn contains_key() {
    let map = Map::from_pairs(vec![(string("a"), int(1))]);
    assert!(map.contains_key(&string("a")));
    assert!(!map.contains_key(&string("b")));
}

// =============================================================================
// Mutation Tests (Copy-on-Write)
// =============================================================================

#[test]
fn assoc_new_key() {
    let map = Map::empty();
    let new_map = map.assoc(string("key"), int(42));

    assert!(map.is_empty());
    assert_eq!(new_map.len(), 1);
    assert_eq!(new_map.get(&string("key")), Some(&int(42)));
}

#[test]
fn assoc_existing_key() {
    let map = Map::from_pairs(vec![(string("key"), int(1))]);
    let new_map = map.assoc(string("key"), int(2));

    // Original unchanged
    assert_eq!(map.get(&string("key")), Some(&int(1)));
    // New map has updated value
    assert_eq!(new_map.get(&string("key")), Some(&int(2)));
}

#[test]
fn dissoc_existing_key() {
    let map = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
    let new_map = map.dissoc(&string("a"));

    // Original unchanged
    assert_eq!(map.len(), 2);
    // New map has key removed
    assert_eq!(new_map.len(), 1);
    assert!(!new_map.contains_key(&string("a")));
    assert!(new_map.contains_key(&string("b")));
}

#[test]
fn dissoc_missing_key() {
    let map = Map::from_pairs(vec![(string("a"), int(1))]);
    let new_map = map.dissoc(&string("missing"));

    // Same length, nothing removed
    assert_eq!(new_map.len(), 1);
}

// =============================================================================
// Iterator Tests
// =============================================================================

#[test]
fn keys_iterator() {
    let map = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
    let keys: Vec<_> = map.keys().cloned().collect();
    assert_eq!(keys.len(), 2);
}

#[test]
fn values_iterator() {
    let map = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
    let values: Vec<_> = map.values().cloned().collect();
    assert_eq!(values.len(), 2);
}

#[test]
fn iter() {
    let map = Map::from_pairs(vec![(string("a"), int(1))]);
    let entries: Vec<_> = map.iter().collect();
    assert_eq!(entries.len(), 1);
}

// =============================================================================
// Display Tests
// =============================================================================

#[test]
fn display_empty() {
    let map = Map::empty();
    assert_eq!(map.to_string(), "{}");
}

#[test]
fn display_single() {
    let map = Map::from_pairs(vec![(int(1), int(2))]);
    assert_eq!(map.to_string(), "{1 2}");
}

// =============================================================================
// Equality Tests
// =============================================================================

#[test]
fn equality_empty() {
    let m1 = Map::empty();
    let m2 = Map::empty();
    assert_eq!(m1, m2);
}

#[test]
fn equality_same_entries() {
    let m1 = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
    let m2 = Map::from_pairs(vec![(string("b"), int(2)), (string("a"), int(1))]);
    assert_eq!(m1, m2);
}

#[test]
fn equality_different_values() {
    let m1 = Map::from_pairs(vec![(string("a"), int(1))]);
    let m2 = Map::from_pairs(vec![(string("a"), int(2))]);
    assert_ne!(m1, m2);
}

#[test]
fn equality_different_keys() {
    let m1 = Map::from_pairs(vec![(string("a"), int(1))]);
    let m2 = Map::from_pairs(vec![(string("b"), int(1))]);
    assert_ne!(m1, m2);
}

#[test]
fn equality_different_sizes() {
    let m1 = Map::from_pairs(vec![(string("a"), int(1))]);
    let m2 = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
    assert_ne!(m1, m2);
}

// =============================================================================
// Clone Tests
// =============================================================================

#[test]
fn clone_shares_data() {
    let m1 = Map::from_pairs(vec![(string("a"), int(1))]);
    let m2 = m1.clone();

    // Both have the same elements
    assert_eq!(m1, m2);
}

// =============================================================================
// Default Test
// =============================================================================

#[test]
fn default_is_empty() {
    let map: Map = Map::default();
    assert!(map.is_empty());
}

// =============================================================================
// ValueKey Ordering Tests
// =============================================================================

#[test]
fn value_key_nil_ordering() {
    let nil1 = ValueKey::new(Value::Nil);
    let nil2 = ValueKey::new(Value::Nil);
    assert_eq!(nil1.cmp(&nil2), Ordering::Equal);
}

#[test]
fn value_key_bool_ordering() {
    let false_key = ValueKey::new(Value::Bool(false));
    let true_key = ValueKey::new(Value::Bool(true));
    assert_eq!(false_key.cmp(&true_key), Ordering::Less);
}

#[test]
fn value_key_integer_ordering() {
    let k1 = ValueKey::new(int(1));
    let k2 = ValueKey::new(int(2));
    assert_eq!(k1.cmp(&k2), Ordering::Less);
}

#[test]
fn value_key_cross_type_ordering() {
    let nil_key = ValueKey::new(Value::Nil);
    let bool_key = ValueKey::new(Value::Bool(true));
    let int_key = ValueKey::new(int(1));
    let string_key = ValueKey::new(string("a"));

    // Nil < Bool < Integer < ... < String
    assert_eq!(nil_key.cmp(&bool_key), Ordering::Less);
    assert_eq!(bool_key.cmp(&int_key), Ordering::Less);
    assert_eq!(int_key.cmp(&string_key), Ordering::Less);
}

#[test]
fn value_key_float_nan_ordering() {
    let nan = ValueKey::new(Value::Float(f64::NAN));
    let number = ValueKey::new(Value::Float(1.0));

    // NaN should be greater than any number for consistent ordering
    assert_eq!(nan.cmp(&number), Ordering::Greater);
    assert_eq!(number.cmp(&nan), Ordering::Less);
}

// =============================================================================
// Integer and Symbol Key Tests
// =============================================================================

#[test]
fn integer_keys() {
    let map = Map::from_pairs(vec![(int(1), string("one")), (int(2), string("two"))]);
    assert_eq!(map.get(&int(1)), Some(&string("one")));
    assert_eq!(map.get(&int(2)), Some(&string("two")));
}

#[test]
fn mixed_type_keys() {
    let map = Map::from_pairs(vec![
        (Value::Nil, int(0)),
        (Value::Bool(true), int(1)),
        (int(42), int(2)),
    ]);
    assert_eq!(map.get(&Value::Nil), Some(&int(0)));
    assert_eq!(map.get(&Value::Bool(true)), Some(&int(1)));
    assert_eq!(map.get(&int(42)), Some(&int(2)));
}

// =============================================================================
// Nested Map Tests
// =============================================================================

#[test]
fn nested_maps() {
    let inner = Map::from_pairs(vec![(string("x"), int(1))]);
    let outer = Map::from_pairs(vec![(string("inner"), Value::Map(inner.clone()))]);

    if let Some(Value::Map(inner_map)) = outer.get(&string("inner")) {
        assert_eq!(inner_map.get(&string("x")), Some(&int(1)));
    } else {
        panic!("Expected Map value");
    }
}

// =============================================================================
// Structural Sharing Tests
// =============================================================================

#[test]
fn structural_sharing_on_assoc() {
    let m1 = Map::from_pairs(vec![(string("a"), int(1))]);
    let m2 = m1.assoc(string("b"), int(2));

    // m1 unchanged
    assert_eq!(m1.len(), 1);
    assert_eq!(m1.get(&string("a")), Some(&int(1)));
    assert_eq!(m1.get(&string("b")), None);

    // m2 has both
    assert_eq!(m2.len(), 2);
    assert_eq!(m2.get(&string("a")), Some(&int(1)));
    assert_eq!(m2.get(&string("b")), Some(&int(2)));
}

#[test]
fn structural_sharing_on_dissoc() {
    let m1 = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
    let m2 = m1.dissoc(&string("a"));

    // m1 unchanged
    assert_eq!(m1.len(), 2);
    assert_eq!(m1.get(&string("a")), Some(&int(1)));

    // m2 has removal
    assert_eq!(m2.len(), 1);
    assert_eq!(m2.get(&string("a")), None);
    assert_eq!(m2.get(&string("b")), Some(&int(2)));
}

// =============================================================================
// Large Map Tests
// =============================================================================

#[test]
fn large_map_operations() {
    let mut map = Map::empty();
    let count = 100_i64;

    for i in 0..count {
        map = map.assoc(int(i), int(i.saturating_mul(10)));
    }

    assert_eq!(map.len(), count as usize);

    for i in 0..count {
        assert_eq!(map.get(&int(i)), Some(&int(i.saturating_mul(10))));
    }
}
