// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for maps.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::value::Value;

#[test]
fn eval_map_empty() {
    let (mut proc, mut mem) = setup();
    let result = eval("%{}", &mut proc, &mut mem).unwrap();
    assert!(result.is_map());
    // Empty map has 0 entries
    let map = proc.read_map(&mem, result).unwrap();
    assert!(map.entries.is_nil());
}

#[test]
fn eval_map_simple() {
    let (mut proc, mut mem) = setup();
    let result = eval("%{:a 1 :b 2}", &mut proc, &mut mem).unwrap();
    assert!(result.is_map());
}

#[test]
fn eval_map_predicate() {
    let (mut proc, mut mem) = setup();
    let result = eval("(map? %{:a 1})", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(map? [1 2])", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));

    let result = eval("(map? 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_map_get() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get %{:a 1 :b 2} :a)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));

    let result = eval("(get %{:a 1 :b 2} :b)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(2));
}

#[test]
fn eval_map_get_not_found() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get %{:a 1} :x)", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_map_get_with_default() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get %{:a 1} :x :default)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "default");

    // Existing key should return value, not default
    let result = eval("(get %{:a 1} :a :default)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_map_put() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get (put %{:a 1} :b 2) :b)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(2));

    // Original key still accessible
    let result = eval("(get (put %{:a 1} :b 2) :a)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_map_count() {
    let (mut proc, mut mem) = setup();
    let result = eval("(count %{:a 1 :b 2})", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(2));

    let result = eval("(count %{})", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(0));
}

#[test]
fn eval_map_keys() {
    let (mut proc, mut mem) = setup();
    let result = eval("(keys %{:a 1})", &mut proc, &mut mem).unwrap();
    assert!(result.is_pair());
    let pair = proc.read_pair(&mem, result).unwrap();
    assert!(pair.first.is_keyword());
    assert_eq!(proc.read_string(&mem, pair.first).unwrap(), "a");
}

#[test]
fn eval_map_vals() {
    let (mut proc, mut mem) = setup();
    let result = eval("(vals %{:a 1})", &mut proc, &mut mem).unwrap();
    assert!(result.is_pair());
    let pair = proc.read_pair(&mem, result).unwrap();
    assert_eq!(pair.first, Value::int(1));
}

#[test]
fn eval_map_nested_values() {
    let (mut proc, mut mem) = setup();
    let result = eval("%{:a [1 2] :b [3 4]}", &mut proc, &mut mem).unwrap();
    assert!(result.is_map());

    let inner = eval("(get %{:a [1 2] :b [3 4]} :a)", &mut proc, &mut mem).unwrap();
    assert!(inner.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, inner).unwrap(), 2);
}

#[test]
fn eval_map_elements_evaluated() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get %{:a (+ 1 2)} :a)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(3));
}
