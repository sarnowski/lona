// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the `vector` primitive.

use alloc::vec::Vec;

use lona_core::symbol::Interner;
use lona_core::value::Value;

use super::{ctx, int, string};
use crate::vm::collections::native_vector;

#[test]
fn vector_empty() {
    let interner = Interner::new();
    let args: Vec<Value> = alloc::vec![];

    let result = native_vector(&args, &ctx(&interner)).unwrap();

    if let Value::Vector(vec) = result {
        assert!(vec.is_empty());
    } else {
        panic!("Expected Vector");
    }
}

#[test]
fn vector_with_elements() {
    let interner = Interner::new();
    let args = alloc::vec![int(1), int(2), int(3)];

    let result = native_vector(&args, &ctx(&interner)).unwrap();

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
fn vector_preserves_order() {
    let interner = Interner::new();
    let args = alloc::vec![string("a"), string("b"), string("c")];

    let result = native_vector(&args, &ctx(&interner)).unwrap();

    if let Value::Vector(vec) = result {
        assert_eq!(vec.get(0_usize), Some(&string("a")));
        assert_eq!(vec.get(1_usize), Some(&string("b")));
        assert_eq!(vec.get(2_usize), Some(&string("c")));
    } else {
        panic!("Expected Vector");
    }
}
