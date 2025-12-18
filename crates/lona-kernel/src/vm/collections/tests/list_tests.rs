// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the `list` primitive.

use alloc::vec::Vec;

use lona_core::list::List;
use lona_core::symbol::Interner;
use lona_core::value::Value;

use super::{ctx, int};
use crate::vm::collections::native_list;

#[test]
fn list_empty() {
    let interner = Interner::new();
    let args: Vec<Value> = alloc::vec![];

    let result = native_list(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert!(list.is_empty());
    } else {
        panic!("Expected List");
    }
}

#[test]
fn list_single_element() {
    let interner = Interner::new();
    let args = alloc::vec![int(42)];

    let result = native_list(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert_eq!(list.len(), 1);
        assert_eq!(list.first(), Some(&int(42)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn list_multiple_elements() {
    let interner = Interner::new();
    let args = alloc::vec![int(1), int(2), int(3)];

    let result = native_list(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.first(), Some(&int(1)));
    } else {
        panic!("Expected List");
    }
}

#[test]
fn list_nested() {
    let interner = Interner::new();
    let inner = List::from_vec(alloc::vec![int(1), int(2)]);
    let args = alloc::vec![Value::List(inner), int(3)];

    let result = native_list(&args, &ctx(&interner)).unwrap();

    if let Value::List(list) = result {
        assert_eq!(list.len(), 2);
    } else {
        panic!("Expected List");
    }
}
