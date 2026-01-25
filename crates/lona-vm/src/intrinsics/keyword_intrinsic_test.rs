// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for keyword intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

#[test]
fn is_keyword_true() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = realm.intern_keyword(&mut mem, "foo").unwrap();
    x_regs[1] = kw;
    call_intrinsic(
        id::IS_KEYWORD,
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
fn is_keyword_false() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(42);
    call_intrinsic(
        id::IS_KEYWORD,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::FALSE);

    // Symbol is not a keyword
    let sym = realm.intern_symbol(&mut mem, "foo").unwrap();
    x_regs[1] = sym;
    call_intrinsic(
        id::IS_KEYWORD,
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
fn keyword_constructor() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let s = proc.alloc_term_string(&mut mem, "bar").unwrap();
    x_regs[1] = s;
    call_intrinsic(id::KEYWORD, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let kw = x_regs[0];
    // The result should be a keyword - check via is_keyword? intrinsic
    x_regs[1] = kw;
    call_intrinsic(
        id::IS_KEYWORD,
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
fn name_keyword() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = realm.intern_keyword(&mut mem, "hello").unwrap();
    x_regs[1] = kw;
    call_intrinsic(id::NAME, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let name = x_regs[0];
    // Result should be a string "hello"
    let name_str = proc.read_term_string(&mem, name).unwrap();
    assert_eq!(name_str, "hello");
}

#[test]
fn name_keyword_qualified() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = realm.intern_keyword(&mut mem, "ns/hello").unwrap();
    x_regs[1] = kw;
    call_intrinsic(id::NAME, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let name = x_regs[0];
    // Result should be just the name part "hello"
    let name_str = proc.read_term_string(&mem, name).unwrap();
    assert_eq!(name_str, "hello");
}

#[test]
fn name_symbol() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let sym = realm.intern_symbol(&mut mem, "world").unwrap();
    x_regs[1] = sym;
    call_intrinsic(id::NAME, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let name = x_regs[0];
    let name_str = proc.read_term_string(&mem, name).unwrap();
    assert_eq!(name_str, "world");
}

#[test]
fn namespace_keyword_qualified() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = realm.intern_keyword(&mut mem, "ns/hello").unwrap();
    x_regs[1] = kw;
    call_intrinsic(
        id::NAMESPACE,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();

    let ns = x_regs[0];
    let ns_str = proc.read_term_string(&mem, ns).unwrap();
    assert_eq!(ns_str, "ns");
}

#[test]
fn namespace_keyword_unqualified() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = realm.intern_keyword(&mut mem, "hello").unwrap();
    x_regs[1] = kw;
    call_intrinsic(
        id::NAMESPACE,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();

    assert!(x_regs[0].is_nil());
}

#[test]
fn keyword_equality() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Same keywords should be equal (interned, same index)
    let k1 = realm.intern_keyword(&mut mem, "foo").unwrap();
    let k2 = realm.intern_keyword(&mut mem, "foo").unwrap();

    // Since keywords are interned, they should be identical
    assert_eq!(k1, k2);

    x_regs[1] = k1;
    x_regs[2] = k2;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    // Different keywords should not be equal
    let k3 = realm.intern_keyword(&mut mem, "bar").unwrap();
    x_regs[1] = k1;
    x_regs[2] = k3;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}
