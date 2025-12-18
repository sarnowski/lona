// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the `concat` primitive.

use alloc::vec::Vec;

use lona_core::error_context::TypeExpectation;
use lona_core::list::List;
use lona_core::map::Map;
use lona_core::symbol::Interner;
use lona_core::value::{self, Value};
use lona_core::vector::Vector;

use super::{ctx, int, string};
use crate::vm::collections::native_concat;
use crate::vm::natives::NativeError;

#[test]
fn concat_empty() {
    let interner = Interner::new();
    let args: Vec<Value> = alloc::vec![];

    let result = native_concat(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert!(list.is_empty());
    } else {
        panic!("Expected List");
    }
}

#[test]
fn concat_single_list() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(1), int(2)]);
    let args = alloc::vec![Value::List(list)];

    let result = native_concat(&args, &ctx(&interner)).unwrap();

    if let Value::List(result_list) = result {
        assert_eq!(result_list.len(), 2);
        assert_eq!(result_list.first(), Some(&int(1)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn concat_single_vector() {
    let interner = Interner::new();
    let vec = Vector::from_vec(alloc::vec![int(1), int(2)]);
    let args = alloc::vec![Value::Vector(vec)];

    let result = native_concat(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert_eq!(list.len(), 2);
        assert_eq!(list.first(), Some(&int(1)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn concat_multiple_lists() {
    let interner = Interner::new();
    let list1 = List::from_vec(alloc::vec![int(1), int(2)]);
    let list2 = List::from_vec(alloc::vec![int(3), int(4)]);
    let args = alloc::vec![Value::List(list1), Value::List(list2)];

    let result = native_concat(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert_eq!(list.len(), 4);
        assert_eq!(list.first(), Some(&int(1)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn concat_mixed_types() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(1), int(2)]);
    let vec = Vector::from_vec(alloc::vec![int(3), int(4)]);
    let args = alloc::vec![Value::List(list), Value::Vector(vec)];

    let result = native_concat(&args, &ctx(&interner)).unwrap();

    if let Value::List(result_list) = result {
        assert_eq!(result_list.len(), 4);
    } else {
        panic!("Expected List");
    }
}

#[test]
fn concat_with_nil() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(1), int(2)]);
    let args = alloc::vec![Value::Nil, Value::List(list), Value::Nil];

    let result = native_concat(&args, &ctx(&interner)).unwrap();

    if let Value::List(result_list) = result {
        assert_eq!(result_list.len(), 2);
        assert_eq!(result_list.first(), Some(&int(1)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn concat_with_map() {
    let interner = Interner::new();
    let map = Map::from_pairs(alloc::vec![(string("a"), int(1)), (string("b"), int(2))]);
    let args = alloc::vec![Value::Map(map)];

    let result = native_concat(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        // Map entries become [key value] vectors
        assert_eq!(list.len(), 2);
        // Each element should be a vector
        if let Some(Value::Vector(entry)) = list.first() {
            assert_eq!(entry.len(), 2);
        } else {
            panic!("Expected Vector entry");
        }
    } else {
        panic!("Expected List");
    }
}

#[test]
fn concat_map_with_list() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(1), int(2)]);
    let map = Map::from_pairs(alloc::vec![(string("a"), int(3))]);
    let args = alloc::vec![Value::List(list), Value::Map(map)];

    let result = native_concat(&args, &ctx(&interner)).unwrap();

    if let Value::List(result_list) = result {
        // (1 2 ["a" 3])
        assert_eq!(result_list.len(), 3);
        assert_eq!(result_list.first(), Some(&int(1)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn concat_type_error() {
    let interner = Interner::new();
    let args = alloc::vec![int(42)];

    let result = native_concat(&args, &ctx(&interner));

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
fn concat_type_error_second_arg() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(1)]);
    let args = alloc::vec![Value::List(list), int(42)];

    let result = native_concat(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::TypeError {
            expected: TypeExpectation::Sequence,
            got: value::Kind::Integer,
            arg_index: 1_u8
        })
    ));
}
