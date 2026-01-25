// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for tuple intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

#[test]
fn is_tuple_true() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc.alloc_term_tuple(&mut mem, &[int(1), int(2)]).unwrap();
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
    assert_eq!(x_regs[0], Term::TRUE);
}

#[test]
fn is_tuple_false() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(42);
    call_intrinsic(
        id::IS_TUPLE,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::FALSE);

    // List is not a tuple
    let pair = proc.alloc_term_pair(&mut mem, int(1), Term::NIL).unwrap();
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
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn nth_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc
        .alloc_term_tuple(&mut mem, &[int(10), int(20), int(30)])
        .unwrap();
    x_regs[1] = tuple;
    x_regs[2] = int(0);
    call_intrinsic(id::NTH, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(10));

    x_regs[2] = int(1);
    call_intrinsic(id::NTH, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(20));

    x_regs[2] = int(2);
    call_intrinsic(id::NTH, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(30));
}

#[test]
fn nth_out_of_bounds() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc.alloc_term_tuple(&mut mem, &[int(1)]).unwrap();
    x_regs[1] = tuple;
    x_regs[2] = int(5); // Out of bounds

    let result = call_intrinsic(id::NTH, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert!(result.is_err());
}

#[test]
fn count_tuple() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc
        .alloc_term_tuple(&mut mem, &[int(1), int(2), int(3)])
        .unwrap();
    x_regs[1] = tuple;
    call_intrinsic(id::COUNT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(3));
}

#[test]
fn count_empty_tuple() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let tuple = proc.alloc_term_tuple(&mut mem, &[]).unwrap();
    x_regs[1] = tuple;
    call_intrinsic(id::COUNT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(0));
}

#[test]
fn count_list() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Create list (1 2 3)
    let p3 = proc.alloc_term_pair(&mut mem, int(3), Term::NIL).unwrap();
    let p2 = proc.alloc_term_pair(&mut mem, int(2), p3).unwrap();
    let p1 = proc.alloc_term_pair(&mut mem, int(1), p2).unwrap();

    x_regs[1] = p1;
    call_intrinsic(id::COUNT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(3));
}
