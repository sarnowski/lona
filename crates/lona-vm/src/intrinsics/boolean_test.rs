// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for boolean intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

#[test]
fn not_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = Term::TRUE;
    call_intrinsic(id::NOT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);

    x_regs[1] = Term::FALSE;
    call_intrinsic(id::NOT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);
}

#[test]
fn not_nil() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = Term::NIL;
    call_intrinsic(id::NOT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE); // nil is falsy
}

#[test]
fn not_int() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(0);
    call_intrinsic(id::NOT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE); // 0 is truthy (not nil/false)

    x_regs[1] = int(42);
    call_intrinsic(id::NOT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}
