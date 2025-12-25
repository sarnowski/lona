// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 6.X - case
//!
//! Reference: docs/lonala/special-forms.md
//!
//! Tests the `case` special form for value-based dispatch on compile-time constants.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Basic Pattern Matching
// ============================================================================

/// Spec 6.X: Integer pattern matching
#[test]
fn test_case_integer_dispatch() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case 2 1 \"one\" 2 \"two\" 3 \"three\")",
        "two",
        &spec_ref(
            "6.X",
            "case",
            "integer pattern 2 matches and returns \"two\"",
        ),
    );
}

/// Spec 6.X: First integer in sequence
#[test]
fn test_case_integer_first() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case 1 1 \"one\" 2 \"two\" 3 \"three\")",
        "one",
        &spec_ref("6.X", "case", "integer pattern 1 matches first clause"),
    );
}

/// Spec 6.X: Last integer in sequence
#[test]
fn test_case_integer_last() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case 3 1 \"one\" 2 \"two\" 3 \"three\")",
        "three",
        &spec_ref("6.X", "case", "integer pattern 3 matches last clause"),
    );
}

/// Spec 6.X: Keyword pattern matching
#[test]
fn test_case_keyword_dispatch() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(case :b :a 1 :b 2 :c 3)",
        2,
        &spec_ref("6.X", "case", "keyword pattern :b matches"),
    );
}

/// Spec 6.X: String pattern matching
#[test]
fn test_case_string_dispatch() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(case \"hello\" \"world\" 1 \"hello\" 2 \"foo\" 3)",
        2,
        &spec_ref("6.X", "case", "string pattern \"hello\" matches"),
    );
}

/// Spec 6.X: nil pattern matches nil value
#[test]
fn test_case_nil_pattern() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case nil nil \"matched-nil\" :other \"other\")",
        "matched-nil",
        &spec_ref("6.X", "case", "nil pattern matches nil value"),
    );
}

/// Spec 6.X: true boolean pattern
#[test]
fn test_case_boolean_true_pattern() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case true false \"false\" true \"true\")",
        "true",
        &spec_ref("6.X", "case", "boolean pattern true matches"),
    );
}

/// Spec 6.X: false boolean pattern
#[test]
fn test_case_boolean_false_pattern() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case false false \"false\" true \"true\")",
        "false",
        &spec_ref("6.X", "case", "boolean pattern false matches"),
    );
}

// ============================================================================
// Default Clause (:else)
// ============================================================================

/// Spec 6.X: :else default returns correct result when no match
#[test]
fn test_case_else_default() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case 99 1 \"one\" 2 \"two\" :else \"default\")",
        "default",
        &spec_ref(
            "6.X",
            "case",
            ":else clause matches when no other pattern matches",
        ),
    );
}

/// Spec 6.X: :else with keyword dispatch value
#[test]
fn test_case_else_keyword_no_match() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(case :unknown :a 1 :b 2 :else 0)",
        0,
        &spec_ref("6.X", "case", ":else returns 0 when keyword doesn't match"),
    );
}

/// Spec 6.X: Matching clause wins over :else
#[test]
fn test_case_match_wins_over_else() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case 1 1 \"matched\" :else \"default\")",
        "matched",
        &spec_ref("6.X", "case", "matching clause is preferred over :else"),
    );
}

// ============================================================================
// No Match Without Default
// ============================================================================

/// Spec 6.X: No-match without default triggers runtime error
#[test]
fn test_case_no_match_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error_contains(
        "(case 99 1 \"one\" 2 \"two\")",
        "NoMatchingCase",
        &spec_ref(
            "6.X",
            "case",
            "no match without :else triggers NoMatchingCase error",
        ),
    );
}

// ============================================================================
// First Matching Clause Wins
// ============================================================================

/// Spec 6.X: First matching clause wins (for documentation)
/// Note: Duplicate patterns are rejected at compile time, but if somehow
/// the same logical value could appear in different forms, first wins.
#[test]
fn test_case_first_clause_wins() {
    let mut ctx = SpecTestContext::new();
    // Test with different values to show linear scan behavior
    ctx.assert_string(
        "(case 1 1 \"first\" :else \"default\")",
        "first",
        &spec_ref("6.X", "case", "first matching clause is evaluated"),
    );
}

