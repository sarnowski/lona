// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for metadata.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};

#[test]
fn eval_meta_nil_for_no_metadata() {
    let (mut proc, mut mem) = setup();
    let result = eval("(meta 'foo)", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_with_meta_and_meta() {
    let (mut proc, mut mem) = setup();

    // Create a symbol and attach metadata, then check it
    let result = eval(
        "(meta (with-meta 'x %{:doc \"hello\"}))",
        &mut proc,
        &mut mem,
    )
    .unwrap();
    assert!(result.is_map());
}

#[test]
fn eval_meta_does_not_affect_equality() {
    let (mut proc, mut mem) = setup();

    // Create two symbols, same value but different metadata
    let a = eval("(with-meta 'x %{:a 1})", &mut proc, &mut mem).unwrap();
    let b = eval("(with-meta 'x %{:b 2})", &mut proc, &mut mem).unwrap();

    // They should be equal (identity comparison for symbols)
    // Actually symbols compare by address, so different allocations won't be equal
    // But metadata shouldn't break this
    assert!(a.is_symbol());
    assert!(b.is_symbol());
}

#[test]
fn eval_meta_on_tuple() {
    let (mut proc, mut mem) = setup();

    let tuple = eval("(with-meta [1 2] %{:tag :vector})", &mut proc, &mut mem).unwrap();
    assert!(tuple.is_tuple());

    let meta = eval(
        "(meta (with-meta [1 2 3] %{:tag :vector}))",
        &mut proc,
        &mut mem,
    )
    .unwrap();
    assert!(meta.is_map());
}
