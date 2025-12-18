// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the HAMT implementation.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::Hamt;

#[test]
fn new_creates_empty() {
    let hamt: Hamt<String, i32> = Hamt::new();
    assert!(hamt.is_empty());
    assert_eq!(hamt.len(), 0);
}

#[test]
fn insert_single() {
    let hamt: Hamt<String, i32> = Hamt::new();
    let hamt = hamt.insert(String::from("key"), 42);

    assert_eq!(hamt.len(), 1);
    assert_eq!(hamt.get("key"), Some(&42));
}

#[test]
fn insert_multiple() {
    let mut hamt: Hamt<String, i32> = Hamt::new();
    for i in 0..100 {
        let key = alloc::format!("key{i}");
        hamt = hamt.insert(key, i);
    }

    assert_eq!(hamt.len(), 100);
    for i in 0..100 {
        let key = alloc::format!("key{i}");
        assert_eq!(hamt.get(&key), Some(&i));
    }
}

#[test]
fn insert_replace() {
    let hamt: Hamt<String, i32> = Hamt::new();
    let hamt = hamt.insert(String::from("key"), 1);
    let hamt = hamt.insert(String::from("key"), 2);

    assert_eq!(hamt.len(), 1);
    assert_eq!(hamt.get("key"), Some(&2));
}

#[test]
fn get_missing() {
    let hamt: Hamt<String, i32> = Hamt::new();
    assert_eq!(hamt.get("missing"), None);
}

#[test]
fn contains_key() {
    let hamt: Hamt<String, i32> = Hamt::new();
    let hamt = hamt.insert(String::from("exists"), 1);

    assert!(hamt.contains_key("exists"));
    assert!(!hamt.contains_key("missing"));
}

#[test]
fn remove_existing() {
    let hamt: Hamt<String, i32> = Hamt::new();
    let hamt = hamt.insert(String::from("key"), 42);
    let hamt = hamt.remove("key");

    assert!(hamt.is_empty());
    assert_eq!(hamt.get("key"), None);
}

#[test]
fn remove_missing() {
    let hamt: Hamt<String, i32> = Hamt::new();
    let hamt = hamt.insert(String::from("key"), 42);
    let hamt = hamt.remove("other");

    assert_eq!(hamt.len(), 1);
    assert_eq!(hamt.get("key"), Some(&42));
}

#[test]
fn insert_preserves_original() {
    let h1: Hamt<String, i32> = Hamt::new();
    let h1 = h1.insert(String::from("a"), 1);
    let h2 = h1.insert(String::from("b"), 2);

    assert_eq!(h1.len(), 1);
    assert_eq!(h1.get("b"), None);
    assert_eq!(h2.len(), 2);
}

#[test]
fn iter_elements() {
    let mut hamt: Hamt<i32, i32> = Hamt::new();
    for i in 0..10 {
        hamt = hamt.insert(i, i.saturating_mul(10));
    }

    let mut collected: Vec<_> = hamt.iter().map(|(&key, &val)| (key, val)).collect();
    collected.sort();

    let expected: Vec<(i32, i32)> = (0..10_i32).map(|i| (i, i.saturating_mul(10))).collect();
    assert_eq!(collected, expected);
}

#[test]
fn keys_iterator() {
    let hamt: Hamt<String, i32> = Hamt::new();
    let hamt = hamt.insert(String::from("a"), 1);
    let hamt = hamt.insert(String::from("b"), 2);

    let mut keys: Vec<_> = hamt.keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, vec![String::from("a"), String::from("b")]);
}

#[test]
fn large_map() {
    let mut hamt: Hamt<i32, i32> = Hamt::new();
    let count = 1000;

    for i in 0..count {
        hamt = hamt.insert(i, i);
    }

    assert_eq!(hamt.len(), 1000);

    for i in 0..count {
        assert_eq!(hamt.get(&i), Some(&i));
    }
}

// =============================================================================
// Collision Tests
//
// These tests use a custom key type that allows us to control the hash
// value, enabling direct testing of collision handling.
// =============================================================================

/// A key type with a controllable hash value for testing collisions.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CollisionKey {
    id: i32,
    forced_hash: u64,
}

impl CollisionKey {
    fn new(id: i32, hash: u64) -> Self {
        Self {
            id,
            forced_hash: hash,
        }
    }
}

impl core::hash::Hash for CollisionKey {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // Write the forced hash value directly
        state.write_u64(self.forced_hash);
    }
}

