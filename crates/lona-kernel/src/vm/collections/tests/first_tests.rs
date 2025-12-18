// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the `first` primitive.

use lona_core::list::List;
use lona_core::map::Map;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_core::vector::Vector;

use super::{ctx, int, string};
use crate::vm::collections::native_first;
use crate::vm::natives::NativeError;

#[test]
fn first_of_list() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
    let args = alloc::vec![Value::List(list)];

    let result = native_first(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, int(1));
}

#[test]
fn first_of_empty_list() {
    let interner = Interner::new();
    let list = List::empty();
    let args = alloc::vec![Value::List(list)];

    let result = native_first(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn first_of_vector() {
    let interner = Interner::new();
    let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
    let args = alloc::vec![Value::Vector(vec)];

    let result = native_first(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, int(1));
}

#[test]
fn first_of_empty_vector() {
    let interner = Interner::new();
    let vec = Vector::empty();
    let args = alloc::vec![Value::Vector(vec)];

    let result = native_first(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn first_of_map() {
    let interner = Interner::new();
    let map = Map::from_pairs(alloc::vec![(string("a"), int(1))]);
    let args = alloc::vec![Value::Map(map)];

    let result = native_first(&args, &ctx(&interner)).unwrap();

    // Should be a vector [key value]
    if let Value::Vector(vec) = result {
        assert_eq!(vec.len(), 2);
    } else {
        panic!("Expected Vector");
    }
}

#[test]
fn first_of_empty_map() {
    let interner = Interner::new();
    let map = Map::empty();
    let args = alloc::vec![Value::Map(map)];

    let result = native_first(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn first_of_nil() {
    let interner = Interner::new();
    let args = alloc::vec![Value::Nil];

    let result = native_first(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn first_type_error() {
    let interner = Interner::new();
    let args = alloc::vec![int(42)];

    let result = native_first(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::TypeError {
            expected: "list, vector, map, or nil",
            got: "integer",
            arg_index: 0
        })
    ));
}

#[test]
fn first_arity_error() {
    let interner = Interner::new();
    let args = alloc::vec![];

    let result = native_first(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::ArityMismatch {
            expected: 1,
            got: 0
        })
    ));
}
