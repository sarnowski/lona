// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the value printer.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::Value;
use super::printer::print_to_string;
use crate::Vaddr;
use crate::heap::Heap;
use crate::platform::MockVSpace;

fn setup() -> (MockVSpace, Heap) {
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x10000));
    let heap = Heap::new(Vaddr::new(0x20000), 0x10000);
    (mem, heap)
}

#[test]
fn print_nil() {
    let (mem, heap) = setup();
    assert_eq!(print_to_string(Value::nil(), &heap, &mem), "nil");
}

#[test]
fn print_booleans() {
    let (mem, heap) = setup();
    assert_eq!(print_to_string(Value::bool(true), &heap, &mem), "true");
    assert_eq!(print_to_string(Value::bool(false), &heap, &mem), "false");
}

#[test]
fn print_integers() {
    let (mem, heap) = setup();
    assert_eq!(print_to_string(Value::int(0), &heap, &mem), "0");
    assert_eq!(print_to_string(Value::int(42), &heap, &mem), "42");
    assert_eq!(print_to_string(Value::int(-1), &heap, &mem), "-1");
    assert_eq!(
        print_to_string(Value::int(123_456_789), &heap, &mem),
        "123456789"
    );
    assert_eq!(
        print_to_string(Value::int(i64::MIN), &heap, &mem),
        "-9223372036854775808"
    );
}

#[test]
fn print_strings() {
    let (mut mem, mut heap) = setup();

    let s = heap.alloc_string(&mut mem, "hello").unwrap();
    assert_eq!(print_to_string(s, &heap, &mem), "\"hello\"");

    let empty = heap.alloc_string(&mut mem, "").unwrap();
    assert_eq!(print_to_string(empty, &heap, &mem), "\"\"");
}

#[test]
fn print_string_escapes() {
    let (mut mem, mut heap) = setup();

    let s = heap.alloc_string(&mut mem, "a\nb").unwrap();
    assert_eq!(print_to_string(s, &heap, &mem), "\"a\\nb\"");

    let s2 = heap.alloc_string(&mut mem, "tab\there").unwrap();
    assert_eq!(print_to_string(s2, &heap, &mem), "\"tab\\there\"");

    let s3 = heap.alloc_string(&mut mem, "quote: \"hi\"").unwrap();
    assert_eq!(print_to_string(s3, &heap, &mem), "\"quote: \\\"hi\\\"\"");
}

#[test]
fn print_symbols() {
    let (mut mem, mut heap) = setup();

    let sym = heap.alloc_symbol(&mut mem, "foo").unwrap();
    assert_eq!(print_to_string(sym, &heap, &mem), "foo");
}

#[test]
fn print_empty_list() {
    let (mem, heap) = setup();
    // Empty list is just nil printed as a list
    // Actually empty list is nil
    assert_eq!(print_to_string(Value::nil(), &heap, &mem), "nil");
}

#[test]
fn print_list() {
    let (mut mem, mut heap) = setup();

    // (1 2 3)
    let p3 = heap
        .alloc_pair(&mut mem, Value::int(3), Value::nil())
        .unwrap();
    let p2 = heap.alloc_pair(&mut mem, Value::int(2), p3).unwrap();
    let p1 = heap.alloc_pair(&mut mem, Value::int(1), p2).unwrap();

    assert_eq!(print_to_string(p1, &heap, &mem), "(1 2 3)");
}

#[test]
fn print_singleton_list() {
    let (mut mem, mut heap) = setup();

    // (42)
    let p = heap
        .alloc_pair(&mut mem, Value::int(42), Value::nil())
        .unwrap();
    assert_eq!(print_to_string(p, &heap, &mem), "(42)");
}

#[test]
fn print_nested_list() {
    let (mut mem, mut heap) = setup();

    // ((1 2) 3)
    let inner2 = heap
        .alloc_pair(&mut mem, Value::int(2), Value::nil())
        .unwrap();
    let inner1 = heap.alloc_pair(&mut mem, Value::int(1), inner2).unwrap();
    let outer2 = heap
        .alloc_pair(&mut mem, Value::int(3), Value::nil())
        .unwrap();
    let outer1 = heap.alloc_pair(&mut mem, inner1, outer2).unwrap();

    assert_eq!(print_to_string(outer1, &heap, &mem), "((1 2) 3)");
}

#[test]
fn print_mixed_list() {
    let (mut mem, mut heap) = setup();

    // (1 "two" nil true)
    let s = heap.alloc_string(&mut mem, "two").unwrap();
    let p4 = heap
        .alloc_pair(&mut mem, Value::bool(true), Value::nil())
        .unwrap();
    let p3 = heap.alloc_pair(&mut mem, Value::nil(), p4).unwrap();
    let p2 = heap.alloc_pair(&mut mem, s, p3).unwrap();
    let p1 = heap.alloc_pair(&mut mem, Value::int(1), p2).unwrap();

    assert_eq!(print_to_string(p1, &heap, &mem), "(1 \"two\" nil true)");
}

#[test]
fn print_improper_list() {
    let (mut mem, mut heap) = setup();

    // (1 . 2) - pair with non-nil rest that isn't a pair
    let p = heap
        .alloc_pair(&mut mem, Value::int(1), Value::int(2))
        .unwrap();
    assert_eq!(print_to_string(p, &heap, &mem), "(1 . 2)");
}
