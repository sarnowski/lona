// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for arithmetic and comparison intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::value::Value;

/// Create a test environment with process, memory, and realm.
pub(super) fn setup() -> (Process, MockVSpace, Realm) {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let proc = Process::new(1, young_base, young_size, old_base, old_size);
    let realm_base = base.add(128 * 1024);
    let realm = Realm::new(realm_base, 64 * 1024);
    (proc, mem, realm)
}

// --- Arithmetic tests ---

#[test]
fn add_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(3);

    call_intrinsic(id::ADD, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(5));
}

#[test]
fn add_negative() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(-10);
    proc.x_regs[2] = Value::int(7);

    call_intrinsic(id::ADD, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(-3));
}

#[test]
fn sub_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(10);
    proc.x_regs[2] = Value::int(3);

    call_intrinsic(id::SUB, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(7));
}

#[test]
fn mul_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(6);
    proc.x_regs[2] = Value::int(7);

    call_intrinsic(id::MUL, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(42));
}

#[test]
fn div_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(20);
    proc.x_regs[2] = Value::int(4);

    call_intrinsic(id::DIV, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(5));
}

#[test]
fn div_by_zero() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(10);
    proc.x_regs[2] = Value::int(0);

    let result = call_intrinsic(id::DIV, 2, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn mod_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(17);
    proc.x_regs[2] = Value::int(5);

    call_intrinsic(id::MOD, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(2));
}

#[test]
fn mod_by_zero() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(10);
    proc.x_regs[2] = Value::int(0);

    let result = call_intrinsic(id::MOD, 2, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn arithmetic_type_error() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::bool(true); // Wrong type
    proc.x_regs[2] = Value::int(5);

    let result = call_intrinsic(id::ADD, 2, &mut proc, &mut mem, &mut realm);
    assert!(matches!(result, Err(IntrinsicError::TypeError { .. })));
}

// --- Comparison tests ---

#[test]
fn eq_integers() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(42);
    proc.x_regs[2] = Value::int(42);
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_strings() {
    let (mut proc, mut mem, mut realm) = setup();

    let s1 = proc.alloc_string(&mut mem, "hello").unwrap();
    let s2 = proc.alloc_string(&mut mem, "hello").unwrap();
    let s3 = proc.alloc_string(&mut mem, "world").unwrap();

    // Same content = equal
    proc.x_regs[1] = s1;
    proc.x_regs[2] = s2;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // Different content = not equal
    proc.x_regs[1] = s1;
    proc.x_regs[2] = s3;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_different_types() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::bool(true);
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn lt_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LT, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(1);
    call_intrinsic(id::LT, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LT, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn le_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(3);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn gt_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(5);
    proc.x_regs[2] = Value::int(3);
    call_intrinsic(id::GT, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn ge_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(5);
    proc.x_regs[2] = Value::int(5);
    call_intrinsic(id::GE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(5);
    proc.x_regs[2] = Value::int(6);
    call_intrinsic(id::GE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}
