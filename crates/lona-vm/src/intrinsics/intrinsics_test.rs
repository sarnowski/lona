// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for intrinsic functions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;

/// Create a test environment with process and memory.
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
    let (mut proc, mut mem) = setup();
    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(3);

    call_intrinsic(id::ADD, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(5));
}

#[test]
fn add_negative() {
    let (mut proc, mut mem) = setup();
    proc.x_regs[1] = Value::int(-10);
    proc.x_regs[2] = Value::int(7);

    call_intrinsic(id::ADD, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(-3));
}

#[test]
fn sub_basic() {
    let (mut proc, mut mem) = setup();
    proc.x_regs[1] = Value::int(10);
    proc.x_regs[2] = Value::int(3);

    call_intrinsic(id::SUB, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(7));
}

#[test]
fn mul_basic() {
    let (mut proc, mut mem) = setup();
    proc.x_regs[1] = Value::int(6);
    proc.x_regs[2] = Value::int(7);

    call_intrinsic(id::MUL, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(42));
}

#[test]
fn div_basic() {
    let (mut proc, mut mem) = setup();
    proc.x_regs[1] = Value::int(20);
    proc.x_regs[2] = Value::int(4);

    call_intrinsic(id::DIV, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(5));
}

#[test]
fn div_by_zero() {
    let (mut proc, mut mem) = setup();
    proc.x_regs[1] = Value::int(10);
    proc.x_regs[2] = Value::int(0);

    let result = call_intrinsic(id::DIV, 2, &mut proc, &mut mem);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn mod_basic() {
    let (mut proc, mut mem) = setup();
    proc.x_regs[1] = Value::int(17);
    proc.x_regs[2] = Value::int(5);

    call_intrinsic(id::MOD, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(2));
}

#[test]
fn mod_by_zero() {
    let (mut proc, mut mem) = setup();
    proc.x_regs[1] = Value::int(10);
    proc.x_regs[2] = Value::int(0);

    let result = call_intrinsic(id::MOD, 2, &mut proc, &mut mem);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn arithmetic_type_error() {
    let (mut proc, mut mem) = setup();
    proc.x_regs[1] = Value::bool(true); // Wrong type
    proc.x_regs[2] = Value::int(5);

    let result = call_intrinsic(id::ADD, 2, &mut proc, &mut mem);
    assert!(matches!(result, Err(IntrinsicError::TypeError { .. })));
}

// --- Comparison tests ---

#[test]
fn eq_integers() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(42);
    proc.x_regs[2] = Value::int(42);
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_strings() {
    let (mut proc, mut mem) = setup();

    let s1 = proc.alloc_string(&mut mem, "hello").unwrap();
    let s2 = proc.alloc_string(&mut mem, "hello").unwrap();
    let s3 = proc.alloc_string(&mut mem, "world").unwrap();

    // Same content = equal
    proc.x_regs[1] = s1;
    proc.x_regs[2] = s2;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // Different content = not equal
    proc.x_regs[1] = s1;
    proc.x_regs[2] = s3;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_different_types() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::bool(true);
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn lt_basic() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LT, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(1);
    call_intrinsic(id::LT, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LT, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn le_basic() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(3);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn gt_basic() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(5);
    proc.x_regs[2] = Value::int(3);
    call_intrinsic(id::GT, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn ge_basic() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(5);
    proc.x_regs[2] = Value::int(5);
    call_intrinsic(id::GE, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(5);
    proc.x_regs[2] = Value::int(6);
    call_intrinsic(id::GE, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

// --- Boolean tests ---

#[test]
fn not_basic() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::bool(true);
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    proc.x_regs[1] = Value::bool(false);
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn not_nil() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::nil();
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true)); // nil is falsy
}

#[test]
fn not_int() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(0);
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false)); // 0 is truthy (not nil/false)

    proc.x_regs[1] = Value::int(42);
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

// --- Type predicate tests ---

#[test]
fn is_nil() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::nil();
    call_intrinsic(id::IS_NIL, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(0);
    call_intrinsic(id::IS_NIL, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    proc.x_regs[1] = Value::bool(false);
    call_intrinsic(id::IS_NIL, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn is_int() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(42);
    call_intrinsic(id::IS_INT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::nil();
    call_intrinsic(id::IS_INT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn is_str() {
    let (mut proc, mut mem) = setup();

    let s = proc.alloc_string(&mut mem, "hello").unwrap();
    proc.x_regs[1] = s;
    call_intrinsic(id::IS_STR, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(42);
    call_intrinsic(id::IS_STR, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

// --- String concatenation tests ---

#[test]
fn str_single_string() {
    let (mut proc, mut mem) = setup();

    let s = proc.alloc_string(&mut mem, "hello").unwrap();
    proc.x_regs[1] = s;

    call_intrinsic(id::STR, 1, &mut proc, &mut mem).unwrap();

    let result = proc.read_string(&mem, proc.x_regs[0]).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn str_concatenation() {
    let (mut proc, mut mem) = setup();

    let s1 = proc.alloc_string(&mut mem, "hello").unwrap();
    let s2 = proc.alloc_string(&mut mem, " ").unwrap();
    let s3 = proc.alloc_string(&mut mem, "world").unwrap();

    proc.x_regs[1] = s1;
    proc.x_regs[2] = s2;
    proc.x_regs[3] = s3;

    call_intrinsic(id::STR, 3, &mut proc, &mut mem).unwrap();

    let result = proc.read_string(&mem, proc.x_regs[0]).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn str_mixed_types() {
    let (mut proc, mut mem) = setup();

    let s = proc.alloc_string(&mut mem, "x=").unwrap();
    proc.x_regs[1] = s;
    proc.x_regs[2] = Value::int(42);

    call_intrinsic(id::STR, 2, &mut proc, &mut mem).unwrap();

    let result = proc.read_string(&mem, proc.x_regs[0]).unwrap();
    assert_eq!(result, "x=42");
}

#[test]
fn str_nil_and_bool() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::nil();
    proc.x_regs[2] = Value::bool(true);
    proc.x_regs[3] = Value::bool(false);

    call_intrinsic(id::STR, 3, &mut proc, &mut mem).unwrap();

    let result = proc.read_string(&mem, proc.x_regs[0]).unwrap();
    assert_eq!(result, "niltruefalse");
}

#[test]
fn str_negative_int() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(-12345);

    call_intrinsic(id::STR, 1, &mut proc, &mut mem).unwrap();

    let result = proc.read_string(&mem, proc.x_regs[0]).unwrap();
    assert_eq!(result, "-12345");
}

#[test]
fn str_zero() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(0);

    call_intrinsic(id::STR, 1, &mut proc, &mut mem).unwrap();

    let result = proc.read_string(&mem, proc.x_regs[0]).unwrap();
    assert_eq!(result, "0");
}

// --- Error handling ---

#[test]
fn unknown_intrinsic() {
    let (mut proc, mut mem) = setup();

    let result = call_intrinsic(200, 0, &mut proc, &mut mem);
    assert_eq!(result, Err(IntrinsicError::UnknownIntrinsic(200)));
}

// --- Keyword intrinsic tests ---

#[test]
fn is_keyword_true() {
    let (mut proc, mut mem) = setup();

    let kw = proc.alloc_keyword(&mut mem, "foo").unwrap();
    proc.x_regs[1] = kw;
    call_intrinsic(id::IS_KEYWORD, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn is_keyword_false() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(42);
    call_intrinsic(id::IS_KEYWORD, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    // Symbol is not a keyword
    let sym = proc.alloc_symbol(&mut mem, "foo").unwrap();
    proc.x_regs[1] = sym;
    call_intrinsic(id::IS_KEYWORD, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn keyword_constructor() {
    let (mut proc, mut mem) = setup();

    let s = proc.alloc_string(&mut mem, "bar").unwrap();
    proc.x_regs[1] = s;
    call_intrinsic(id::KEYWORD, 1, &mut proc, &mut mem).unwrap();

    let kw = proc.x_regs[0];
    assert!(kw.is_keyword());
    assert_eq!(proc.read_string(&mem, kw).unwrap(), "bar");
}

#[test]
fn name_keyword() {
    let (mut proc, mut mem) = setup();

    let kw = proc.alloc_keyword(&mut mem, "hello").unwrap();
    proc.x_regs[1] = kw;
    call_intrinsic(id::NAME, 1, &mut proc, &mut mem).unwrap();

    let name = proc.x_regs[0];
    assert!(name.is_string());
    assert_eq!(proc.read_string(&mem, name).unwrap(), "hello");
}

#[test]
fn name_keyword_qualified() {
    let (mut proc, mut mem) = setup();

    let kw = proc.alloc_keyword(&mut mem, "ns/hello").unwrap();
    proc.x_regs[1] = kw;
    call_intrinsic(id::NAME, 1, &mut proc, &mut mem).unwrap();

    let name = proc.x_regs[0];
    assert!(name.is_string());
    assert_eq!(proc.read_string(&mem, name).unwrap(), "hello");
}

#[test]
fn name_symbol() {
    let (mut proc, mut mem) = setup();

    let sym = proc.alloc_symbol(&mut mem, "world").unwrap();
    proc.x_regs[1] = sym;
    call_intrinsic(id::NAME, 1, &mut proc, &mut mem).unwrap();

    let name = proc.x_regs[0];
    assert!(name.is_string());
    assert_eq!(proc.read_string(&mem, name).unwrap(), "world");
}

#[test]
fn namespace_keyword_qualified() {
    let (mut proc, mut mem) = setup();

    let kw = proc.alloc_keyword(&mut mem, "ns/hello").unwrap();
    proc.x_regs[1] = kw;
    call_intrinsic(id::NAMESPACE, 1, &mut proc, &mut mem).unwrap();

    let ns = proc.x_regs[0];
    assert!(ns.is_string());
    assert_eq!(proc.read_string(&mem, ns).unwrap(), "ns");
}

#[test]
fn namespace_keyword_unqualified() {
    let (mut proc, mut mem) = setup();

    let kw = proc.alloc_keyword(&mut mem, "hello").unwrap();
    proc.x_regs[1] = kw;
    call_intrinsic(id::NAMESPACE, 1, &mut proc, &mut mem).unwrap();

    assert!(proc.x_regs[0].is_nil());
}

#[test]
fn keyword_equality() {
    let (mut proc, mut mem) = setup();

    // Due to interning, same keyword literals should be equal
    let k1 = proc.alloc_keyword(&mut mem, "foo").unwrap();
    let k2 = proc.alloc_keyword(&mut mem, "foo").unwrap();

    proc.x_regs[1] = k1;
    proc.x_regs[2] = k2;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // Different keywords should not be equal
    let k3 = proc.alloc_keyword(&mut mem, "bar").unwrap();
    proc.x_regs[1] = k1;
    proc.x_regs[2] = k3;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

// --- Tuple intrinsic tests ---

#[test]
fn is_tuple_true() {
    let (mut proc, mut mem) = setup();

    let tuple = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2)])
        .unwrap();
    proc.x_regs[1] = tuple;
    call_intrinsic(id::IS_TUPLE, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn is_tuple_false() {
    let (mut proc, mut mem) = setup();

    proc.x_regs[1] = Value::int(42);
    call_intrinsic(id::IS_TUPLE, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    // List is not a tuple
    let pair = proc
        .alloc_pair(&mut mem, Value::int(1), Value::nil())
        .unwrap();
    proc.x_regs[1] = pair;
    call_intrinsic(id::IS_TUPLE, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn nth_basic() {
    let (mut proc, mut mem) = setup();

    let tuple = proc
        .alloc_tuple(&mut mem, &[Value::int(10), Value::int(20), Value::int(30)])
        .unwrap();
    proc.x_regs[1] = tuple;
    proc.x_regs[2] = Value::int(0);
    call_intrinsic(id::NTH, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(10));

    proc.x_regs[2] = Value::int(1);
    call_intrinsic(id::NTH, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(20));

    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::NTH, 2, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(30));
}

#[test]
fn nth_out_of_bounds() {
    let (mut proc, mut mem) = setup();

    let tuple = proc.alloc_tuple(&mut mem, &[Value::int(1)]).unwrap();
    proc.x_regs[1] = tuple;
    proc.x_regs[2] = Value::int(5); // Out of bounds

    let result = call_intrinsic(id::NTH, 2, &mut proc, &mut mem);
    assert!(result.is_err());
}

#[test]
fn count_tuple() {
    let (mut proc, mut mem) = setup();

    let tuple = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2), Value::int(3)])
        .unwrap();
    proc.x_regs[1] = tuple;
    call_intrinsic(id::COUNT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(3));
}

#[test]
fn count_empty_tuple() {
    let (mut proc, mut mem) = setup();

    let tuple = proc.alloc_tuple(&mut mem, &[]).unwrap();
    proc.x_regs[1] = tuple;
    call_intrinsic(id::COUNT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(0));
}

#[test]
fn count_list() {
    let (mut proc, mut mem) = setup();

    // Create list (1 2 3)
    let p3 = proc
        .alloc_pair(&mut mem, Value::int(3), Value::nil())
        .unwrap();
    let p2 = proc.alloc_pair(&mut mem, Value::int(2), p3).unwrap();
    let p1 = proc.alloc_pair(&mut mem, Value::int(1), p2).unwrap();

    proc.x_regs[1] = p1;
    call_intrinsic(id::COUNT, 1, &mut proc, &mut mem).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(3));
}
