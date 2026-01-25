// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for maps.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

#[test]
fn eval_map_empty() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("%{}", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_map(&mem, result));
    // Empty map has nil entries
    let entries = proc.read_term_map_entries(&mem, result).unwrap();
    assert!(entries.is_nil());
}

#[test]
fn eval_map_simple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("%{:a 1 :b 2}", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_map(&mem, result));
}

#[test]
fn eval_map_predicate() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(map? %{:a 1})", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::TRUE);

    let result = eval("(map? [1 2])", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);

    let result = eval("(map? 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);
}

#[test]
fn eval_map_get() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(get %{:a 1 :b 2} :a)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(1));

    let result = eval("(get %{:a 1 :b 2} :b)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(2));
}

#[test]
fn eval_map_get_not_found() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(get %{:a 1} :x)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_map_get_with_default() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(get %{:a 1} :x :default)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_keyword(result));
    let idx = result.as_keyword_index().unwrap();
    assert_eq!(realm.keyword_name(&mem, idx).unwrap(), "default");

    // Existing key should return value, not default
    let result = eval("(get %{:a 1} :a :default)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(1));
}

#[test]
fn eval_map_put() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval(
        "(get (put %{:a 1} :b 2) :b)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, int(2));

    // Original key still accessible
    let result = eval(
        "(get (put %{:a 1} :b 2) :a)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, int(1));
}

#[test]
fn eval_map_count() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(count %{:a 1 :b 2})", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(2));

    let result = eval("(count %{})", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(0));
}

#[test]
fn eval_map_keys() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(keys %{:a 1})", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_list());
    let (first, _rest) = proc.read_term_pair(&mem, result).unwrap();
    assert!(proc.is_term_keyword(first));
    let idx = first.as_keyword_index().unwrap();
    assert_eq!(realm.keyword_name(&mem, idx).unwrap(), "a");
}

#[test]
fn eval_map_vals() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(vals %{:a 1})", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_list());
    let (first, _rest) = proc.read_term_pair(&mem, result).unwrap();
    assert_eq!(first, int(1));
}

#[test]
fn eval_map_nested_values() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("%{:a [1 2] :b [3 4]}", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_map(&mem, result));

    let inner = eval(
        "(get %{:a [1 2] :b [3 4]} :a)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert!(proc.is_term_tuple(&mem, inner));
    assert_eq!(proc.read_term_tuple_len(&mem, inner).unwrap(), 2);
}

#[test]
fn eval_map_elements_evaluated() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(get %{:a (+ 1 2)} :a)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(3));
}
