// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the Lonala parser.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{ParseError, ReadError, read};
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::value::Value;

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

#[test]
fn read_nil() {
    let (mut proc, mut mem) = setup();
    let value = read("nil", &mut proc, &mut mem).unwrap().unwrap();
    assert!(value.is_nil());
}

#[test]
fn read_booleans() {
    let (mut proc, mut mem) = setup();

    let t = read("true", &mut proc, &mut mem).unwrap().unwrap();
    assert_eq!(t, Value::bool(true));

    let f = read("false", &mut proc, &mut mem).unwrap().unwrap();
    assert_eq!(f, Value::bool(false));
}

#[test]
fn read_integers() {
    let (mut proc, mut mem) = setup();

    assert_eq!(
        read("0", &mut proc, &mut mem).unwrap().unwrap(),
        Value::int(0)
    );
    assert_eq!(
        read("42", &mut proc, &mut mem).unwrap().unwrap(),
        Value::int(42)
    );
    assert_eq!(
        read("-123", &mut proc, &mut mem).unwrap().unwrap(),
        Value::int(-123)
    );
}

#[test]
fn read_strings() {
    let (mut proc, mut mem) = setup();

    let value = read("\"hello\"", &mut proc, &mut mem).unwrap().unwrap();
    let s = proc.read_string(&mem, value).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn read_empty_list() {
    let (mut proc, mut mem) = setup();

    let value = read("()", &mut proc, &mut mem).unwrap().unwrap();
    assert!(value.is_nil());
}

#[test]
fn read_list() {
    let (mut proc, mut mem) = setup();

    let value = read("(1 2 3)", &mut proc, &mut mem).unwrap().unwrap();
    assert!(value.is_pair());

    // Check structure: (1 . (2 . (3 . nil)))
    let p1 = proc.read_pair(&mem, value).unwrap();
    assert_eq!(p1.first, Value::int(1));

    let p2 = proc.read_pair(&mem, p1.rest).unwrap();
    assert_eq!(p2.first, Value::int(2));

    let p3 = proc.read_pair(&mem, p2.rest).unwrap();
    assert_eq!(p3.first, Value::int(3));
    assert!(p3.rest.is_nil());
}

#[test]
fn read_nested_list() {
    let (mut proc, mut mem) = setup();

    let value = read("(1 (2 3))", &mut proc, &mut mem).unwrap().unwrap();

    let p1 = proc.read_pair(&mem, value).unwrap();
    assert_eq!(p1.first, Value::int(1));

    let inner = p1.rest;
    let p2 = proc.read_pair(&mem, inner).unwrap();
    assert!(p2.first.is_pair()); // (2 3)

    let inner_list = proc.read_pair(&mem, p2.first).unwrap();
    assert_eq!(inner_list.first, Value::int(2));
}

#[test]
fn read_quote() {
    let (mut proc, mut mem) = setup();

    // 'x => (quote x)
    let value = read("'x", &mut proc, &mut mem).unwrap().unwrap();
    assert!(value.is_pair());

    let p1 = proc.read_pair(&mem, value).unwrap();
    let quote_name = proc.read_string(&mem, p1.first).unwrap();
    assert_eq!(quote_name, "quote");

    let p2 = proc.read_pair(&mem, p1.rest).unwrap();
    let x_name = proc.read_string(&mem, p2.first).unwrap();
    assert_eq!(x_name, "x");
    assert!(p2.rest.is_nil());
}

#[test]
fn read_quote_list() {
    let (mut proc, mut mem) = setup();

    // '(1 2 3) => (quote (1 2 3))
    let value = read("'(1 2 3)", &mut proc, &mut mem).unwrap().unwrap();
    assert!(value.is_pair());

    let p1 = proc.read_pair(&mem, value).unwrap();
    let quote_name = proc.read_string(&mem, p1.first).unwrap();
    assert_eq!(quote_name, "quote");

    let p2 = proc.read_pair(&mem, p1.rest).unwrap();
    assert!(p2.first.is_pair()); // The list (1 2 3)
}

#[test]
fn read_empty_input() {
    let (mut proc, mut mem) = setup();
    let value = read("", &mut proc, &mut mem).unwrap();
    assert!(value.is_none());
}

#[test]
fn read_whitespace_only() {
    let (mut proc, mut mem) = setup();
    let value = read("   \n\t  ", &mut proc, &mut mem).unwrap();
    assert!(value.is_none());
}

#[test]
fn read_unmatched_rparen() {
    let (mut proc, mut mem) = setup();
    let err = read(")", &mut proc, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::UnmatchedRParen)));
}

#[test]
fn read_unclosed_list() {
    let (mut proc, mut mem) = setup();
    let err = read("(1 2", &mut proc, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::UnexpectedEof)));
}
