// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Symbol Operations.
//!
//! Section 9.5 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.5: Symbol Operations
// Reference: docs/lonala.md#95-symbol-operations
// ============================================================================

/// Spec 9.5: symbol creates/interns a symbol
#[test]
fn test_9_5_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(symbol \"foo\")",
        "foo",
        &spec_ref("9.5", "symbol", "create symbol 'foo' from string"),
    );
}

/// Spec 9.5: symbol from string equality
#[test]
fn test_9_5_symbol_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (symbol \"foo\") 'foo)",
        true,
        &spec_ref("9.5", "symbol", "created symbol equals quoted symbol"),
    );
}

/// Spec 9.5: gensym generates unique symbol
#[test]
fn test_9_5_gensym() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol(
        "(gensym)",
        &spec_ref("9.5", "gensym", "generate unique symbol"),
    );
}

/// Spec 9.5: gensym with prefix
#[test]
fn test_9_5_gensym_prefix() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol(
        "(gensym \"temp\")",
        &spec_ref("9.5", "gensym", "generate symbol with prefix"),
    );
}

/// Spec 9.5: gensym symbols are unique
#[test]
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

// ============================================================================
// Edge Cases
// ============================================================================

/// Spec 9.5: symbol with empty string creates empty-named symbol
#[test]
fn test_9_5_symbol_empty_string() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(symbol \"\")",
        "",
        &spec_ref("9.5", "symbol", "create symbol from empty string"),
    );
}

/// Spec 9.5: gensym with empty prefix uses empty prefix
#[test]
fn test_9_5_gensym_empty_prefix() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol(
        "(gensym \"\")",
        &spec_ref("9.5", "gensym", "generate symbol with empty prefix"),
    );
}

// ============================================================================
// Error Cases
// ============================================================================

/// Spec 9.5: symbol requires string argument
#[test]
fn test_9_5_symbol_type_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(symbol 42)",
        &spec_ref("9.5", "symbol", "type error for non-string argument"),
    );
}

/// Spec 9.5: symbol requires exactly one argument
#[test]
fn test_9_5_symbol_arity_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(symbol)",
        &spec_ref("9.5", "symbol", "arity error for no arguments"),
    );
}

/// Spec 9.5: gensym prefix must be string
#[test]
fn test_9_5_gensym_type_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(gensym 42)",
        &spec_ref("9.5", "gensym", "type error for non-string prefix"),
    );
}

/// Spec 9.5: gensym takes at most one argument
#[test]
fn test_9_5_gensym_arity_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(gensym \"a\" \"b\")",
        &spec_ref("9.5", "gensym", "arity error for too many arguments"),
    );
}
