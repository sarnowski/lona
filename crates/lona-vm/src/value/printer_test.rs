// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the value printer.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::Value;
use super::printer::print_to_string;
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;

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
fn print_nil() {
    let (proc, mem) = setup();
    assert_eq!(print_to_string(Value::nil(), &proc, &mem), "nil");
}

#[test]
fn print_booleans() {
    let (proc, mem) = setup();
    assert_eq!(print_to_string(Value::bool(true), &proc, &mem), "true");
    assert_eq!(print_to_string(Value::bool(false), &proc, &mem), "false");
}

#[test]
fn print_integers() {
    let (proc, mem) = setup();
    assert_eq!(print_to_string(Value::int(0), &proc, &mem), "0");
    assert_eq!(print_to_string(Value::int(42), &proc, &mem), "42");
    assert_eq!(print_to_string(Value::int(-1), &proc, &mem), "-1");
    assert_eq!(
        print_to_string(Value::int(123_456_789), &proc, &mem),
        "123456789"
    );
    assert_eq!(
        print_to_string(Value::int(i64::MIN), &proc, &mem),
        "-9223372036854775808"
    );
}

#[test]
fn print_strings() {
    let (mut proc, mut mem) = setup();

    let s = proc.alloc_string(&mut mem, "hello").unwrap();
    assert_eq!(print_to_string(s, &proc, &mem), "\"hello\"");

    let empty = proc.alloc_string(&mut mem, "").unwrap();
    assert_eq!(print_to_string(empty, &proc, &mem), "\"\"");
}

#[test]
fn print_string_escapes() {
    let (mut proc, mut mem) = setup();

    let s = proc.alloc_string(&mut mem, "a\nb").unwrap();
    assert_eq!(print_to_string(s, &proc, &mem), "\"a\\nb\"");

    let s2 = proc.alloc_string(&mut mem, "tab\there").unwrap();
    assert_eq!(print_to_string(s2, &proc, &mem), "\"tab\\there\"");

    let s3 = proc.alloc_string(&mut mem, "quote: \"hi\"").unwrap();
    assert_eq!(print_to_string(s3, &proc, &mem), "\"quote: \\\"hi\\\"\"");
}

#[test]
fn print_symbols() {
    let (mut proc, mut mem) = setup();

    let sym = proc.alloc_symbol(&mut mem, "foo").unwrap();
    assert_eq!(print_to_string(sym, &proc, &mem), "foo");
}

#[test]
fn print_empty_list() {
    let (proc, mem) = setup();
    // Empty list is just nil
    assert_eq!(print_to_string(Value::nil(), &proc, &mem), "nil");
}

#[test]
fn print_list() {
    let (mut proc, mut mem) = setup();

    // (1 2 3)
    let p3 = proc
        .alloc_pair(&mut mem, Value::int(3), Value::nil())
        .unwrap();
    let p2 = proc.alloc_pair(&mut mem, Value::int(2), p3).unwrap();
    let p1 = proc.alloc_pair(&mut mem, Value::int(1), p2).unwrap();

    assert_eq!(print_to_string(p1, &proc, &mem), "(1 2 3)");
}

#[test]
fn print_singleton_list() {
    let (mut proc, mut mem) = setup();

    // (42)
    let p = proc
        .alloc_pair(&mut mem, Value::int(42), Value::nil())
        .unwrap();
    assert_eq!(print_to_string(p, &proc, &mem), "(42)");
}

#[test]
fn print_nested_list() {
    let (mut proc, mut mem) = setup();

    // ((1 2) 3)
    let inner2 = proc
        .alloc_pair(&mut mem, Value::int(2), Value::nil())
        .unwrap();
    let inner1 = proc.alloc_pair(&mut mem, Value::int(1), inner2).unwrap();
    let outer2 = proc
        .alloc_pair(&mut mem, Value::int(3), Value::nil())
        .unwrap();
    let outer1 = proc.alloc_pair(&mut mem, inner1, outer2).unwrap();

    assert_eq!(print_to_string(outer1, &proc, &mem), "((1 2) 3)");
}

