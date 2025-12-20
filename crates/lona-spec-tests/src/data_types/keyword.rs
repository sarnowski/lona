// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Keyword type.
//!
//! Section 3.6 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.6: Keyword
// Reference: docs/lonala.md#36-keyword
// ============================================================================

/// Spec 3.6: Keyword literals are self-evaluating
#[test]
fn test_3_6_keyword_self_evaluating() {
    let mut ctx = SpecTestContext::new();
    // Keywords should evaluate to themselves
    // Note: We can't directly compare keywords yet, so we test via equality
    ctx.assert_bool(
        "(= :foo :foo)",
        true,
        &spec_ref("3.6", "Keyword", "keyword :foo equals itself"),
    );
}

/// Spec 3.6: Keywords are interned for fast equality comparison
#[test]
fn test_3_6_keyword_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= :foo :foo)",
        true,
        &spec_ref("3.6", "Keyword", "same keywords are equal"),
    );
    ctx.assert_bool(
        "(= :foo :bar)",
        false,
        &spec_ref("3.6", "Keyword", "different keywords are not equal"),
    );
    ctx.assert_bool(
        "(= :foo 'foo)",
        false,
        &spec_ref(
            "3.6",
            "Keyword",
            "keyword does not equal symbol with same name",
        ),
    );
}

/// [IGNORED] Spec 3.6: Qualified keywords with namespace prefix
/// Tracking: Namespace support not yet implemented
#[test]
#[ignore]
fn test_3_6_keyword_qualified() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= :ns/name :ns/name)",
        true,
        &spec_ref("3.6", "Keyword", "qualified keywords are equal"),
    );
    ctx.assert_bool(
        "(= :ns/name :other/name)",
        false,
        &spec_ref(
            "3.6",
            "Keyword",
            "different namespace makes keywords unequal",
        ),
    );
}

/// [IGNORED] Spec 3.6: Keywords as map keys (common use case)
/// Tracking: Map literal syntax and `get` function not yet implemented
#[test]
#[ignore]
fn test_3_6_keyword_as_map_key() {
    let mut ctx = SpecTestContext::new();
    // Map lookup with keyword key
    ctx.assert_int(
        "(get {:a 1 :b 2} :a)",
        1,
        &spec_ref("3.6", "Keyword", "keyword as map key"),
    );
    ctx.assert_int(
        "(get {:a 1 :b 2} :b)",
        2,
        &spec_ref("3.6", "Keyword", "second keyword key lookup"),
    );
    ctx.assert_nil(
        "(get {:a 1} :missing)",
        &spec_ref("3.6", "Keyword", "missing keyword key returns nil"),
    );
}

/// Spec 3.6: Keywords with hyphenated names
#[test]
fn test_3_6_keyword_hyphenated() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= :foo-bar :foo-bar)",
        true,
        &spec_ref("3.6", "Keyword", "hyphenated keyword"),
    );
}

/// Spec 3.6: keyword? predicate
#[test]
fn test_3_6_keyword_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(keyword? :foo)",
        true,
        &spec_ref("3.6", "Keyword", "keyword? returns true for keyword"),
    );
    ctx.assert_bool(
        "(keyword? 'foo)",
        false,
        &spec_ref("3.6", "Keyword", "keyword? returns false for symbol"),
    );
    ctx.assert_bool(
        "(keyword? \"foo\")",
        false,
        &spec_ref("3.6", "Keyword", "keyword? returns false for string"),
    );
}
