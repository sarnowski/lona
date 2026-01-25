// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for tuple intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::value::Value;

#[test]
fn is_tuple_true() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2)])
        .unwrap();
    x_regs[1] = tuple;
    call_intrinsic(
        id::IS_TUPLE,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Value::bool(true));
}

#[test]
fn is_tuple_false() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = Value::int(42);
    call_intrinsic(
        id::IS_TUPLE,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Value::bool(false));

    // List is not a tuple
    let pair = proc
        .alloc_pair(&mut mem, Value::int(1), Value::nil())
        .unwrap();
    x_regs[1] = pair;
    call_intrinsic(
        id::IS_TUPLE,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Value::bool(false));
}

#[test]
fn nth_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc
        .alloc_tuple(&mut mem, &[Value::int(10), Value::int(20), Value::int(30)])
        .unwrap();
    x_regs[1] = tuple;
    x_regs[2] = Value::int(0);
    call_intrinsic(id::NTH, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Value::int(10));

    x_regs[2] = Value::int(1);
    call_intrinsic(id::NTH, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Value::int(20));

    x_regs[2] = Value::int(2);
    call_intrinsic(id::NTH, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Value::int(30));
}

#[test]
fn nth_out_of_bounds() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc.alloc_tuple(&mut mem, &[Value::int(1)]).unwrap();
    x_regs[1] = tuple;
    x_regs[2] = Value::int(5); // Out of bounds

    let result = call_intrinsic(id::NTH, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert!(result.is_err());
}

#[test]
fn count_tuple() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2), Value::int(3)])
        .unwrap();
    x_regs[1] = tuple;
    call_intrinsic(id::COUNT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Value::int(3));
}

#[test]
fn count_empty_tuple() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc.alloc_tuple(&mut mem, &[]).unwrap();
    x_regs[1] = tuple;
    call_intrinsic(id::COUNT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Value::int(0));
}

#[test]
fn count_list() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Create list (1 2 3)
    let p3 = proc
        .alloc_pair(&mut mem, Value::int(3), Value::nil())
        .unwrap();
    let p2 = proc.alloc_pair(&mut mem, Value::int(2), p3).unwrap();
    let p1 = proc.alloc_pair(&mut mem, Value::int(1), p2).unwrap();

    x_regs[1] = p1;
    call_intrinsic(id::COUNT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Value::int(3));
}
