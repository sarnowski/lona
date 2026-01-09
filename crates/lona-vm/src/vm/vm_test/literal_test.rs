// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for literal evaluation and quote.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::value::Value;

// --- Literal tests ---

#[test]
fn eval_nil() {
    let (mut proc, mut mem) = setup();
    let result = eval("nil", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn eval_true() {
    let (mut proc, mut mem) = setup();
    let result = eval("true", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));
}

#[test]
fn eval_false() {
    let (mut proc, mut mem) = setup();
    let result = eval("false", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_integer() {
    let (mut proc, mut mem) = setup();
    let result = eval("42", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(42));
}

#[test]
fn eval_negative_integer() {
    let (mut proc, mut mem) = setup();
    let result = eval("-100", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(-100));
}

#[test]
fn eval_large_integer() {
    let (mut proc, mut mem) = setup();
    let result = eval("1000000", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1_000_000));
}

#[test]
fn eval_string() {
    let (mut proc, mut mem) = setup();
    let result = eval("\"hello\"", &mut proc, &mut mem).unwrap();
    let s = proc.read_string(&mem, result).unwrap();
    assert_eq!(s, "hello");
}

// --- Quote tests ---

#[test]
fn quote_list_not_evaluated() {
    let (mut proc, mut mem) = setup();
    let result = eval("'(+ 1 2)", &mut proc, &mut mem).unwrap();
    assert!(matches!(result, Value::Pair(_)));
}

// --- Nested expressions ---

#[test]
fn nested_both_add_add() {
    let (mut proc, mut mem) = setup();
    assert_eq!(
        eval("(+ (+ 1 2) (+ 3 4))", &mut proc, &mut mem).unwrap(),
        Value::int(10)
    );
}

#[test]
fn plan_deliverable() {
    let (mut proc, mut mem) = setup();
    assert_eq!(
        eval("(+ 1 (* 2 3))", &mut proc, &mut mem).unwrap(),
        Value::int(7)
    );
}
