// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the `hash-map` primitive.

use alloc::vec::Vec;

use lona_core::symbol::Interner;
use lona_core::value::Value;

use super::{ctx, int, string};
use crate::vm::collections::native_hash_map;
use crate::vm::natives::NativeError;

#[test]
fn hash_map_empty() {
    let interner = Interner::new();
    let args: Vec<Value> = alloc::vec![];

    let result = native_hash_map(&args, &ctx(&interner)).unwrap();

    if let Value::Map(map) = result {
        assert!(map.is_empty());
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn hash_map_with_pairs() {
    let interner = Interner::new();
    let args = alloc::vec![string("a"), int(1), string("b"), int(2)];

    let result = native_hash_map(&args, &ctx(&interner)).unwrap();

    if let Value::Map(map) = result {
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&string("a")), Some(&int(1)));
        assert_eq!(map.get(&string("b")), Some(&int(2)));
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn hash_map_odd_args_error() {
    let interner = Interner::new();
    let args = alloc::vec![string("a"), int(1), string("b")];

    let result = native_hash_map(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::Error(
            "hash-map requires even number of arguments"
        ))
    ));
}

#[test]
fn hash_map_duplicate_keys() {
    let interner = Interner::new();
    // Later value wins
    let args = alloc::vec![string("a"), int(1), string("a"), int(2)];

    let result = native_hash_map(&args, &ctx(&interner)).unwrap();

    if let Value::Map(map) = result {
        // Only one entry since key is duplicated
        assert_eq!(map.len(), 1);
        // Later value should win
        assert_eq!(map.get(&string("a")), Some(&int(2)));
    } else {
        panic!("Expected Map");
    }
}

#[test]
fn hash_map_mixed_key_types() {
    let interner = Interner::new();
    let args = alloc::vec![string("str"), int(1), int(42), int(2), Value::Nil, int(3)];

    let result = native_hash_map(&args, &ctx(&interner)).unwrap();

    if let Value::Map(map) = result {
        assert_eq!(map.len(), 3);
        assert_eq!(map.get(&string("str")), Some(&int(1)));
        assert_eq!(map.get(&int(42)), Some(&int(2)));
        assert_eq!(map.get(&Value::Nil), Some(&int(3)));
    } else {
        panic!("Expected Map");
    }
}
