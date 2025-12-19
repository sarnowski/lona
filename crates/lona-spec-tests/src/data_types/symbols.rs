// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Symbol type.
//!
//! Section 3.5 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.5: Symbol
// Reference: docs/lonala.md#35-symbol
// ============================================================================

/// Spec 3.5: Symbols are interned for fast equality comparison
#[test]
fn test_3_5_symbol_equality() {
    let mut ctx = SpecTestContext::new();
    // Define and compare symbols
    let _res = ctx.eval("(def sym1 'foo)").unwrap();
    let _res = ctx.eval("(def sym2 'foo)").unwrap();
    ctx.assert_bool(
        "(= sym1 sym2)",
        true,
        &spec_ref("3.5", "Symbol", "interned symbols are equal"),
    );
}

/// Spec 3.5: Different symbols are not equal
#[test]
fn test_3_5_symbol_inequality() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def sym1 'foo)").unwrap();
    let _res = ctx.eval("(def sym2 'bar)").unwrap();
    ctx.assert_bool(
        "(= sym1 sym2)",
        false,
        &spec_ref("3.5", "Symbol", "different symbols are not equal"),
    );
}

/// Spec 3.5: Symbols with special characters
#[test]
fn test_3_5_symbol_special_chars() {
    let mut ctx = SpecTestContext::new();
    // Test various valid symbol names
    ctx.assert_symbol_eq(
        "'empty?",
        "empty?",
        &spec_ref("3.5", "Symbol", "predicate symbol with ?"),
    );
    ctx.assert_symbol_eq(
        "'set!",
        "set!",
        &spec_ref("3.5", "Symbol", "mutating symbol with !"),
    );
    ctx.assert_symbol_eq(
        "'my-var",
        "my-var",
        &spec_ref("3.5", "Symbol", "hyphenated symbol"),
    );
    ctx.assert_symbol_eq(
        "'*special*",
        "*special*",
        &spec_ref("3.5", "Symbol", "earmuffed symbol"),
    );
}

/// [IGNORED] Spec 3.5: Qualified symbols with namespace prefix
/// Tracking: Namespace support planned for Phase 6
#[test]
#[ignore]
fn test_3_5_qualified_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "'ns/name",
        "ns/name",
        &spec_ref("3.5", "Symbol", "qualified symbol with namespace"),
    );
}
