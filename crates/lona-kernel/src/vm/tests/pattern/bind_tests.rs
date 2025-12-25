// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Bind pattern tests.

use alloc::vec;

use lona_core::ratio::Ratio;
use lona_core::value::Value;
use lona_core::vector::Vector;
use lona_core::{integer::Integer, string::HeapStr, symbol::Interner};

use super::make_symbol;
use crate::vm::pattern::{Pattern, try_match};

#[test]
fn bind_captures_nil() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");

    let result = try_match(&Pattern::Bind(x), &Value::Nil);
    assert_eq!(result, Some(vec![(x, Value::Nil)]));
}

#[test]
fn bind_captures_integer() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let value = Value::Integer(Integer::from(42));

    let result = try_match(&Pattern::Bind(x), &value);
    assert_eq!(result, Some(vec![(x, value)]));
}

#[test]
fn bind_captures_string() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let value = Value::String(HeapStr::from("hello"));

    let result = try_match(&Pattern::Bind(x), &value);
    assert_eq!(result, Some(vec![(x, value)]));
}

#[test]
fn bind_captures_bool() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let value = Value::Bool(true);

    let result = try_match(&Pattern::Bind(x), &value);
    assert_eq!(result, Some(vec![(x, value)]));
}

#[test]
fn bind_captures_float() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let value = Value::Float(3.14);

    let result = try_match(&Pattern::Bind(x), &value);
    assert_eq!(result, Some(vec![(x, value)]));
}

#[test]
fn bind_captures_ratio() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let numer = Integer::from(1);
    let denom = Integer::from(2);
    let ratio = Ratio::new(&numer, &denom);
    let value = Value::Ratio(ratio);

    let result = try_match(&Pattern::Bind(x), &value);
    assert_eq!(result, Some(vec![(x, value)]));
}

#[test]
fn bind_captures_vector() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
    ]);
    let value = Value::Vector(vec);

    let result = try_match(&Pattern::Bind(x), &value);
    assert_eq!(result, Some(vec![(x, value)]));
}
