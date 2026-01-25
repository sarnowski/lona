// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for tuples.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

#[test]
fn eval_tuple_simple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("[1 2 3]", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_tuple(&mem, result));
    assert_eq!(proc.read_term_tuple_len(&mem, result).unwrap(), 3);
    assert_eq!(
        proc.read_term_tuple_element(&mem, result, 0).unwrap(),
        int(1)
    );
    assert_eq!(
        proc.read_term_tuple_element(&mem, result, 1).unwrap(),
        int(2)
    );
    assert_eq!(
        proc.read_term_tuple_element(&mem, result, 2).unwrap(),
        int(3)
    );
}

#[test]
fn eval_tuple_empty() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("[]", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_tuple(&mem, result));
    assert_eq!(proc.read_term_tuple_len(&mem, result).unwrap(), 0);
}

#[test]
fn eval_tuple_elements_evaluated() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Elements should be evaluated
    let result = eval("[(+ 1 2) 4]", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_tuple(&mem, result));
    assert_eq!(proc.read_term_tuple_len(&mem, result).unwrap(), 2);
    assert_eq!(
        proc.read_term_tuple_element(&mem, result, 0).unwrap(),
        int(3)
    );
    assert_eq!(
        proc.read_term_tuple_element(&mem, result, 1).unwrap(),
        int(4)
    );
}

#[test]
fn eval_tuple_nested() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("[[1 2] [3 4]]", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_tuple(&mem, result));
    assert_eq!(proc.read_term_tuple_len(&mem, result).unwrap(), 2);

    let inner1 = proc.read_term_tuple_element(&mem, result, 0).unwrap();
    assert!(proc.is_term_tuple(&mem, inner1));
    assert_eq!(proc.read_term_tuple_len(&mem, inner1).unwrap(), 2);
}

#[test]
fn eval_tuple_predicate() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(tuple? [1 2])", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);

    let result = eval("(tuple? '(1 2))", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);

    let result = eval("(tuple? 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);
}

#[test]
fn eval_tuple_nth() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(nth [10 20 30] 0)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(10));

    let result = eval("(nth [10 20 30] 1)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(20));

    let result = eval("(nth [10 20 30] 2)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(30));
}

#[test]
fn eval_tuple_count() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(count [1 2 3])", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(3));

    let result = eval("(count [])", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(0));
}

#[test]
fn eval_tuple_with_keywords() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("[:a :b :c]", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_tuple(&mem, result));
    assert_eq!(proc.read_term_tuple_len(&mem, result).unwrap(), 3);

    let k1 = proc.read_term_tuple_element(&mem, result, 0).unwrap();
    assert!(proc.is_term_keyword(k1));
    let idx = k1.as_keyword_index().unwrap();
    assert_eq!(realm.keyword_name(&mem, idx).unwrap(), "a");
}
