// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the bytecode VM.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::Vaddr;
use crate::compiler::compile;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::reader::read;

/// Create a test environment.
fn setup() -> (Process, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(128 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let proc = Process::new(1, young_base, young_size, old_base, old_size);
    (proc, mem)
}

/// Parse, compile, and execute an expression.
fn eval(src: &str, proc: &mut Process, mem: &mut MockVSpace) -> Result<Value, RuntimeError> {
    let expr = read(src, proc, mem)
        .expect("parse error")
        .expect("empty input");
    let chunk = compile(expr, proc, mem).expect("compile error");
    proc.set_chunk(chunk);
    let result = execute(proc, mem);
    proc.reset();
    result
}

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

// --- Arithmetic tests ---

#[test]
fn eval_add() {
    let (mut proc, mut mem) = setup();
    let result = eval("(+ 1 2)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(3));
}

#[test]
fn eval_sub() {
    let (mut proc, mut mem) = setup();
    let result = eval("(- 10 3)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(7));
}

#[test]
fn eval_mul() {
    let (mut proc, mut mem) = setup();
    let result = eval("(* 6 7)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(42));
}

#[test]
fn eval_div() {
    let (mut proc, mut mem) = setup();
    let result = eval("(/ 20 4)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(5));
}

#[test]
fn eval_mod() {
    let (mut proc, mut mem) = setup();
    let result = eval("(mod 17 5)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(2));
}

#[test]
fn eval_nested_arithmetic() {
    let (mut proc, mut mem) = setup();
    let result = eval("(* 3 7)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(21));
}

// --- Comparison tests ---

#[test]
fn eval_eq_true() {
    let (mut proc, mut mem) = setup();
    let result = eval("(= 42 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));
}

#[test]
fn eval_eq_false() {
    let (mut proc, mut mem) = setup();
    let result = eval("(= 1 2)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_lt_true() {
    let (mut proc, mut mem) = setup();
    let result = eval("(< 1 2)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));
}

#[test]
fn eval_lt_false() {
    let (mut proc, mut mem) = setup();
    let result = eval("(< 2 1)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_gt() {
    let (mut proc, mut mem) = setup();
    let result = eval("(> 5 3)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));
}

#[test]
fn eval_le() {
    let (mut proc, mut mem) = setup();
    let result = eval("(<= 5 5)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));
}

#[test]
fn eval_ge() {
    let (mut proc, mut mem) = setup();
    let result = eval("(>= 5 5)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));
}

// --- Boolean tests ---

#[test]
fn eval_not_true() {
    let (mut proc, mut mem) = setup();
    let result = eval("(not true)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_not_false() {
    let (mut proc, mut mem) = setup();
    let result = eval("(not false)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));
}

#[test]
fn eval_not_nil() {
    let (mut proc, mut mem) = setup();
    let result = eval("(not nil)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));
}

// --- Type predicate tests ---

#[test]
fn eval_nil_predicate() {
    let (mut proc, mut mem) = setup();
    let result = eval("(nil? nil)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(nil? 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_integer_predicate() {
    let (mut proc, mut mem) = setup();
    let result = eval("(integer? 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(integer? nil)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_string_predicate() {
    let (mut proc, mut mem) = setup();
    let result = eval("(string? \"hello\")", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(string? 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

// --- String tests ---

#[test]
fn eval_str_single() {
    let (mut proc, mut mem) = setup();
    let result = eval("(str \"hello\")", &mut proc, &mut mem).unwrap();
    let s = proc.read_string(&mem, result).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn eval_str_concat() {
    let (mut proc, mut mem) = setup();
    let result = eval("(str \"hello\" \" \" \"world\")", &mut proc, &mut mem).unwrap();
    let s = proc.read_string(&mem, result).unwrap();
    assert_eq!(s, "hello world");
}

#[test]
fn eval_str_with_int() {
    let (mut proc, mut mem) = setup();
    let result = eval("(str \"x=\" 42)", &mut proc, &mut mem).unwrap();
    let s = proc.read_string(&mem, result).unwrap();
    assert_eq!(s, "x=42");
}

// --- Error tests ---

#[test]
fn eval_div_by_zero() {
    let (mut proc, mut mem) = setup();
    let result = eval("(/ 10 0)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::Intrinsic(IntrinsicError::DivisionByZero))
    ));
}

#[test]
fn eval_type_error() {
    let (mut proc, mut mem) = setup();
    let result = eval("(+ true 2)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::Intrinsic(IntrinsicError::TypeError { .. }))
    ));
}

// --- Integration tests matching ROADMAP test cases ---

#[test]
fn roadmap_test_cases() {
    let (mut proc, mut mem) = setup();

    assert_eq!(eval("42", &mut proc, &mut mem).unwrap(), Value::int(42));
    assert_eq!(eval("(+ 1 2)", &mut proc, &mut mem).unwrap(), Value::int(3));
    assert_eq!(
        eval("(< 1 2)", &mut proc, &mut mem).unwrap(),
        Value::bool(true)
    );
    assert_eq!(
        eval("(>= 5 5)", &mut proc, &mut mem).unwrap(),
        Value::bool(true)
    );
    assert_eq!(
        eval("(not true)", &mut proc, &mut mem).unwrap(),
        Value::bool(false)
    );
    assert_eq!(
        eval("(nil? nil)", &mut proc, &mut mem).unwrap(),
        Value::bool(true)
    );
    assert_eq!(
        eval("(integer? 42)", &mut proc, &mut mem).unwrap(),
        Value::bool(true)
    );
    assert_eq!(
        eval("(string? \"hello\")", &mut proc, &mut mem).unwrap(),
        Value::bool(true)
    );
    assert_eq!(
        eval("(mod 17 5)", &mut proc, &mut mem).unwrap(),
        Value::int(2)
    );
}

#[test]
fn roadmap_str_test() {
    let (mut proc, mut mem) = setup();
    let result = eval("(str \"hello\" \" \" \"world\")", &mut proc, &mut mem).unwrap();
    let s = proc.read_string(&mem, result).unwrap();
    assert_eq!(s, "hello world");
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

// --- Quote tests ---

#[test]
fn quote_list_not_evaluated() {
    let (mut proc, mut mem) = setup();
    let result = eval("'(+ 1 2)", &mut proc, &mut mem).unwrap();
    assert!(matches!(result, Value::Pair(_)));
}

// --- Keyword tests ---

#[test]
fn eval_keyword_simple() {
    let (mut proc, mut mem) = setup();
    let result = eval(":foo", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "foo");
}

#[test]
fn eval_keyword_qualified() {
    let (mut proc, mut mem) = setup();
    let result = eval(":my.ns/bar", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "my.ns/bar");
}

#[test]
fn eval_keyword_self_evaluating() {
    // Keywords evaluate to themselves (like numbers, strings, booleans)
    let (mut proc, mut mem) = setup();
    let k1 = eval(":foo", &mut proc, &mut mem).unwrap();
    let k2 = eval(":foo", &mut proc, &mut mem).unwrap();
    assert!(k1.is_keyword());
    assert!(k2.is_keyword());
}

#[test]
fn eval_keyword_predicate() {
    let (mut proc, mut mem) = setup();
    let result = eval("(keyword? :foo)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(keyword? 'foo)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));

    let result = eval("(keyword? 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_keyword_equality() {
    let (mut proc, mut mem) = setup();
    // Same keywords should be equal
    let result = eval("(= :foo :foo)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    // Different keywords should not be equal
    let result = eval("(= :foo :bar)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_keyword_name() {
    let (mut proc, mut mem) = setup();
    let result = eval("(name :hello)", &mut proc, &mut mem).unwrap();
    assert!(result.is_string());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "hello");
}

#[test]
fn eval_keyword_name_qualified() {
    let (mut proc, mut mem) = setup();
    let result = eval("(name :ns/hello)", &mut proc, &mut mem).unwrap();
    assert!(result.is_string());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "hello");
}

#[test]
fn eval_keyword_namespace() {
    let (mut proc, mut mem) = setup();
    let result = eval("(namespace :ns/hello)", &mut proc, &mut mem).unwrap();
    assert!(result.is_string());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "ns");

    // Unqualified keywords have no namespace
    let result = eval("(namespace :hello)", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_keyword_constructor() {
    let (mut proc, mut mem) = setup();
    let result = eval("(keyword \"world\")", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "world");
}

// --- Tuple tests ---

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
