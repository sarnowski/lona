// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for string intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::value::Value;

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