#[test]
fn print_mixed_list() {
    let (mut proc, mut mem) = setup();

    // (1 "two" nil true)
    let s = proc.alloc_string(&mut mem, "two").unwrap();
    let p4 = proc
        .alloc_pair(&mut mem, Value::bool(true), Value::nil())
        .unwrap();
    let p3 = proc.alloc_pair(&mut mem, Value::nil(), p4).unwrap();
    let p2 = proc.alloc_pair(&mut mem, s, p3).unwrap();
    let p1 = proc.alloc_pair(&mut mem, Value::int(1), p2).unwrap();

    assert_eq!(print_to_string(p1, &proc, &mem), "(1 \"two\" nil true)");
}

#[test]
fn print_improper_list() {
    let (mut proc, mut mem) = setup();

    // (1 . 2) - pair with non-nil rest that isn't a pair
    let p = proc
        .alloc_pair(&mut mem, Value::int(1), Value::int(2))
        .unwrap();
    assert_eq!(print_to_string(p, &proc, &mem), "(1 . 2)");
}

// --- Keyword printer tests ---

#[test]
fn print_keyword_simple() {
    let (mut proc, mut mem) = setup();
    let kw = proc.alloc_keyword(&mut mem, "foo").unwrap();
    assert_eq!(print_to_string(kw, &proc, &mem), ":foo");
}

#[test]
fn print_keyword_qualified() {
    let (mut proc, mut mem) = setup();
    let kw = proc.alloc_keyword(&mut mem, "ns/bar").unwrap();
    assert_eq!(print_to_string(kw, &proc, &mem), ":ns/bar");
}

#[test]
fn print_keyword_in_list() {
    let (mut proc, mut mem) = setup();
    let k1 = proc.alloc_keyword(&mut mem, "a").unwrap();
    let k2 = proc.alloc_keyword(&mut mem, "b").unwrap();
    let p2 = proc.alloc_pair(&mut mem, k2, Value::nil()).unwrap();
    let p1 = proc.alloc_pair(&mut mem, k1, p2).unwrap();
    assert_eq!(print_to_string(p1, &proc, &mem), "(:a :b)");
}

// --- Tuple printer tests ---

#[test]
fn print_tuple_empty() {
    let (mut proc, mut mem) = setup();
    let tuple = proc.alloc_tuple(&mut mem, &[]).unwrap();
    assert_eq!(print_to_string(tuple, &proc, &mem), "[]");
}

#[test]
fn print_tuple_simple() {
    let (mut proc, mut mem) = setup();
    let tuple = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2), Value::int(3)])
        .unwrap();
    assert_eq!(print_to_string(tuple, &proc, &mem), "[1 2 3]");
}

#[test]
fn print_tuple_nested() {
    let (mut proc, mut mem) = setup();
    let inner = proc.alloc_tuple(&mut mem, &[Value::int(1)]).unwrap();
    let outer = proc.alloc_tuple(&mut mem, &[inner, Value::int(2)]).unwrap();
    assert_eq!(print_to_string(outer, &proc, &mem), "[[1] 2]");
}

#[test]
fn print_tuple_with_keywords() {
    let (mut proc, mut mem) = setup();
    let k1 = proc.alloc_keyword(&mut mem, "a").unwrap();
    let k2 = proc.alloc_keyword(&mut mem, "b").unwrap();
    let tuple = proc.alloc_tuple(&mut mem, &[k1, k2]).unwrap();
    assert_eq!(print_to_string(tuple, &proc, &mem), "[:a :b]");
}

#[test]
fn print_tuple_mixed() {
    let (mut proc, mut mem) = setup();
    let s = proc.alloc_string(&mut mem, "hello").unwrap();
    let tuple = proc
        .alloc_tuple(&mut mem, &[Value::int(1), s, Value::bool(true)])
        .unwrap();
    assert_eq!(print_to_string(tuple, &proc, &mem), "[1 \"hello\" true]");
}
