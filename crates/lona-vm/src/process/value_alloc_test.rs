// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for value allocation (strings, pairs, symbols, etc.).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::allocation_test::setup;
use crate::value::Value;

#[test]
fn alloc_string() {
    let (mut proc, mut mem) = setup();

    let value = proc.alloc_string(&mut mem, "hello").unwrap();
    assert!(matches!(value, Value::String(_)));

    let s = proc.read_string(&mem, value).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn alloc_string_empty() {
    let (mut proc, mut mem) = setup();

    let value = proc.alloc_string(&mut mem, "").unwrap();
    let s = proc.read_string(&mem, value).unwrap();
    assert_eq!(s, "");
}

#[test]
fn alloc_string_unicode() {
    let (mut proc, mut mem) = setup();

    let value = proc.alloc_string(&mut mem, "你好世界").unwrap();
    let s = proc.read_string(&mem, value).unwrap();
    assert_eq!(s, "你好世界");
}

#[test]
fn alloc_pair() {
    let (mut proc, mut mem) = setup();

    let first = Value::int(1);
    let rest = Value::int(2);
    let value = proc.alloc_pair(&mut mem, first, rest).unwrap();

    assert!(matches!(value, Value::Pair(_)));

    let pair = proc.read_pair(&mem, value).unwrap();
    assert_eq!(pair.first, Value::int(1));
    assert_eq!(pair.rest, Value::int(2));
}

#[test]
fn alloc_list() {
    let (mut proc, mut mem) = setup();

    // Build list (1 2 3)
    let v3 = proc
        .alloc_pair(&mut mem, Value::int(3), Value::Nil)
        .unwrap();
    let v2 = proc.alloc_pair(&mut mem, Value::int(2), v3).unwrap();
    let v1 = proc.alloc_pair(&mut mem, Value::int(1), v2).unwrap();

    // Read back
    let p1 = proc.read_pair(&mem, v1).unwrap();
    assert_eq!(p1.first, Value::int(1));

    let p2 = proc.read_pair(&mem, p1.rest).unwrap();
    assert_eq!(p2.first, Value::int(2));

    let p3 = proc.read_pair(&mem, p2.rest).unwrap();
    assert_eq!(p3.first, Value::int(3));
    assert_eq!(p3.rest, Value::Nil);
}

#[test]
fn alloc_symbol() {
    let (mut proc, mut mem) = setup();

    let value = proc.alloc_symbol(&mut mem, "foo").unwrap();
    assert!(matches!(value, Value::Symbol(_)));

    let name = proc.read_string(&mem, value).unwrap();
    assert_eq!(name, "foo");
}

#[test]
fn symbol_interning() {
    let (mut proc, mut mem) = setup();

    // Same symbol name should return the same address (interned)
    let sym1 = proc.alloc_symbol(&mut mem, "test").unwrap();
    let sym2 = proc.alloc_symbol(&mut mem, "test").unwrap();
    let sym3 = proc.alloc_symbol(&mut mem, "other").unwrap();

    // Same name → same address
    let Value::Symbol(addr1) = sym1 else { panic!() };
    let Value::Symbol(addr2) = sym2 else { panic!() };
    let Value::Symbol(addr3) = sym3 else { panic!() };

    assert_eq!(
        addr1, addr2,
        "Same symbol should be interned to same address"
    );
    assert_ne!(
        addr1, addr3,
        "Different symbols should have different addresses"
    );
}
