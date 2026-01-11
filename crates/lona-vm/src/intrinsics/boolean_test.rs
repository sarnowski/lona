// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for boolean intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::value::Value;

#[test]
fn not_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::bool(true);
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    proc.x_regs[1] = Value::bool(false);
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn not_nil() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::nil();
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true)); // nil is falsy
}

#[test]
fn not_int() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(0);
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false)); // 0 is truthy (not nil/false)

    proc.x_regs[1] = Value::int(42);
    call_intrinsic(id::NOT, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}
