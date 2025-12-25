// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Wildcard pattern tests.

use alloc::vec::Vec;

use lona_core::list::List;
use lona_core::value::Value;
use lona_core::vector::Vector;
use lona_core::{integer::Integer, string::HeapStr, symbol::Interner};

use crate::vm::pattern::{Pattern, try_match};

#[test]
fn wildcard_matches_nil() {
    let result = try_match(&Pattern::Wildcard, &Value::Nil);
    assert_eq!(result, Some(Vec::new()));
}

#[test]
fn wildcard_matches_bool() {
    assert!(try_match(&Pattern::Wildcard, &Value::Bool(true)).is_some());
    assert!(try_match(&Pattern::Wildcard, &Value::Bool(false)).is_some());
}

#[test]
fn wildcard_matches_integer() {
    let value = Value::Integer(Integer::from(42));
    assert_eq!(try_match(&Pattern::Wildcard, &value), Some(Vec::new()));
}

#[test]
fn wildcard_matches_float() {
    let value = Value::Float(3.14);
    assert_eq!(try_match(&Pattern::Wildcard, &value), Some(Vec::new()));
}

#[test]
fn wildcard_matches_string() {
    let value = Value::String(HeapStr::from("hello"));
    assert_eq!(try_match(&Pattern::Wildcard, &value), Some(Vec::new()));
}

#[test]
fn wildcard_matches_keyword() {
    let mut interner = Interner::new();
    let kw_id = interner.intern("foo");
    let value = Value::Keyword(kw_id);
    assert_eq!(try_match(&Pattern::Wildcard, &value), Some(Vec::new()));
}

#[test]
fn wildcard_matches_vector() {
    let vec = Vector::from_vec(alloc::vec![Value::Integer(Integer::from(1))]);
    let value = Value::Vector(vec);
    assert_eq!(try_match(&Pattern::Wildcard, &value), Some(Vec::new()));
}

#[test]
fn wildcard_matches_list() {
    let list = List::empty().cons(Value::Integer(Integer::from(1)));
    let value = Value::List(list);
    assert_eq!(try_match(&Pattern::Wildcard, &value), Some(Vec::new()));
}
