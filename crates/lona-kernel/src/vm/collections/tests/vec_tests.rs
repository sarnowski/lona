// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the `vec` primitive.

use alloc::vec::Vec;

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::list::List;
use lona_core::map::Map;
use lona_core::symbol::Interner;
use lona_core::value::{self, Value};
use lona_core::vector::Vector;

use super::{ctx, int, string};
use crate::vm::collections::native_vec;
use crate::vm::natives::NativeError;

#[test]
fn vec_from_nil() {
    let interner = Interner::new();
    let args = alloc::vec![Value::Nil];

    let result = native_vec(&args, &ctx(&interner)).unwrap();

    if let Value::Vector(vec) = result {
        assert!(vec.is_empty());
    } else {
        panic!("Expected Vector");
    }
}

#[test]
fn vec_from_list() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
    let args = alloc::vec![Value::List(list)];

    let result = native_vec(&args, &ctx(&interner)).unwrap();

    if let Value::Vector(vec) = result {
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.get(0_usize), Some(&int(1)));
        assert_eq!(vec.get(1_usize), Some(&int(2)));
        assert_eq!(vec.get(2_usize), Some(&int(3)));
    } else {
        panic!("Expected Vector");
    }
}

#[test]
fn vec_from_vector() {
    let interner = Interner::new();
    let original = Vector::from_vec(alloc::vec![int(1), int(2)]);
    let args = alloc::vec![Value::Vector(original.clone())];

    let result = native_vec(&args, &ctx(&interner)).unwrap();

    if let Value::Vector(vec) = result {
        assert_eq!(vec.len(), 2);
        assert_eq!(vec, original);
    } else {
        panic!("Expected Vector");
    }
}

#[test]
fn vec_from_map() {
    let interner = Interner::new();
    let map = Map::from_pairs(alloc::vec![(string("a"), int(1)), (string("b"), int(2))]);
    let args = alloc::vec![Value::Map(map)];

    let result = native_vec(&args, &ctx(&interner)).unwrap();

    if let Value::Vector(vec) = result {
        // Map entries become [key value] vectors
        assert_eq!(vec.len(), 2);
        // Each element should be a vector
        if let Some(Value::Vector(entry)) = vec.get(0_usize) {
            assert_eq!(entry.len(), 2);
        } else {
            panic!("Expected Vector entry");
        }
    } else {
        panic!("Expected Vector");
    }
}

#[test]
fn vec_type_error() {
    let interner = Interner::new();
    let args = alloc::vec![int(42)];

    let result = native_vec(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::TypeError {
            expected: TypeExpectation::Sequence,
            got: value::Kind::Integer,
            arg_index: 0_u8
        })
    ));
}

#[test]
fn vec_arity_error_too_few() {
    let interner = Interner::new();
    let args: Vec<Value> = alloc::vec![];

    let result = native_vec(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: 0_u8
        })
    ));
}

#[test]
fn vec_arity_error_too_many() {
    let interner = Interner::new();
    let args = alloc::vec![Value::Nil, Value::Nil];

    let result = native_vec(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: 2_u8
        })
    ));
}
