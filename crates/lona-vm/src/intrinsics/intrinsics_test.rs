// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for intrinsic functions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::Vaddr;
use crate::heap::Heap;
use crate::platform::MockVSpace;

/// Create a test environment with heap and memory.
fn setup() -> (Heap, MockVSpace) {
    let mem = MockVSpace::new(64 * 1024, Vaddr::new(0x1_0000));
    let heap = Heap::new(Vaddr::new(0x1_0000 + 64 * 1024), 64 * 1024);
    (heap, mem)
}

/// Create a register file initialized to nil.
fn make_regs() -> [Value; 256] {
    [Value::Nil; 256]
}

// --- Lookup tests ---

#[test]
fn lookup_arithmetic() {
    assert_eq!(lookup_intrinsic("+"), Some(id::ADD));
    assert_eq!(lookup_intrinsic("-"), Some(id::SUB));
    assert_eq!(lookup_intrinsic("*"), Some(id::MUL));
    assert_eq!(lookup_intrinsic("/"), Some(id::DIV));
    assert_eq!(lookup_intrinsic("mod"), Some(id::MOD));
}

#[test]
fn lookup_comparison() {
    assert_eq!(lookup_intrinsic("="), Some(id::EQ));
    assert_eq!(lookup_intrinsic("<"), Some(id::LT));
    assert_eq!(lookup_intrinsic(">"), Some(id::GT));
    assert_eq!(lookup_intrinsic("<="), Some(id::LE));
    assert_eq!(lookup_intrinsic(">="), Some(id::GE));
}

#[test]
fn lookup_predicates() {
    assert_eq!(lookup_intrinsic("not"), Some(id::NOT));
    assert_eq!(lookup_intrinsic("nil?"), Some(id::IS_NIL));
    assert_eq!(lookup_intrinsic("integer?"), Some(id::IS_INT));
    assert_eq!(lookup_intrinsic("string?"), Some(id::IS_STR));
}

#[test]
fn lookup_str() {
    assert_eq!(lookup_intrinsic("str"), Some(id::STR));
}

#[test]
fn lookup_unknown() {
    assert_eq!(lookup_intrinsic("unknown"), None);
    assert_eq!(lookup_intrinsic("println"), None);
}

#[test]
fn intrinsic_name_roundtrip() {
    for i in 0..INTRINSIC_COUNT as u8 {
        let name = intrinsic_name(i).unwrap();
        assert_eq!(lookup_intrinsic(name), Some(i));
    }
}

// --- Arithmetic tests ---

#[test]
fn add_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();
    regs[1] = Value::int(2);
    regs[2] = Value::int(3);

    call_intrinsic(id::ADD, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::int(5));
}

#[test]
fn add_negative() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();
    regs[1] = Value::int(-10);
    regs[2] = Value::int(7);

    call_intrinsic(id::ADD, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::int(-3));
}

#[test]
fn sub_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();
    regs[1] = Value::int(10);
    regs[2] = Value::int(3);

    call_intrinsic(id::SUB, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::int(7));
}

#[test]
fn mul_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();
    regs[1] = Value::int(6);
    regs[2] = Value::int(7);

    call_intrinsic(id::MUL, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::int(42));
}

#[test]
fn div_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();
    regs[1] = Value::int(20);
    regs[2] = Value::int(4);

    call_intrinsic(id::DIV, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::int(5));
}

#[test]
fn div_by_zero() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();
    regs[1] = Value::int(10);
    regs[2] = Value::int(0);

    let result = call_intrinsic(id::DIV, 2, &mut regs, &mut heap, &mut mem);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn mod_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();
    regs[1] = Value::int(17);
    regs[2] = Value::int(5);

    call_intrinsic(id::MOD, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::int(2));
}

#[test]
fn mod_by_zero() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();
    regs[1] = Value::int(10);
    regs[2] = Value::int(0);

    let result = call_intrinsic(id::MOD, 2, &mut regs, &mut heap, &mut mem);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn arithmetic_type_error() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();
    regs[1] = Value::bool(true); // Wrong type
    regs[2] = Value::int(5);

    let result = call_intrinsic(id::ADD, 2, &mut regs, &mut heap, &mut mem);
    assert!(matches!(result, Err(IntrinsicError::TypeError { .. })));
}

// --- Comparison tests ---

