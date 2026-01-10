// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for type predicate intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::value::Value;

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

#[test]
fn unknown_intrinsic() {
    let (mut proc, mut mem) = setup();

    let result = call_intrinsic(200, 0, &mut proc, &mut mem);
    assert_eq!(result, Err(IntrinsicError::UnknownIntrinsic(200)));
}
