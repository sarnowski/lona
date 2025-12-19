// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Symbol Operations (Planned).
//!
//! Section 9.5 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.5: Symbol Operations
// Reference: docs/lonala.md#95-symbol-operations
// ============================================================================

/// [IGNORED] Spec 9.5: symbol creates/interns a symbol
/// Tracking: Symbol operations not fully exposed yet
#[test]
#[ignore]
fn test_9_5_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(symbol \"foo\")",
        "foo",
        &spec_ref("9.5", "symbol", "create symbol 'foo' from string"),
    );
}

/// [IGNORED] Spec 9.5: symbol from string equality
/// Tracking: Symbol operations not fully exposed yet
#[test]
#[ignore]
fn test_9_5_symbol_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (symbol \"foo\") 'foo)",
        true,
        &spec_ref("9.5", "symbol", "created symbol equals quoted symbol"),
    );
}

/// [IGNORED] Spec 9.5: gensym generates unique symbol
/// Tracking: gensym not yet implemented
#[test]
#[ignore]
fn test_9_5_gensym() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol(
        "(gensym)",
        &spec_ref("9.5", "gensym", "generate unique symbol"),
    );
}

/// [IGNORED] Spec 9.5: gensym with prefix
/// Tracking: gensym not yet implemented
#[test]
#[ignore]
fn test_9_5_gensym_prefix() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol(
        "(gensym \"temp\")",
        &spec_ref("9.5", "gensym", "generate symbol with prefix"),
    );
}

/// [IGNORED] Spec 9.5: gensym symbols are unique
/// Tracking: gensym not yet implemented
#[test]
#[ignore]
fn test_9_5_gensym_unique() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def g1 (gensym))").unwrap();
    let _res = ctx.eval("(def g2 (gensym))").unwrap();
    ctx.assert_bool(
        "(= g1 g2)",
        false,
        &spec_ref("9.5", "gensym", "two gensyms are different"),
    );
}
