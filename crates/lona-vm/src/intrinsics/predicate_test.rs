// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for type predicate intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

#[test]
fn is_nil() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = Term::NIL;
    call_intrinsic(id::IS_NIL, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    x_regs[1] = int(0);
    call_intrinsic(id::IS_NIL, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);

    x_regs[1] = Term::FALSE;
    call_intrinsic(id::IS_NIL, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn is_int() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(42);
    call_intrinsic(id::IS_INT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    x_regs[1] = Term::NIL;
    call_intrinsic(id::IS_INT, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn is_str() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let s = proc.alloc_term_string(&mut mem, "hello").unwrap();
    x_regs[1] = s;
    call_intrinsic(id::IS_STR, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    x_regs[1] = int(42);
    call_intrinsic(id::IS_STR, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn unknown_intrinsic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let result = call_intrinsic(200, 0, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::UnknownIntrinsic(200)));
}
