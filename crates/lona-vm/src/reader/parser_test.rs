// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the Lonala parser.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{ParseError, ReadError, read};
use crate::Vaddr;
use crate::heap::Heap;
use crate::platform::MockVSpace;
use crate::value::Value;

fn setup() -> (MockVSpace, Heap) {
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x10000));
    let heap = Heap::new(Vaddr::new(0x20000), 0x10000);
    (mem, heap)
}

#[test]
fn read_nil() {
    let (mut mem, mut heap) = setup();
    let value = read("nil", &mut heap, &mut mem).unwrap().unwrap();
    assert!(value.is_nil());
}

#[test]
fn read_booleans() {
    let (mut mem, mut heap) = setup();

    let t = read("true", &mut heap, &mut mem).unwrap().unwrap();
    assert_eq!(t, Value::bool(true));

    let f = read("false", &mut heap, &mut mem).unwrap().unwrap();
    assert_eq!(f, Value::bool(false));
}

#[test]
fn read_integers() {
    let (mut mem, mut heap) = setup();

    assert_eq!(
        read("0", &mut heap, &mut mem).unwrap().unwrap(),
        Value::int(0)
    );
    assert_eq!(
        read("42", &mut heap, &mut mem).unwrap().unwrap(),
        Value::int(42)
    );
    assert_eq!(
        read("-123", &mut heap, &mut mem).unwrap().unwrap(),
        Value::int(-123)
    );
}

#[test]
fn read_strings() {
    let (mut mem, mut heap) = setup();

    let value = read("\"hello\"", &mut heap, &mut mem).unwrap().unwrap();
    let s = heap.read_string(&mem, value).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn read_empty_list() {
    let (mut mem, mut heap) = setup();

    let value = read("()", &mut heap, &mut mem).unwrap().unwrap();
    assert!(value.is_nil());
}

#[test]
fn read_list() {
    let (mut mem, mut heap) = setup();

    let value = read("(1 2 3)", &mut heap, &mut mem).unwrap().unwrap();
    assert!(value.is_pair());

    // Check structure: (1 . (2 . (3 . nil)))
    let p1 = heap.read_pair(&mem, value).unwrap();
    assert_eq!(p1.first, Value::int(1));

    let p2 = heap.read_pair(&mem, p1.rest).unwrap();
    assert_eq!(p2.first, Value::int(2));

    let p3 = heap.read_pair(&mem, p2.rest).unwrap();
    assert_eq!(p3.first, Value::int(3));
    assert!(p3.rest.is_nil());
}

#[test]
fn read_nested_list() {
    let (mut mem, mut heap) = setup();

    let value = read("(1 (2 3))", &mut heap, &mut mem).unwrap().unwrap();

    let p1 = heap.read_pair(&mem, value).unwrap();
    assert_eq!(p1.first, Value::int(1));

    let inner = p1.rest;
    let p2 = heap.read_pair(&mem, inner).unwrap();
    assert!(p2.first.is_pair()); // (2 3)

    let inner_list = heap.read_pair(&mem, p2.first).unwrap();
    assert_eq!(inner_list.first, Value::int(2));
}

#[test]
fn read_quote() {
    let (mut mem, mut heap) = setup();

    // 'x => (quote x)
    let value = read("'x", &mut heap, &mut mem).unwrap().unwrap();
    assert!(value.is_pair());

    let p1 = heap.read_pair(&mem, value).unwrap();
    let quote_name = heap.read_string(&mem, p1.first).unwrap();
    assert_eq!(quote_name, "quote");

    let p2 = heap.read_pair(&mem, p1.rest).unwrap();
    let x_name = heap.read_string(&mem, p2.first).unwrap();
    assert_eq!(x_name, "x");
    assert!(p2.rest.is_nil());
}

#[test]
fn read_quote_list() {
    let (mut mem, mut heap) = setup();

    // '(1 2 3) => (quote (1 2 3))
    let value = read("'(1 2 3)", &mut heap, &mut mem).unwrap().unwrap();
    assert!(value.is_pair());

    let p1 = heap.read_pair(&mem, value).unwrap();
    let quote_name = heap.read_string(&mem, p1.first).unwrap();
    assert_eq!(quote_name, "quote");

    let p2 = heap.read_pair(&mem, p1.rest).unwrap();
    assert!(p2.first.is_pair()); // The list (1 2 3)
}

#[test]
fn read_empty_input() {
    let (mut mem, mut heap) = setup();
    let value = read("", &mut heap, &mut mem).unwrap();
    assert!(value.is_none());
}

#[test]
fn read_whitespace_only() {
    let (mut mem, mut heap) = setup();
    let value = read("   \n\t  ", &mut heap, &mut mem).unwrap();
    assert!(value.is_none());
}

#[test]
fn read_unmatched_rparen() {
    let (mut mem, mut heap) = setup();
    let err = read(")", &mut heap, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::UnmatchedRParen)));
}

#[test]
fn read_unclosed_list() {
    let (mut mem, mut heap) = setup();
    let err = read("(1 2", &mut heap, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::UnexpectedEof)));
}