#[test]
fn collision_two_keys_same_hash() {
    // Two different keys with the same hash should both be stored
    let key1 = CollisionKey::new(1, 0xDEAD_BEEF);
    let key2 = CollisionKey::new(2, 0xDEAD_BEEF);

    let hamt: Hamt<CollisionKey, &str> = Hamt::new();
    let hamt = hamt.insert(key1.clone(), "first");
    let hamt = hamt.insert(key2.clone(), "second");

    assert_eq!(hamt.len(), 2);
    assert_eq!(hamt.get(&key1), Some(&"first"));
    assert_eq!(hamt.get(&key2), Some(&"second"));
}

#[test]
fn collision_update_existing_in_collision_bucket() {
    // Updating a key in a collision bucket should work correctly
    let key1 = CollisionKey::new(1, 0xCAFE_BABE);
    let key2 = CollisionKey::new(2, 0xCAFE_BABE);

    let hamt: Hamt<CollisionKey, &str> = Hamt::new();
    let hamt = hamt.insert(key1.clone(), "first");
    let hamt = hamt.insert(key2.clone(), "second");
    let hamt = hamt.insert(key1.clone(), "updated");

    assert_eq!(hamt.len(), 2);
    assert_eq!(hamt.get(&key1), Some(&"updated"));
    assert_eq!(hamt.get(&key2), Some(&"second"));
}

#[test]
fn collision_remove_from_collision_bucket() {
    // Removing from a collision bucket should leave the other entry
    let key1 = CollisionKey::new(1, 0x1234_5678);
    let key2 = CollisionKey::new(2, 0x1234_5678);

    let hamt: Hamt<CollisionKey, &str> = Hamt::new();
    let hamt = hamt.insert(key1.clone(), "first");
    let hamt = hamt.insert(key2.clone(), "second");
    let hamt = hamt.remove(&key1);

    assert_eq!(hamt.len(), 1);
    assert_eq!(hamt.get(&key1), None);
    assert_eq!(hamt.get(&key2), Some(&"second"));
}

#[test]
fn collision_remove_last_from_collision_converts_to_leaf() {
    // Removing the second-to-last from collision should convert to leaf
    let key1 = CollisionKey::new(1, 0xAAAA_BBBB);
    let key2 = CollisionKey::new(2, 0xAAAA_BBBB);

    let hamt: Hamt<CollisionKey, &str> = Hamt::new();
    let hamt = hamt.insert(key1.clone(), "first");
    let hamt = hamt.insert(key2.clone(), "second");
    let hamt = hamt.remove(&key1);
    let hamt = hamt.remove(&key2);

    assert!(hamt.is_empty());
}

#[test]
fn collision_three_keys_same_hash() {
    // Three different keys with the same hash
    let key1 = CollisionKey::new(1, 0x9999_9999);
    let key2 = CollisionKey::new(2, 0x9999_9999);
    let key3 = CollisionKey::new(3, 0x9999_9999);

    let hamt: Hamt<CollisionKey, i32> = Hamt::new();
    let hamt = hamt.insert(key1.clone(), 100);
    let hamt = hamt.insert(key2.clone(), 200);
    let hamt = hamt.insert(key3.clone(), 300);

    assert_eq!(hamt.len(), 3);
    assert_eq!(hamt.get(&key1), Some(&100));
    assert_eq!(hamt.get(&key2), Some(&200));
    assert_eq!(hamt.get(&key3), Some(&300));
}

#[test]
fn collision_iterate_collision_bucket() {
    // Iteration should yield all entries from collision buckets
    let key1 = CollisionKey::new(1, 0xFFFF_0000);
    let key2 = CollisionKey::new(2, 0xFFFF_0000);

    let hamt: Hamt<CollisionKey, &str> = Hamt::new();
    let hamt = hamt.insert(key1.clone(), "first");
    let hamt = hamt.insert(key2.clone(), "second");

    let entries: Vec<_> = hamt.iter().collect();
    assert_eq!(entries.len(), 2);
}

#[test]
fn collision_preserves_original_on_insert() {
    // Structural sharing should work correctly with collisions
    let key1 = CollisionKey::new(1, 0xBEEF_CAFE);
    let key2 = CollisionKey::new(2, 0xBEEF_CAFE);

    let h1: Hamt<CollisionKey, &str> = Hamt::new();
    let h1 = h1.insert(key1.clone(), "first");
    let h2 = h1.insert(key2.clone(), "second");

    // h1 should still have only one entry
    assert_eq!(h1.len(), 1);
    assert_eq!(h1.get(&key2), None);

    // h2 should have both
    assert_eq!(h2.len(), 2);
    assert_eq!(h2.get(&key1), Some(&"first"));
    assert_eq!(h2.get(&key2), Some(&"second"));
}
