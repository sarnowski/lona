// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for tuples.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::value::Value;

#[test]
fn eval_tuple_simple() {
    let (mut proc, mut mem) = setup();
    let result = eval("[1 2 3]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 3);
    assert_eq!(
        proc.read_tuple_element(&mem, result, 0).unwrap(),
        Value::int(1)
    );
    assert_eq!(
        proc.read_tuple_element(&mem, result, 1).unwrap(),
        Value::int(2)
    );
    assert_eq!(
        proc.read_tuple_element(&mem, result, 2).unwrap(),
        Value::int(3)
    );
}

#[test]
fn eval_tuple_empty() {
    let (mut proc, mut mem) = setup();
    let result = eval("[]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 0);
}

#[test]
fn eval_tuple_elements_evaluated() {
    let (mut proc, mut mem) = setup();
    // Elements should be evaluated
    let result = eval("[(+ 1 2) 4]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 2);
    assert_eq!(
        proc.read_tuple_element(&mem, result, 0).unwrap(),
        Value::int(3)
    );
    assert_eq!(
        proc.read_tuple_element(&mem, result, 1).unwrap(),
        Value::int(4)
    );
}

#[test]
fn eval_tuple_nested() {
    let (mut proc, mut mem) = setup();
    let result = eval("[[1 2] [3 4]]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 2);

    let inner1 = proc.read_tuple_element(&mem, result, 0).unwrap();
    assert!(inner1.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, inner1).unwrap(), 2);
}

#[test]
fn eval_tuple_predicate() {
    let (mut proc, mut mem) = setup();
    let result = eval("(tuple? [1 2])", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(tuple? '(1 2))", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));

    let result = eval("(tuple? 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_tuple_nth() {
    let (mut proc, mut mem) = setup();
    let result = eval("(nth [10 20 30] 0)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(10));

    let result = eval("(nth [10 20 30] 1)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(20));

    let result = eval("(nth [10 20 30] 2)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(30));
}

#[test]
fn eval_tuple_count() {
    let (mut proc, mut mem) = setup();
    let result = eval("(count [1 2 3])", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(3));

    let result = eval("(count [])", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(0));
}

#[test]
fn eval_tuple_with_keywords() {
    let (mut proc, mut mem) = setup();
    let result = eval("[:a :b :c]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 3);

    let k1 = proc.read_tuple_element(&mem, result, 0).unwrap();
    assert!(k1.is_keyword());
    assert_eq!(proc.read_string(&mem, k1).unwrap(), "a");
}