#[test]
fn eq_integers() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(42);
    regs[2] = Value::int(42);
    call_intrinsic(id::EQ, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));

    regs[1] = Value::int(1);
    regs[2] = Value::int(2);
    call_intrinsic(id::EQ, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

#[test]
fn eq_strings() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    let s1 = heap.alloc_string(&mut mem, "hello").unwrap();
    let s2 = heap.alloc_string(&mut mem, "hello").unwrap();
    let s3 = heap.alloc_string(&mut mem, "world").unwrap();

    // Same content = equal
    regs[1] = s1;
    regs[2] = s2;
    call_intrinsic(id::EQ, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));

    // Different content = not equal
    regs[1] = s1;
    regs[2] = s3;
    call_intrinsic(id::EQ, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

#[test]
fn eq_different_types() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(1);
    regs[2] = Value::bool(true);
    call_intrinsic(id::EQ, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

#[test]
fn lt_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(1);
    regs[2] = Value::int(2);
    call_intrinsic(id::LT, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));

    regs[1] = Value::int(2);
    regs[2] = Value::int(1);
    call_intrinsic(id::LT, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));

    regs[1] = Value::int(2);
    regs[2] = Value::int(2);
    call_intrinsic(id::LT, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

#[test]
fn le_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(1);
    regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));

    regs[1] = Value::int(2);
    regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));

    regs[1] = Value::int(3);
    regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

#[test]
fn gt_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(5);
    regs[2] = Value::int(3);
    call_intrinsic(id::GT, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));
}

#[test]
fn ge_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(5);
    regs[2] = Value::int(5);
    call_intrinsic(id::GE, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));

    regs[1] = Value::int(5);
    regs[2] = Value::int(6);
    call_intrinsic(id::GE, 2, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

// --- Boolean tests ---

#[test]
fn not_basic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::bool(true);
    call_intrinsic(id::NOT, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));

    regs[1] = Value::bool(false);
    call_intrinsic(id::NOT, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));
}

#[test]
fn not_nil() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::nil();
    call_intrinsic(id::NOT, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true)); // nil is falsy
}

#[test]
fn not_int() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(0);
    call_intrinsic(id::NOT, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false)); // 0 is truthy (not nil/false)

    regs[1] = Value::int(42);
    call_intrinsic(id::NOT, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

// --- Type predicate tests ---

#[test]
fn is_nil() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::nil();
    call_intrinsic(id::IS_NIL, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));

    regs[1] = Value::int(0);
    call_intrinsic(id::IS_NIL, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));

    regs[1] = Value::bool(false);
    call_intrinsic(id::IS_NIL, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

#[test]
fn is_int() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(42);
    call_intrinsic(id::IS_INT, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));

    regs[1] = Value::nil();
    call_intrinsic(id::IS_INT, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

#[test]
fn is_str() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    let s = heap.alloc_string(&mut mem, "hello").unwrap();
    regs[1] = s;
    call_intrinsic(id::IS_STR, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(true));

    regs[1] = Value::int(42);
    call_intrinsic(id::IS_STR, 1, &mut regs, &mut heap, &mut mem).unwrap();
    assert_eq!(regs[0], Value::bool(false));
}

// --- String concatenation tests ---

#[test]
fn str_single_string() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    let s = heap.alloc_string(&mut mem, "hello").unwrap();
    regs[1] = s;

    call_intrinsic(id::STR, 1, &mut regs, &mut heap, &mut mem).unwrap();

    let result = heap.read_string(&mem, regs[0]).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn str_concatenation() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    let s1 = heap.alloc_string(&mut mem, "hello").unwrap();
    let s2 = heap.alloc_string(&mut mem, " ").unwrap();
    let s3 = heap.alloc_string(&mut mem, "world").unwrap();

    regs[1] = s1;
    regs[2] = s2;
    regs[3] = s3;

    call_intrinsic(id::STR, 3, &mut regs, &mut heap, &mut mem).unwrap();

    let result = heap.read_string(&mem, regs[0]).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn str_mixed_types() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    let s = heap.alloc_string(&mut mem, "x=").unwrap();
    regs[1] = s;
    regs[2] = Value::int(42);

    call_intrinsic(id::STR, 2, &mut regs, &mut heap, &mut mem).unwrap();

    let result = heap.read_string(&mem, regs[0]).unwrap();
    assert_eq!(result, "x=42");
}

#[test]
fn str_nil_and_bool() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::nil();
    regs[2] = Value::bool(true);
    regs[3] = Value::bool(false);

    call_intrinsic(id::STR, 3, &mut regs, &mut heap, &mut mem).unwrap();

    let result = heap.read_string(&mem, regs[0]).unwrap();
    assert_eq!(result, "niltruefalse");
}

#[test]
fn str_negative_int() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(-12345);

    call_intrinsic(id::STR, 1, &mut regs, &mut heap, &mut mem).unwrap();

    let result = heap.read_string(&mem, regs[0]).unwrap();
    assert_eq!(result, "-12345");
}

#[test]
fn str_zero() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    regs[1] = Value::int(0);

    call_intrinsic(id::STR, 1, &mut regs, &mut heap, &mut mem).unwrap();

    let result = heap.read_string(&mem, regs[0]).unwrap();
    assert_eq!(result, "0");
}

// --- Error handling ---

#[test]
fn unknown_intrinsic() {
    let (mut heap, mut mem) = setup();
    let mut regs = make_regs();

    let result = call_intrinsic(200, 0, &mut regs, &mut heap, &mut mem);
    assert_eq!(result, Err(IntrinsicError::UnknownIntrinsic(200)));
}
