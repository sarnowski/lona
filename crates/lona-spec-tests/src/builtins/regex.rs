// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Regular Expressions (Planned).
//!
//! Section 9.17 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.17: Regular Expressions
// Reference: docs/lonala.md#917-regular-expressions
// ============================================================================

/// [IGNORED] Spec 9.17: re-pattern compiles string to regex
/// Tracking: Regex not yet implemented
#[test]
#[ignore]
fn test_9_17_re_pattern() {
    let mut _ctx = SpecTestContext::new();
    // (re-pattern "\\d+") creates a compiled regex
}

/// [IGNORED] Spec 9.17: re-find finds first match
/// Tracking: Regex not yet implemented
#[test]
#[ignore]
fn test_9_17_re_find() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(re-find #\"\\d+\" \"abc123def\")",
        "123",
        &spec_ref("9.17", "re-find", "finds first match"),
    );
}

/// [IGNORED] Spec 9.17: re-find returns nil when no match
/// Tracking: Regex not yet implemented
#[test]
#[ignore]
fn test_9_17_re_find_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(re-find #\"\\d+\" \"abcdef\")",
        &spec_ref("9.17", "re-find", "returns nil when no match"),
    );
}

/// [IGNORED] Spec 9.17: re-find with groups
/// Tracking: Regex not yet implemented
#[test]
#[ignore]
fn test_9_17_re_find_groups() {
    let mut ctx = SpecTestContext::new();
    // Returns vector with full match and groups
    ctx.assert_vector_len(
        "(re-find #\"(\\d+)-(\\d+)\" \"phone: 555-1234\")",
        3,
        &spec_ref("9.17", "re-find", "returns match and groups"),
    );
}

/// [IGNORED] Spec 9.17: re-matches matches entire string
/// Tracking: Regex not yet implemented
#[test]
#[ignore]
fn test_9_17_re_matches() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(re-matches #\"\\d+\" \"123\")",
        "123",
        &spec_ref("9.17", "re-matches", "matches entire string"),
    );
    ctx.assert_nil(
        "(re-matches #\"\\d+\" \"abc123\")",
        &spec_ref("9.17", "re-matches", "nil when not entire match"),
    );
}

/// [IGNORED] Spec 9.17: re-seq finds all matches
/// Tracking: Regex not yet implemented
#[test]
#[ignore]
fn test_9_17_re_seq() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(re-seq #\"\\d+\" \"a1b2c3\")",
        "'(\"1\" \"2\" \"3\")",
        &spec_ref("9.17", "re-seq", "returns all matches"),
    );
}

/// [IGNORED] Spec 9.17: regex literal #"pattern"
/// Tracking: Regex literal reader macro not yet implemented
#[test]
#[ignore]
fn test_9_17_regex_literal() {
    let mut _ctx = SpecTestContext::new();
    // #"\\d+" is syntactic sugar for (re-pattern "\\d+")
}

/// [IGNORED] Spec 9.17: case-insensitive flag
/// Tracking: Regex not yet implemented
#[test]
#[ignore]
fn test_9_17_regex_flags() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(re-find #\"(?i)hello\" \"HELLO world\")",
        "HELLO",
        &spec_ref("9.17", "re-find", "case-insensitive match"),
    );
}
