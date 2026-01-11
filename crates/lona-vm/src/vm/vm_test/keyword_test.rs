// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for keywords.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::value::Value;

#[test]
fn eval_keyword_simple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval(":foo", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "foo");
}

#[test]
fn eval_keyword_qualified() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval(":my.ns/bar", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "my.ns/bar");
}

#[test]
fn eval_keyword_self_evaluating() {
    // Keywords evaluate to themselves (like numbers, strings, booleans)
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let k1 = eval(":foo", &mut proc, &mut realm, &mut mem).unwrap();
    let k2 = eval(":foo", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(k1.is_keyword());
    assert!(k2.is_keyword());
}

#[test]
fn eval_keyword_predicate() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(keyword? :foo)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(keyword? 'foo)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));

    let result = eval("(keyword? 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_keyword_equality() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Same keywords should be equal
    let result = eval("(= :foo :foo)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    // Different keywords should not be equal
    let result = eval("(= :foo :bar)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_keyword_name() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(name :hello)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_string());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "hello");
}

#[test]
fn eval_keyword_name_qualified() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(name :ns/hello)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_string());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "hello");
}

#[test]
fn eval_keyword_namespace() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(namespace :ns/hello)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_string());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "ns");

    // Unqualified keywords have no namespace
    let result = eval("(namespace :hello)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_keyword_constructor() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(keyword \"world\")", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "world");
}
