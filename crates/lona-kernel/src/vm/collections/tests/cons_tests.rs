// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the `cons` primitive.

use lona_core::list::List;
use lona_core::map::Map;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_core::vector::Vector;

use super::{ctx, int, string};
use crate::vm::collections::native_cons;
use crate::vm::natives::NativeError;

#[test]
fn cons_to_list() {
    let interner = Interner::new();
    let list = List::from_vec(alloc::vec![int(2), int(3)]);
    let args = alloc::vec![int(1), Value::List(list)];

    let result = native_cons(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.first(), Some(&int(1)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn cons_to_vector() {
    let interner = Interner::new();
    let vec = Vector::from_vec(alloc::vec![int(2), int(3)]);
    let args = alloc::vec![int(1), Value::Vector(vec)];

    let result = native_cons(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.first(), Some(&int(1)));
        // Verify rest is (2 3)
        let rest = list.rest();
        assert_eq!(rest.first(), Some(&int(2)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn cons_to_nil() {
    let interner = Interner::new();
    let args = alloc::vec![int(1), Value::Nil];

    let result = native_cons(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert_eq!(list.len(), 1);
        assert_eq!(list.first(), Some(&int(1)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn cons_type_error() {
    let interner = Interner::new();
    let args = alloc::vec![int(1), int(2)];

    let result = native_cons(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::TypeError {
            expected: "list, vector, or nil",
            got: "integer",
            arg_index: 1
        })
    ));
}

#[test]
fn cons_type_error_map() {
    let interner = Interner::new();
    // cons with a map should produce type error (unlike first/rest which support maps)
    let map = Map::from_pairs(alloc::vec![(string("a"), int(1))]);
    let args = alloc::vec![int(1), Value::Map(map)];

    let result = native_cons(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::TypeError {
            expected: "list, vector, or nil",
            got: "map",
            arg_index: 1
        })
    ));
}

#[test]
fn cons_arity_error() {
    let interner = Interner::new();
    let args = alloc::vec![int(1)];

    let result = native_cons(&args, &ctx(&interner));

    assert!(matches!(
        result,
        Err(NativeError::ArityMismatch {
            expected: 2,
            got: 1
        })
    ));
}
