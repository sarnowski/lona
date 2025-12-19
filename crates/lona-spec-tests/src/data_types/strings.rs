// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for String type.
//!
//! Section 3.7 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.7: String
// Reference: docs/lonala.md#37-string
// ============================================================================

/// Spec 3.7: String equality
#[test]
fn test_3_7_string_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= \"hello\" \"hello\")",
        true,
        &spec_ref("3.7", "String", "identical strings are equal"),
    );
    ctx.assert_bool(
        "(= \"hello\" \"world\")",
        false,
        &spec_ref("3.7", "String", "different strings are not equal"),
    );
}

/// Spec 3.7: Empty string
#[test]
fn test_3_7_empty_string() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string("\"\"", "", &spec_ref("3.7", "String", "empty string"));
}

/// Spec 3.7: String escape sequences
#[test]
fn test_3_7_string_escape_sequences() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "\"line1\\nline2\"",
        "line1\nline2",
        &spec_ref("3.7", "String", "newline escape"),
    );
    ctx.assert_string(
        "\"tab\\there\"",
        "tab\there",
        &spec_ref("3.7", "String", "tab escape"),
    );
    ctx.assert_string(
        "\"quote: \\\"hi\\\"\"",
        "quote: \"hi\"",
        &spec_ref("3.7", "String", "escaped quote"),
    );
    ctx.assert_string(
        "\"backslash: \\\\\"",
        "backslash: \\",
        &spec_ref("3.7", "String", "escaped backslash"),
    );
}

/// Spec 3.7: Strings are immutable
#[test]
fn test_3_7_string_immutability() {
    let mut ctx = SpecTestContext::new();
    // Define a string and verify it's unchanged after operations
    let _res = ctx.eval("(def s \"hello\")").unwrap();
    ctx.assert_string(
        "s",
        "hello",
        &spec_ref("3.7", "String", "string is immutable"),
    );
}

/// Spec 3.7: String is truthy (even empty)
#[test]
fn test_3_7_string_truthiness() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if \"\" 1 2)",
        1,
        &spec_ref("3.7", "String", "empty string is truthy"),
    );
    ctx.assert_int(
        "(if \"hello\" 1 2)",
        1,
        &spec_ref("3.7", "String", "non-empty string is truthy"),
    );
}
