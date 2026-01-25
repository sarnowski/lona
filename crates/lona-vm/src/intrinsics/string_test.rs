// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for string intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

#[test]
fn str_single_string() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let s = proc.alloc_term_string(&mut mem, "hello").unwrap();
    x_regs[1] = s;

    call_intrinsic(id::STR, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let result = proc.read_term_string(&mem, x_regs[0]).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn str_concatenation() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let s1 = proc.alloc_term_string(&mut mem, "hello").unwrap();
    let s2 = proc.alloc_term_string(&mut mem, " ").unwrap();
    let s3 = proc.alloc_term_string(&mut mem, "world").unwrap();

    x_regs[1] = s1;
    x_regs[2] = s2;
    x_regs[3] = s3;

    call_intrinsic(id::STR, 3, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let result = proc.read_term_string(&mem, x_regs[0]).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn str_mixed_types() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let s = proc.alloc_term_string(&mut mem, "x=").unwrap();
    x_regs[1] = s;
    x_regs[2] = int(42);

    call_intrinsic(id::STR, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let result = proc.read_term_string(&mem, x_regs[0]).unwrap();
    assert_eq!(result, "x=42");
}

#[test]
fn str_nil_and_bool() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = Term::NIL;
    x_regs[2] = Term::TRUE;
    x_regs[3] = Term::FALSE;

    call_intrinsic(id::STR, 3, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let result = proc.read_term_string(&mem, x_regs[0]).unwrap();
    assert_eq!(result, "niltruefalse");
}

#[test]
fn str_negative_int() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(-12345);

    call_intrinsic(id::STR, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let result = proc.read_term_string(&mem, x_regs[0]).unwrap();
    assert_eq!(result, "-12345");
}

#[test]
fn str_zero() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(0);

    call_intrinsic(id::STR, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let result = proc.read_term_string(&mem, x_regs[0]).unwrap();
    assert_eq!(result, "0");
}
