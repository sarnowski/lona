// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for arithmetic, comparison, boolean, type predicates, and string operations.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

// --- Arithmetic tests ---

#[test]
fn eval_add() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(+ 1 2)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(3));
}

#[test]
fn eval_sub() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(- 10 3)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(7));
}

#[test]
fn eval_mul() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(* 6 7)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(42));
}

#[test]
fn eval_div() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(/ 20 4)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(5));
}

#[test]
fn eval_mod() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(mod 17 5)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(2));
}

#[test]
fn eval_nested_arithmetic() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(* 3 7)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(21));
}

// --- Comparison tests ---

#[test]
fn eval_eq_true() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(= 42 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);
}

#[test]
fn eval_eq_false() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(= 1 2)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);
}

#[test]
fn eval_lt_true() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(< 1 2)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);
}

#[test]
fn eval_lt_false() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(< 2 1)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);
}

#[test]
fn eval_gt() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(> 5 3)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);
}

#[test]
fn eval_le() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(<= 5 5)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);
}

#[test]
fn eval_ge() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(>= 5 5)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);
}

// --- Boolean tests ---

#[test]
fn eval_not_true() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(not true)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);
}

#[test]
fn eval_not_false() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(not false)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);
}

#[test]
fn eval_not_nil() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(not nil)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);
}

// --- Type predicate tests ---

#[test]
fn eval_nil_predicate() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(nil? nil)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);

    let result = eval("(nil? 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);
}

#[test]
fn eval_integer_predicate() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(integer? 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);

    let result = eval("(integer? nil)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);
}

#[test]
fn eval_string_predicate() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(string? \"hello\")", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);

    let result = eval("(string? 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);
}

// --- String tests ---

#[test]
fn eval_str_single() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(str \"hello\")", &mut proc, &mut realm, &mut mem).unwrap();
    let s = proc.read_term_string(&mem, result).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn eval_str_concat() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval(
        "(str \"hello\" \" \" \"world\")",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    let s = proc.read_term_string(&mem, result).unwrap();
    assert_eq!(s, "hello world");
}

#[test]
fn eval_str_with_int() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(str \"x=\" 42)", &mut proc, &mut realm, &mut mem).unwrap();
    let s = proc.read_term_string(&mem, result).unwrap();
    assert_eq!(s, "x=42");
}
