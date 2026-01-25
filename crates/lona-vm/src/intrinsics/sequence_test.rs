// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for sequence intrinsics (first, rest, empty?).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::similar_names)]

use super::arithmetic_test::setup;
use super::*;
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

// --- first tests ---

#[test]
fn first_of_nil() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = Term::NIL;

    call_intrinsic(id::FIRST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::NIL);
}

#[test]
fn first_of_list() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Create list (1 2 3)
    let p3 = proc.alloc_term_pair(&mut mem, int(3), Term::NIL).unwrap();
    let p2 = proc.alloc_term_pair(&mut mem, int(2), p3).unwrap();
    let p1 = proc.alloc_term_pair(&mut mem, int(1), p2).unwrap();

    x_regs[1] = p1;
    call_intrinsic(id::FIRST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(1));
}

#[test]
fn first_of_tuple() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let t = proc
        .alloc_term_tuple(&mut mem, &[int(10), int(20)])
        .unwrap();

    x_regs[1] = t;
    call_intrinsic(id::FIRST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(10));
}

#[test]
fn first_of_empty_tuple() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let t = proc.alloc_term_tuple(&mut mem, &[]).unwrap();

    x_regs[1] = t;
    call_intrinsic(id::FIRST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::NIL);
}

#[test]
fn first_of_vector() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let v = proc
        .alloc_term_vector(&mut mem, &[int(100), int(200)])
        .unwrap();

    x_regs[1] = v;
    call_intrinsic(id::FIRST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(100));
}

#[test]
fn first_of_map() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Create map %{:a 1}
    let ka = proc.alloc_term_string(&mut mem, "a").unwrap();
    let entry = proc.alloc_term_tuple(&mut mem, &[ka, int(1)]).unwrap();
    let entries = proc.alloc_term_pair(&mut mem, entry, Term::NIL).unwrap();
    let m = proc.alloc_term_map(&mut mem, entries, 1).unwrap();

    x_regs[1] = m;
    call_intrinsic(id::FIRST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    // Result should be [:a 1] tuple
    let result = x_regs[0];
    let elem0 = proc.read_term_tuple_element(&mem, result, 0);
    let elem1 = proc.read_term_tuple_element(&mem, result, 1);
    assert!(elem0.is_some());
    assert_eq!(elem1, Some(int(1)));
}

// --- rest tests ---

#[test]
fn rest_of_nil() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = Term::NIL;

    call_intrinsic(id::REST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::NIL);
}

#[test]
fn rest_of_list() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Create list (1 2 3)
    let p3 = proc.alloc_term_pair(&mut mem, int(3), Term::NIL).unwrap();
    let p2 = proc.alloc_term_pair(&mut mem, int(2), p3).unwrap();
    let p1 = proc.alloc_term_pair(&mut mem, int(1), p2).unwrap();

    x_regs[1] = p1;
    call_intrinsic(id::REST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    // Result should be (2 3)
    let result = x_regs[0];
    assert!(result.is_list());
    let (head, _) = proc.read_term_pair(&mem, result).unwrap();
    assert_eq!(head, int(2));
}

#[test]
fn rest_of_single_element_list() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let p1 = proc.alloc_term_pair(&mut mem, int(1), Term::NIL).unwrap();

    x_regs[1] = p1;
    call_intrinsic(id::REST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::NIL);
}

#[test]
fn rest_of_tuple() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let t = proc
        .alloc_term_tuple(&mut mem, &[int(1), int(2), int(3)])
        .unwrap();

    x_regs[1] = t;
    call_intrinsic(id::REST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    // Result should be list (2 3)
    let result = x_regs[0];
    assert!(result.is_list());

    let (head1, rest1) = proc.read_term_pair(&mem, result).unwrap();
    assert_eq!(head1, int(2));

    let (head2, rest2) = proc.read_term_pair(&mem, rest1).unwrap();
    assert_eq!(head2, int(3));
    assert_eq!(rest2, Term::NIL);
}

#[test]
fn rest_of_vector() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let v = proc
        .alloc_term_vector(&mut mem, &[int(10), int(20)])
        .unwrap();

    x_regs[1] = v;
    call_intrinsic(id::REST, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    // Result should be list (20)
    let result = x_regs[0];
    assert!(result.is_list());

    let (head, rest) = proc.read_term_pair(&mem, result).unwrap();
    assert_eq!(head, int(20));
    assert_eq!(rest, Term::NIL);
}

// --- empty? tests ---

#[test]
fn is_empty_nil() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = Term::NIL;

    call_intrinsic(
        id::IS_EMPTY,
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
fn is_empty_list() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let p1 = proc.alloc_term_pair(&mut mem, int(1), Term::NIL).unwrap();

    x_regs[1] = p1;
    call_intrinsic(
        id::IS_EMPTY,
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
fn is_empty_tuple() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let empty = proc.alloc_term_tuple(&mut mem, &[]).unwrap();
    x_regs[1] = empty;
    call_intrinsic(
        id::IS_EMPTY,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    let non_empty = proc.alloc_term_tuple(&mut mem, &[int(1)]).unwrap();
    x_regs[1] = non_empty;
    call_intrinsic(
        id::IS_EMPTY,
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
fn is_empty_vector() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let empty = proc.alloc_term_vector(&mut mem, &[]).unwrap();
    x_regs[1] = empty;
    call_intrinsic(
        id::IS_EMPTY,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    let non_empty = proc.alloc_term_vector(&mut mem, &[int(1)]).unwrap();
    x_regs[1] = non_empty;
    call_intrinsic(
        id::IS_EMPTY,
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
fn is_empty_map() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let empty = proc.alloc_term_map(&mut mem, Term::NIL, 0).unwrap();
    x_regs[1] = empty;
    call_intrinsic(
        id::IS_EMPTY,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    let ka = proc.alloc_term_string(&mut mem, "a").unwrap();
    let entry = proc.alloc_term_tuple(&mut mem, &[ka, int(1)]).unwrap();
    let entries = proc.alloc_term_pair(&mut mem, entry, Term::NIL).unwrap();
    let non_empty = proc.alloc_term_map(&mut mem, entries, 1).unwrap();
    x_regs[1] = non_empty;
    call_intrinsic(
        id::IS_EMPTY,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}
