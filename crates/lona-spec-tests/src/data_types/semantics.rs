// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for truthiness and equality semantics.
//!
//! Sections 3.14-3.15 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.14: Truthiness
// Reference: docs/lonala.md#314-truthiness
// ============================================================================

/// Spec 3.14: "nil is falsy, false is falsy, everything else is truthy"
#[test]
fn test_3_14_truthiness_nil_false() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if nil 1 2)",
        2,
        &spec_ref("3.14", "Truthiness", "nil is falsy"),
    );
    ctx.assert_int(
        "(if false 1 2)",
        2,
        &spec_ref("3.14", "Truthiness", "false is falsy"),
    );
}

/// Spec 3.14: "0 is truthy, empty string is truthy, empty collections are truthy"
#[test]
fn test_3_14_truthiness_zero_and_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if 0 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "0 is truthy"),
    );
    ctx.assert_int(
        "(if \"\" 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "empty string is truthy"),
    );
    ctx.assert_int(
        "(if '() 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "empty list is truthy"),
    );
}

/// Spec 3.14: true is truthy
#[test]
fn test_3_14_truthiness_true() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if true 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "true is truthy"),
    );
}

/// Spec 3.14: Numbers other than 0 are truthy
#[test]
fn test_3_14_truthiness_numbers() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if 1 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "1 is truthy"),
    );
    ctx.assert_int(
        "(if -1 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "-1 is truthy"),
    );
    ctx.assert_int(
        "(if 0.0 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "0.0 is truthy"),
    );
}

/// Spec 3.14: Functions are truthy
#[test]
fn test_3_14_truthiness_function() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if (fn [x] x) 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "function is truthy"),
    );
}

/// Spec 3.14: Non-empty collections are truthy
#[test]
fn test_3_14_truthiness_collections() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if '(1 2 3) 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "non-empty list is truthy"),
    );
    ctx.assert_int(
        "(if (vector 1 2 3) 1 2)",
        1,
        &spec_ref("3.14", "Truthiness", "non-empty vector is truthy"),
    );
}

// ============================================================================
// Section 3.15: Equality
// Reference: docs/lonala.md#315-equality
// ============================================================================

/// Spec 3.15: Structural equality for most types
#[test]
fn test_3_15_structural_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 1 1)",
        true,
        &spec_ref("3.15", "Equality", "integer equality"),
    );
    ctx.assert_bool(
        "(= \"abc\" \"abc\")",
        true,
        &spec_ref("3.15", "Equality", "string equality"),
    );
}

/// Spec 3.15: "Numbers of different types can be equal if they represent the same value"
#[test]
fn test_3_15_cross_type_numeric_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 1 1.0)",
        true,
        &spec_ref("3.15", "Equality", "integer equals float when same value"),
    );
}

/// Spec 3.15: Collection equality
#[test]
fn test_3_15_collection_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= '(1 2 3) '(1 2 3))",
        true,
        &spec_ref("3.15", "Equality", "list equality"),
    );
    ctx.assert_bool(
        "(= (vector 1 2 3) (vector 1 2 3))",
        true,
        &spec_ref("3.15", "Equality", "vector equality"),
    );
    ctx.assert_bool(
        "(= (hash-map 'a 1) (hash-map 'a 1))",
        true,
        &spec_ref("3.15", "Equality", "map equality"),
    );
}

/// Spec 3.15: ##NaN is not equal to anything, including itself
#[test]
fn test_3_15_nan_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= ##NaN ##NaN)",
        false,
        &spec_ref("3.15", "Equality", "NaN is not equal to itself"),
    );
}

/// Spec 3.15: Different types are not equal (except numeric and sequential)
#[test]
fn test_3_15_different_types() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 1 \"1\")",
        false,
        &spec_ref("3.15", "Equality", "integer not equal to string"),
    );
    ctx.assert_bool(
        "(= nil false)",
        false,
        &spec_ref("3.15", "Equality", "nil not equal to false"),
    );
}

/// Spec 3.15: Sequential equality (lists and vectors with same elements are equal)
#[test]
fn test_3_15_sequential_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= '(1 2 3) (vector 1 2 3))",
        true,
        &spec_ref("3.15", "Equality", "list equals vector with same elements"),
    );
}
