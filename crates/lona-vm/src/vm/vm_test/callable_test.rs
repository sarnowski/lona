// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for callable data structures (keywords, maps, tuples as functions).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::intrinsics::IntrinsicError;
use crate::value::Value;
use crate::vm::RuntimeError;

// --- Keywords as functions ---

#[test]
fn eval_keyword_callable_basic() {
    let (mut proc, mut mem) = setup();
    // (:a %{:a 1 :b 2}) → 1
    let result = eval("(:a %{:a 1 :b 2})", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_keyword_callable_not_found() {
    let (mut proc, mut mem) = setup();
    // (:missing %{:a 1}) → nil
    let result = eval("(:missing %{:a 1})", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_keyword_callable_with_default() {
    let (mut proc, mut mem) = setup();
    // (:missing %{:a 1} :default) → :default
    let result = eval("(:missing %{:a 1} :default)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "default");
}

#[test]
fn eval_keyword_callable_found_with_default() {
    let (mut proc, mut mem) = setup();
    // (:a %{:a 1} :default) → 1 (ignores default when found)
    let result = eval("(:a %{:a 1} :default)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_keyword_callable_arity_error_zero_args() {
    let (mut proc, mut mem) = setup();
    // (:a) with no args → arity error
    let result = eval("(:a)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableArityError {
            expected: "1-2",
            got: 0
        })
    ));
}

#[test]
fn eval_keyword_callable_arity_error_too_many_args() {
    let (mut proc, mut mem) = setup();
    // (:a %{:a 1} :d :extra) → arity error
    let result = eval("(:a %{:a 1} :d :extra)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableArityError {
            expected: "1-2",
            got: 3
        })
    ));
}

#[test]
fn eval_keyword_callable_type_error_not_map() {
    let (mut proc, mut mem) = setup();
    // (:a 42) → ERROR: expected map
    let result = eval("(:a 42)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableTypeError {
            callable: "keyword",
            arg: 0,
            expected: "map"
        })
    ));
}

// --- Maps as functions ---

#[test]
fn eval_map_callable_basic() {
    let (mut proc, mut mem) = setup();
    // (%{:a 1 :b 2} :a) → 1
    let result = eval("(%{:a 1 :b 2} :a)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_map_callable_not_found() {
    let (mut proc, mut mem) = setup();
    // (%{:a 1} :missing) → nil
    let result = eval("(%{:a 1} :missing)", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_map_callable_with_default() {
    let (mut proc, mut mem) = setup();
    // (%{:a 1} :missing :default) → :default
    let result = eval("(%{:a 1} :missing :default)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "default");
}

#[test]
fn eval_map_callable_arity_error() {
    let (mut proc, mut mem) = setup();
    // (%{:a 1}) with no args → arity error
    let result = eval("(%{:a 1})", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableArityError {
            expected: "1-2",
            got: 0
        })
    ));
}

// --- Tuples as functions ---

#[test]
fn eval_tuple_callable_basic() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 1) → 20
    let result = eval("([10 20 30] 1)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(20));
}

#[test]
fn eval_tuple_callable_first_element() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 0) → 10
    let result = eval("([10 20 30] 0)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(10));
}

#[test]
fn eval_tuple_callable_last_element() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 2) → 30
    let result = eval("([10 20 30] 2)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(30));
}

#[test]
fn eval_tuple_callable_out_of_bounds() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 5) → error (no default)
    let result = eval("([10 20 30] 5)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::Intrinsic(IntrinsicError::IndexOutOfBounds {
            index: 5,
            len: 3
        }))
    ));
}

#[test]
fn eval_tuple_callable_with_default() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 5 :default) → :default
    let result = eval("([10 20 30] 5 :default)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "default");
}

#[test]
fn eval_tuple_callable_in_bounds_with_default() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 1 :default) → 20 (ignores default when in bounds)
    let result = eval("([10 20 30] 1 :default)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(20));
}

#[test]
fn eval_tuple_callable_arity_error() {
    let (mut proc, mut mem) = setup();
    // ([1 2]) with no args → arity error
    let result = eval("([1 2])", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableArityError {
            expected: "1-2",
            got: 0
        })
    ));
}

#[test]
fn eval_tuple_callable_type_error_not_integer() {
    let (mut proc, mut mem) = setup();
    // ([1 2 3] :a) → ERROR: expected integer index
    let result = eval("([1 2 3] :a)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableTypeError {
            callable: "tuple",
            arg: 0,
            expected: "integer index"
        })
    ));
}

// --- nth with 3 args (not-found) ---

#[test]
fn eval_nth_with_default_out_of_bounds() {
    let (mut proc, mut mem) = setup();
    // (nth [1 2 3] 10 :not-found) → :not-found
    let result = eval("(nth [1 2 3] 10 :not-found)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "not-found");
}

#[test]
fn eval_nth_with_default_in_bounds() {
    let (mut proc, mut mem) = setup();
    // (nth [1 2 3] 1 :not-found) → 2
    let result = eval("(nth [1 2 3] 1 :not-found)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(2));
}

#[test]
fn eval_nth_with_default_negative_index() {
    let (mut proc, mut mem) = setup();
    // (nth [1 2 3] -1 :not-found) → :not-found (negative out of bounds)
    let result = eval("(nth [1 2 3] -1 :not-found)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "not-found");
}
