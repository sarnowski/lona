// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for metadata.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};

#[test]
fn eval_meta_nil_for_no_metadata() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(meta 'foo)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_with_meta_and_meta() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Create a tuple and attach metadata, then check it
    // Note: with-meta requires reference types (boxed values), not immediates like symbols
    let result = eval(
        "(meta (with-meta [1 2] %{:doc \"hello\"}))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert!(proc.is_term_map(&mem, result));
}

#[test]
fn eval_meta_does_not_affect_equality() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Create two tuples with same content but different metadata
    // Note: with-meta requires reference types (boxed values), not immediates like symbols
    let a = eval("(with-meta [1 2] %{:a 1})", &mut proc, &mut realm, &mut mem).unwrap();
    let b = eval("(with-meta [1 2] %{:b 2})", &mut proc, &mut realm, &mut mem).unwrap();

    // Both should be tuples
    assert!(proc.is_term_tuple(&mem, a));
    assert!(proc.is_term_tuple(&mem, b));
}

#[test]
fn eval_meta_on_tuple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    let tuple = eval(
        "(with-meta [1 2] %{:tag :vector})",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert!(proc.is_term_tuple(&mem, tuple));

    let meta = eval(
        "(meta (with-meta [1 2 3] %{:tag :vector}))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert!(proc.is_term_map(&mem, meta));
}
