// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Literal pattern tests.

use alloc::vec::Vec;

use lona_core::ratio::Ratio;
use lona_core::value::Symbol;
use lona_core::value::Value;
use lona_core::{integer::Integer, string::HeapStr, symbol::Interner};

use crate::vm::pattern::{Pattern, try_match};

#[test]
fn literal_matches_nil() {
    let pattern = Pattern::Literal(Value::Nil);
    assert_eq!(try_match(&pattern, &Value::Nil), Some(Vec::new()));
}

#[test]
fn literal_matches_true() {
    let pattern = Pattern::Literal(Value::Bool(true));
    assert_eq!(try_match(&pattern, &Value::Bool(true)), Some(Vec::new()));
}

#[test]
fn literal_matches_false() {
    let pattern = Pattern::Literal(Value::Bool(false));
    assert_eq!(try_match(&pattern, &Value::Bool(false)), Some(Vec::new()));
}

#[test]
fn literal_rejects_wrong_bool() {
    let pattern = Pattern::Literal(Value::Bool(true));
    assert_eq!(try_match(&pattern, &Value::Bool(false)), None);
}

#[test]
fn literal_matches_integer() {
    let pattern = Pattern::Literal(Value::Integer(Integer::from(42)));
    let value = Value::Integer(Integer::from(42));
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn literal_rejects_wrong_integer() {
    let pattern = Pattern::Literal(Value::Integer(Integer::from(42)));
    let value = Value::Integer(Integer::from(43));
    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn literal_matches_float() {
    let pattern = Pattern::Literal(Value::Float(3.14));
    let value = Value::Float(3.14);
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn literal_rejects_wrong_float() {
    let pattern = Pattern::Literal(Value::Float(3.14));
    let value = Value::Float(2.71);
    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn literal_matches_string() {
    let pattern = Pattern::Literal(Value::String(HeapStr::from("hello")));
    let value = Value::String(HeapStr::from("hello"));
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn literal_rejects_wrong_string() {
    let pattern = Pattern::Literal(Value::String(HeapStr::from("hello")));
    let value = Value::String(HeapStr::from("world"));
    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn literal_matches_keyword() {
    let mut interner = Interner::new();
    let kw_id = interner.intern("foo");
    let pattern = Pattern::Literal(Value::Keyword(kw_id));
    let value = Value::Keyword(kw_id);
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn literal_rejects_wrong_keyword() {
    let mut interner = Interner::new();
    let foo_id = interner.intern("foo");
    let bar_id = interner.intern("bar");
    let pattern = Pattern::Literal(Value::Keyword(foo_id));
    let value = Value::Keyword(bar_id);
    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn literal_matches_ratio() {
    let numer = Integer::from(1);
    let denom = Integer::from(2);
    let ratio = Ratio::new(&numer, &denom);
    let pattern = Pattern::Literal(Value::Ratio(ratio.clone()));
    let value = Value::Ratio(ratio);
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn literal_rejects_wrong_ratio() {
    let numer = Integer::from(1);
    let denom1 = Integer::from(2);
    let denom2 = Integer::from(3);
    let r1 = Ratio::new(&numer, &denom1);
    let r2 = Ratio::new(&numer, &denom2);
    let pattern = Pattern::Literal(Value::Ratio(r1));
    let value = Value::Ratio(r2);
    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn literal_matches_symbol() {
    let mut interner = Interner::new();
    let sym_id = interner.intern("my-symbol");
    let sym = Symbol::new(sym_id);
    let pattern = Pattern::Literal(Value::Symbol(sym.clone()));
    let value = Value::Symbol(sym);
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn literal_rejects_wrong_symbol() {
    let mut interner = Interner::new();
    let foo_id = interner.intern("foo");
    let bar_id = interner.intern("bar");
    let pattern = Pattern::Literal(Value::Symbol(Symbol::new(foo_id)));
    let value = Value::Symbol(Symbol::new(bar_id));
    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn literal_rejects_type_mismatch() {
    let pattern = Pattern::Literal(Value::Integer(Integer::from(42)));
    assert_eq!(try_match(&pattern, &Value::Nil), None);
    assert_eq!(try_match(&pattern, &Value::Bool(true)), None);
    assert_eq!(
        try_match(&pattern, &Value::String(HeapStr::from("42"))),
        None
    );
}
