// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for functions (fn*) and closures.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::value::Value;
use crate::vm::RuntimeError;

// --- Function tests ---

#[test]
fn eval_fn_creation() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(fn* [x] x)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_fn());
}

#[test]
fn eval_fn_identity() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("((fn* [x] x) 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::int(42));
}

#[test]
fn eval_fn_with_body() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("((fn* [x] (+ x 1)) 5)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::int(6));
}

#[test]
fn eval_fn_two_args() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("((fn* [a b] (+ a b)) 3 4)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::int(7));
}

#[test]
fn eval_fn_three_args() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval(
        "((fn* [x y z] (+ x (+ y z))) 1 2 3)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Value::int(6));
}

#[test]
fn eval_fn_zero_args() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("((fn* [] 99))", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::int(99));
}

#[test]
fn eval_fn_returns_nil() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("((fn* [] nil))", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_fn_predicate_true() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(fn? (fn* [] nil))", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));
}

#[test]
fn eval_fn_predicate_false_int() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(fn? 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_fn_predicate_false_keyword() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(fn? :keyword)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_fn_not_callable_int() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(42 1 2)", &mut proc, &mut realm, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::NotCallable {
            type_name: "integer"
        })
    ));
}

#[test]
fn eval_fn_not_callable_string() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(\"hello\" 1)", &mut proc, &mut realm, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::NotCallable {
            type_name: "string"
        })
    ));
}

#[test]
fn eval_fn_arity_mismatch() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Pass 1 arg to a 2-arg function
    let result = eval("((fn* [a b] (+ a b)) 1)", &mut proc, &mut realm, &mut mem);
    assert!(matches!(result, Err(RuntimeError::ArityMismatch { .. })));
}

#[test]
fn eval_fn_nested_call() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // A function that returns a value, used in arithmetic
    let result = eval(
        "(+ ((fn* [] 10)) ((fn* [] 20)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Value::int(30));
}

#[test]
fn eval_fn_with_tuple_param() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Function that returns the count of its tuple argument
    let result = eval(
        "((fn* [t] (count t)) [1 2 3])",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Value::int(3));
}

#[test]
fn eval_fn_with_map_param() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Function that gets a value from a map argument
    let result = eval(
        "((fn* [m] (get m :a)) %{:a 42})",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Value::int(42));
}

// --- Closure tests ---

#[test]
fn eval_closure_simple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Outer function returns inner function that captures x
    let result = eval(
        "(((fn* [x] (fn* [y] (+ x y))) 10) 5)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Value::int(15));
}

#[test]
fn eval_closure_multiple_captures() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Inner function captures both a and b
    let result = eval(
        "(((fn* [a b] (fn* [c] (+ a (+ b c)))) 1 2) 3)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Value::int(6));
}

#[test]
fn eval_closure_nested_multi_level() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Deeply nested closure capturing from multiple levels:
    // - Level 3 (innermost) captures z from params, y from level 2, x from level 1
    let result = eval(
        "((((fn* [x] (fn* [y] (fn* [z] (+ x (+ y z))))) 1) 2) 3)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Value::int(6));
}

#[test]
fn eval_closure_is_fn() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // A closure should pass the fn? predicate
    let result = eval(
        "(fn? ((fn* [x] (fn* [y] (+ x y))) 5))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Value::bool(true));
}

#[test]
fn eval_closure_capture_keyword() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Capture a keyword
    let result = eval(
        "(((fn* [k] (fn* [m] (get m k))) :a) %{:a 42})",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Value::int(42));
}
