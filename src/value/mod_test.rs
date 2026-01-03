// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the Value type.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{HeapString, Pair, Value};
use crate::Vaddr;

#[test]
fn value_nil() {
    let v = Value::nil();
    assert!(v.is_nil());
    assert!(!v.is_truthy());
    assert!(v.is_list_head());
}

#[test]
fn value_bool() {
    let t = Value::bool(true);
    let f = Value::bool(false);

    assert!(!t.is_nil());
    assert!(t.is_truthy());

    assert!(!f.is_nil());
    assert!(!f.is_truthy());
}

#[test]
fn value_int() {
    let v = Value::int(42);
    assert!(!v.is_nil());
    assert!(v.is_truthy());

    let neg = Value::int(-123);
    assert!(neg.is_truthy());

    let zero = Value::int(0);
    assert!(zero.is_truthy()); // 0 is truthy in Lisps
}

#[test]
fn value_pair() {
    let addr = Vaddr::new(0x1000);
    let v = Value::pair(addr);
    assert!(v.is_pair());
    assert!(v.is_list_head());
    assert!(v.is_truthy());
}

#[test]
fn value_string() {
    let addr = Vaddr::new(0x2000);
    let v = Value::string(addr);
    assert!(!v.is_nil());
    assert!(v.is_truthy());
    assert!(!v.is_pair());
}

#[test]
fn value_symbol() {
    let addr = Vaddr::new(0x3000);
    let v = Value::symbol(addr);
    assert!(!v.is_nil());
    assert!(v.is_truthy());
}

#[test]
fn value_default_is_nil() {
    let v = Value::default();
    assert!(v.is_nil());
}

#[test]
fn value_equality() {
    assert_eq!(Value::nil(), Value::nil());
    assert_eq!(Value::bool(true), Value::bool(true));
    assert_eq!(Value::int(42), Value::int(42));
    assert_ne!(Value::int(1), Value::int(2));
    assert_ne!(Value::bool(true), Value::bool(false));
}

#[test]
fn heap_string_sizes() {
    assert_eq!(HeapString::HEADER_SIZE, 4);
    assert_eq!(HeapString::alloc_size(0), 4);
    assert_eq!(HeapString::alloc_size(5), 9);
    assert_eq!(HeapString::alloc_size(100), 104);
}

#[test]
fn pair_size() {
    // Value is at least 9 bytes (tag + i64), likely 16 with padding
    // Pair is 2 Values
    const _: () = assert!(Pair::SIZE >= 18);
}

#[test]
fn pair_construction() {
    let pair = Pair::new(Value::int(1), Value::nil());
    assert_eq!(pair.first, Value::int(1));
    assert_eq!(pair.rest, Value::nil());
}

#[test]
fn value_debug_format() {
    assert_eq!(format!("{:?}", Value::nil()), "Nil");
    assert_eq!(format!("{:?}", Value::bool(true)), "Bool(true)");
    assert_eq!(format!("{:?}", Value::int(42)), "Int(42)");
}
