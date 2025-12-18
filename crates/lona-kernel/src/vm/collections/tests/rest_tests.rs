// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the `rest` primitive.

use lona_core::list::List;
use lona_core::map::Map;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_core::vector::Vector;

use super::{ctx, int, string};
use crate::vm::collections::native_rest;
use crate::vm::natives::NativeError;

#[test]
fn rest_of_list() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
    let args = alloc::vec![Value::List(list)];

    let result = native_rest(&args, &ctx(&interner)).unwrap();

    if let Value::List(rest_list) = result {
        assert_eq!(rest_list.len(), 2);
        assert_eq!(rest_list.first(), Some(&int(2)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn rest_of_single_element_list() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(1)]);
    let args = alloc::vec![Value::List(list)];

    let result = native_rest(&args, &ctx(&interner)).unwrap();

    if let Value::List(rest_list) = result {
        assert!(rest_list.is_empty());
    } else {
        panic!("Expected List");
    }
}

#[test]
fn rest_of_empty_list() {
    let interner = Interner::new();
    let list = List::empty();
    let args = alloc::vec![Value::List(list)];

    let result = native_rest(&args, &ctx(&interner)).unwrap();

    if let Value::List(rest_list) = result {
        assert!(rest_list.is_empty());
    } else {
        panic!("Expected List");
    }
}

#[test]
fn rest_of_vector() {
    let interner = Interner::new();
    let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
    let args = alloc::vec![Value::Vector(vec)];

    let result = native_rest(&args, &ctx(&interner)).unwrap();

    if let Value::List(rest_list) = result {
        assert_eq!(rest_list.len(), 2);
        assert_eq!(rest_list.first(), Some(&int(2)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn rest_of_empty_vector() {
    let interner = Interner::new();
    let vec = Vector::empty();
    let args = alloc::vec![Value::Vector(vec)];

    let result = native_rest(&args, &ctx(&interner)).unwrap();

    if let Value::List(rest_list) = result {
        assert!(rest_list.is_empty());
    } else {
        panic!("Expected List");
    }
}

#[test]
fn rest_of_nil() {
    let interner = Interner::new();
    let args = alloc::vec![Value::Nil];

    let result = native_rest(&args, &ctx(&interner)).unwrap();

    if let Value::List(rest_list) = result {
        assert!(rest_list.is_empty());
    } else {
        panic!("Expected List");
    }
}

#[test]
fn rest_of_map() {
    let interner = Interner::new();
    // Map with two entries
    let map = Map::from_pairs(alloc::vec![(string("a"), int(1)), (string("b"), int(2)),]);
    let args = alloc::vec![Value::Map(map)];

    let result = native_rest(&args, &ctx(&interner)).unwrap();

    // Should return a list with one [key value] vector (the second entry)
    if let Value::List(rest_list) = result {
        assert_eq!(rest_list.len(), 1);
        // The single entry should be a vector
        if let Some(Value::Vector(entry)) = rest_list.first() {
            assert_eq!(entry.len(), 2);
        } else {
            panic!("Expected Vector entry");
        }
    } else {
        panic!("Expected List");
    }
}

#[test]
fn rest_arity_error() {
    let interner = Interner::new();
    let args = alloc::vec![];

    let result = native_rest(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::ArityMismatch {
            expected: 1,
            got: 0
        })
    ));
}

#[test]
fn rest_type_error() {
    let interner = Interner::new();
    let args = alloc::vec![int(42)];

    let result = native_rest(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::TypeError {
            expected: "list, vector, map, or nil",
            got: "integer",
            arg_index: 0
        })
    ));
}
