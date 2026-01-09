// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Integration tests and error handling tests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::intrinsics::IntrinsicError;
use crate::value::Value;
use crate::vm::RuntimeError;

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
