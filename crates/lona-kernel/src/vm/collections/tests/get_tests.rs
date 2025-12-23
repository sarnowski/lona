// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the `get` primitive.

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::map::Map;
use lona_core::symbol::Interner;
use lona_core::value::{self, Value};

use super::{ctx, int, string};
use crate::vm::collections::native_get;
use crate::vm::natives::NativeError;

#[test]
fn get_existing_key() {
    let interner = Interner::new();
    let map = Map::from_pairs(alloc::vec![(string("a"), int(1)), (string("b"), int(2))]);
    let args = alloc::vec![Value::Map(map), string("a")];

    let result = native_get(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, int(1));
}

#[test]
fn get_missing_key_returns_nil() {
    let interner = Interner::new();
    let map = Map::from_pairs(alloc::vec![(string("a"), int(1))]);
    let args = alloc::vec![Value::Map(map), string("b")];

    let result = native_get(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn get_missing_key_with_default() {
    let interner = Interner::new();
    let map = Map::from_pairs(alloc::vec![(string("a"), int(1))]);
    let args = alloc::vec![Value::Map(map), string("b"), string("default")];

    let result = native_get(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, string("default"));
}

#[test]
fn get_from_nil_map_returns_nil() {
    let interner = Interner::new();
    let args = alloc::vec![Value::Nil, string("a")];

    let result = native_get(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn get_from_nil_map_with_default() {
    let interner = Interner::new();
    let args = alloc::vec![Value::Nil, string("a"), string("default")];

    let result = native_get(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, string("default"));
}

#[test]
fn get_from_empty_map() {
    let interner = Interner::new();
    let map = Map::empty();
    let args = alloc::vec![Value::Map(map), string("a")];

    let result = native_get(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn get_with_integer_key() {
    let interner = Interner::new();
    let map = Map::from_pairs(alloc::vec![(int(42), string("answer"))]);
    let args = alloc::vec![Value::Map(map), int(42)];

    let result = native_get(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, string("answer"));
}

#[test]
fn get_with_nil_key() {
    let interner = Interner::new();
    let map = Map::from_pairs(alloc::vec![(Value::Nil, int(100))]);
    let args = alloc::vec![Value::Map(map), Value::Nil];

    let result = native_get(&args, &ctx(&interner)).unwrap();
    assert_eq!(result, int(100));
}

#[test]
fn get_arity_error_too_few() {
    let interner = Interner::new();
    let args = alloc::vec![Value::Map(Map::empty())];

    let result = native_get(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Range {
                min: 2_u8,
                max: 3_u8
            },
            got: 1_u8
        })
    ));
}

#[test]
fn get_arity_error_too_many() {
    let interner = Interner::new();
    let args = alloc::vec![Value::Map(Map::empty()), string("a"), int(1), int(2)];

    let result = native_get(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Range {
                min: 2_u8,
                max: 3_u8
            },
            got: 4_u8
        })
    ));
}

#[test]
fn get_type_error_first_arg_not_map() {
    let interner = Interner::new();
    let args = alloc::vec![int(42), string("a")];

    let result = native_get(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::TypeError {
            expected: TypeExpectation::Single(value::Kind::Map),
            got: value::Kind::Integer,
            arg_index: 0_u8
        })
    ));
}
