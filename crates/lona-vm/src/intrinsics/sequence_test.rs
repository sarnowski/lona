// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for sequence intrinsics (first, rest, empty?).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::similar_names)]

use super::arithmetic_test::setup;
use super::*;
use crate::value::Value;

// --- first tests ---

#[test]
fn first_of_nil() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::Nil;

    call_intrinsic(id::FIRST, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::Nil);
}

#[test]
fn first_of_list() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create list (1 2 3)
    let p3 = proc
        .alloc_pair(&mut mem, Value::int(3), Value::Nil)
        .unwrap();
    let p2 = proc.alloc_pair(&mut mem, Value::int(2), p3).unwrap();
    let p1 = proc.alloc_pair(&mut mem, Value::int(1), p2).unwrap();

    proc.x_regs[1] = p1;
    call_intrinsic(id::FIRST, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(1));
}

#[test]
fn first_of_tuple() {
    let (mut proc, mut mem, mut realm) = setup();

    let t = proc
        .alloc_tuple(&mut mem, &[Value::int(10), Value::int(20)])
        .unwrap();

    proc.x_regs[1] = t;
    call_intrinsic(id::FIRST, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(10));
}

#[test]
fn first_of_empty_tuple() {
    let (mut proc, mut mem, mut realm) = setup();

    let t = proc.alloc_tuple(&mut mem, &[]).unwrap();

    proc.x_regs[1] = t;
    call_intrinsic(id::FIRST, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::Nil);
}

#[test]
fn first_of_vector() {
    let (mut proc, mut mem, mut realm) = setup();

    let v = proc
        .alloc_vector(&mut mem, &[Value::int(100), Value::int(200)])
        .unwrap();

    proc.x_regs[1] = v;
    call_intrinsic(id::FIRST, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(100));
}

#[test]
fn first_of_map() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create map %{:a 1}
    let ka = proc.alloc_keyword(&mut mem, "a").unwrap();
    let entry = proc.alloc_tuple(&mut mem, &[ka, Value::int(1)]).unwrap();
    let entries = proc.alloc_pair(&mut mem, entry, Value::Nil).unwrap();
    let m = proc.alloc_map(&mut mem, entries).unwrap();

    proc.x_regs[1] = m;
    call_intrinsic(id::FIRST, 1, &mut proc, &mut mem, &mut realm).unwrap();

    // Result should be [:a 1] tuple
    let result = proc.x_regs[0];
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_element(&mem, result, 0), Some(ka));
    assert_eq!(
        proc.read_tuple_element(&mem, result, 1),
        Some(Value::int(1))
    );
}

// --- rest tests ---

#[test]
fn rest_of_nil() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::Nil;

    call_intrinsic(id::REST, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::Nil);
}

#[test]
fn rest_of_list() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create list (1 2 3)
    let p3 = proc
        .alloc_pair(&mut mem, Value::int(3), Value::Nil)
        .unwrap();
    let p2 = proc.alloc_pair(&mut mem, Value::int(2), p3).unwrap();
    let p1 = proc.alloc_pair(&mut mem, Value::int(1), p2).unwrap();

    proc.x_regs[1] = p1;
    call_intrinsic(id::REST, 1, &mut proc, &mut mem, &mut realm).unwrap();

    // Result should be (2 3)
    let result = proc.x_regs[0];
    assert!(result.is_pair());
    let pair = proc.read_pair(&mem, result).unwrap();
    assert_eq!(pair.first, Value::int(2));
}

#[test]
fn rest_of_single_element_list() {
    let (mut proc, mut mem, mut realm) = setup();

    let p1 = proc
        .alloc_pair(&mut mem, Value::int(1), Value::Nil)
        .unwrap();

    proc.x_regs[1] = p1;
    call_intrinsic(id::REST, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::Nil);
}

#[test]
fn rest_of_tuple() {
    let (mut proc, mut mem, mut realm) = setup();

    let t = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2), Value::int(3)])
        .unwrap();

    proc.x_regs[1] = t;
    call_intrinsic(id::REST, 1, &mut proc, &mut mem, &mut realm).unwrap();

    // Result should be list (2 3)
    let result = proc.x_regs[0];
    assert!(result.is_pair());

    let pair1 = proc.read_pair(&mem, result).unwrap();
    assert_eq!(pair1.first, Value::int(2));

    let pair2 = proc.read_pair(&mem, pair1.rest).unwrap();
    assert_eq!(pair2.first, Value::int(3));
    assert_eq!(pair2.rest, Value::Nil);
}

#[test]
fn rest_of_vector() {
    let (mut proc, mut mem, mut realm) = setup();

    let v = proc
        .alloc_vector(&mut mem, &[Value::int(10), Value::int(20)])
        .unwrap();

    proc.x_regs[1] = v;
    call_intrinsic(id::REST, 1, &mut proc, &mut mem, &mut realm).unwrap();

    // Result should be list (20)
    let result = proc.x_regs[0];
    assert!(result.is_pair());

    let pair = proc.read_pair(&mem, result).unwrap();
    assert_eq!(pair.first, Value::int(20));
    assert_eq!(pair.rest, Value::Nil);
}

// --- empty? tests ---

#[test]
fn is_empty_nil() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::Nil;

    call_intrinsic(id::IS_EMPTY, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn is_empty_list() {
    let (mut proc, mut mem, mut realm) = setup();

    let p1 = proc
        .alloc_pair(&mut mem, Value::int(1), Value::Nil)
        .unwrap();

    proc.x_regs[1] = p1;
    call_intrinsic(id::IS_EMPTY, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn is_empty_tuple() {
    let (mut proc, mut mem, mut realm) = setup();

    let empty = proc.alloc_tuple(&mut mem, &[]).unwrap();
    proc.x_regs[1] = empty;
    call_intrinsic(id::IS_EMPTY, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    let non_empty = proc.alloc_tuple(&mut mem, &[Value::int(1)]).unwrap();
    proc.x_regs[1] = non_empty;
    call_intrinsic(id::IS_EMPTY, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn is_empty_vector() {
    let (mut proc, mut mem, mut realm) = setup();

    let empty = proc.alloc_vector(&mut mem, &[]).unwrap();
    proc.x_regs[1] = empty;
    call_intrinsic(id::IS_EMPTY, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    let non_empty = proc.alloc_vector(&mut mem, &[Value::int(1)]).unwrap();
    proc.x_regs[1] = non_empty;
    call_intrinsic(id::IS_EMPTY, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn is_empty_map() {
    let (mut proc, mut mem, mut realm) = setup();

    let empty = proc.alloc_map(&mut mem, Value::Nil).unwrap();
    proc.x_regs[1] = empty;
    call_intrinsic(id::IS_EMPTY, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    let ka = proc.alloc_keyword(&mut mem, "a").unwrap();
    let entry = proc.alloc_tuple(&mut mem, &[ka, Value::int(1)]).unwrap();
    let entries = proc.alloc_pair(&mut mem, entry, Value::Nil).unwrap();
    let non_empty = proc.alloc_map(&mut mem, entries).unwrap();
    proc.x_regs[1] = non_empty;
    call_intrinsic(id::IS_EMPTY, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}