// ============================================================================
// Expression Evaluation
// ============================================================================

/// Spec 6.X: Dispatch expression is evaluated
#[test]
fn test_case_expr_evaluated() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case (+ 1 1) 1 \"one\" 2 \"two\" 3 \"three\")",
        "two",
        &spec_ref("6.X", "case", "dispatch expression (+ 1 1) evaluates to 2"),
    );
}

/// Spec 6.X: Result expression is evaluated
#[test]
fn test_case_result_evaluated() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(case 1 1 (+ 10 20) :else 0)",
        30,
        &spec_ref("6.X", "case", "result expression (+ 10 20) is evaluated"),
    );
}

/// Spec 6.X: Only matching result is evaluated (no side effects from others)
#[test]
fn test_case_only_matching_evaluated() {
    let mut ctx = SpecTestContext::new();
    // Define a counter and increment function
    let _res = ctx.eval("(def counter 0)").unwrap();
    let _res = ctx
        .eval("(def inc-counter (fn [] (def counter (+ counter 1)) counter))")
        .unwrap();

    // The case should only evaluate the matching clause
    ctx.assert_int(
        "(case 2 1 (inc-counter) 2 42 3 (inc-counter))",
        42,
        &spec_ref(
            "6.X",
            "case",
            "only matching result expression is evaluated",
        ),
    );

    // Counter should still be 0 (neither inc-counter call was made)
    ctx.assert_int(
        "counter",
        0,
        &spec_ref("6.X", "case", "non-matching clauses are not evaluated"),
    );
}

// ============================================================================
// Mixed Pattern Types
// ============================================================================

/// Spec 6.X: Mixed pattern types in same case
#[test]
fn test_case_mixed_patterns() {
    let mut ctx = SpecTestContext::new();

    // Match integer
    ctx.assert_string(
        "(case 42 42 \"int\" :key \"keyword\" \"str\" \"string\" :else \"default\")",
        "int",
        &spec_ref("6.X", "case", "integer 42 matches in mixed patterns"),
    );

    // Match keyword
    ctx.assert_string(
        "(case :key 42 \"int\" :key \"keyword\" \"str\" \"string\" :else \"default\")",
        "keyword",
        &spec_ref("6.X", "case", "keyword :key matches in mixed patterns"),
    );

    // Match string
    ctx.assert_string(
        "(case \"str\" 42 \"int\" :key \"keyword\" \"str\" \"string\" :else \"default\")",
        "string",
        &spec_ref("6.X", "case", "string \"str\" matches in mixed patterns"),
    );

    // Fall through to default
    ctx.assert_string(
        "(case :unknown 42 \"int\" :key \"keyword\" \"str\" \"string\" :else \"default\")",
        "default",
        &spec_ref("6.X", "case", "unmatched value falls through to :else"),
    );
}

// ============================================================================
// Negative Numbers
// ============================================================================

/// Spec 6.X: Negative integer patterns
#[test]
fn test_case_negative_integers() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case -1 -2 \"neg-two\" -1 \"neg-one\" 0 \"zero\" 1 \"one\")",
        "neg-one",
        &spec_ref("6.X", "case", "negative integer -1 matches"),
    );
}

// ============================================================================
// Empty String
// ============================================================================

/// Spec 6.X: Empty string pattern
#[test]
fn test_case_empty_string() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case \"\" \"\" \"empty\" \"hello\" \"hello\" :else \"other\")",
        "empty",
        &spec_ref("6.X", "case", "empty string pattern matches"),
    );
}

// ============================================================================
// Zero Value
// ============================================================================

/// Spec 6.X: Zero as pattern
#[test]
fn test_case_zero_pattern() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(case 0 -1 \"negative\" 0 \"zero\" 1 \"positive\")",
        "zero",
        &spec_ref("6.X", "case", "zero pattern matches"),
    );
}
