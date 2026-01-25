// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for keyword intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::arithmetic_test::setup;
use super::*;
use crate::value::Value;

#[test]
fn is_keyword_true() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = proc.alloc_keyword(&mut mem, "foo").unwrap();
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
    assert_eq!(x_regs[0], Value::bool(true));
}

#[test]
fn is_keyword_false() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = Value::int(42);
    call_intrinsic(
        id::IS_KEYWORD,
        1,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Value::bool(false));

    // Symbol is not a keyword
    let sym = proc.alloc_symbol(&mut mem, "foo").unwrap();
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
    assert_eq!(x_regs[0], Value::bool(false));
}

#[test]
fn keyword_constructor() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let s = proc.alloc_string(&mut mem, "bar").unwrap();
    x_regs[1] = s;
    call_intrinsic(id::KEYWORD, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let kw = x_regs[0];
    assert!(kw.is_keyword());
    assert_eq!(proc.read_string(&mem, kw).unwrap(), "bar");
}

#[test]
fn name_keyword() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = proc.alloc_keyword(&mut mem, "hello").unwrap();
    x_regs[1] = kw;
    call_intrinsic(id::NAME, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let name = x_regs[0];
    assert!(name.is_string());
    assert_eq!(proc.read_string(&mem, name).unwrap(), "hello");
}

#[test]
fn name_keyword_qualified() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = proc.alloc_keyword(&mut mem, "ns/hello").unwrap();
    x_regs[1] = kw;
    call_intrinsic(id::NAME, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let name = x_regs[0];
    assert!(name.is_string());
    assert_eq!(proc.read_string(&mem, name).unwrap(), "hello");
}

#[test]
fn name_symbol() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let sym = proc.alloc_symbol(&mut mem, "world").unwrap();
    x_regs[1] = sym;
    call_intrinsic(id::NAME, 1, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();

    let name = x_regs[0];
    assert!(name.is_string());
    assert_eq!(proc.read_string(&mem, name).unwrap(), "world");
}

#[test]
fn namespace_keyword_qualified() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = proc.alloc_keyword(&mut mem, "ns/hello").unwrap();
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
    assert!(ns.is_string());
    assert_eq!(proc.read_string(&mem, ns).unwrap(), "ns");
}

#[test]
fn namespace_keyword_unqualified() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let kw = proc.alloc_keyword(&mut mem, "hello").unwrap();
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

    // Due to interning, same keyword literals should be equal
    let k1 = proc.alloc_keyword(&mut mem, "foo").unwrap();
    let k2 = proc.alloc_keyword(&mut mem, "foo").unwrap();

    x_regs[1] = k1;
    x_regs[2] = k2;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Value::bool(true));

    // Different keywords should not be equal
    let k3 = proc.alloc_keyword(&mut mem, "bar").unwrap();
    x_regs[1] = k1;
    x_regs[2] = k3;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Value::bool(false));
}
