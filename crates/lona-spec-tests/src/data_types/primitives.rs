// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for primitive types: Nil, Bool, and Numbers.
//!
//! Sections 3.2-3.4 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.2: Nil
// Reference: docs/lonala.md#32-nil
// ============================================================================

/// Spec 3.2: "Truthiness: nil is falsy"
#[test]
fn test_3_2_nil_is_falsy() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if nil 1 2)",
        2,
        &spec_ref("3.2", "Nil", "nil should be falsy in conditionals"),
    );
}

/// Spec 3.2: "Equality: nil equals only itself"
#[test]
fn test_3_2_nil_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= nil nil)",
        true,
        &spec_ref("3.2", "Nil", "nil equals nil"),
    );
    ctx.assert_bool(
        "(= nil false)",
        false,
        &spec_ref("3.2", "Nil", "nil does not equal false"),
    );
}

// ============================================================================
// Section 3.3: Bool
// Reference: docs/lonala.md#33-bool
// ============================================================================

/// Spec 3.3: "Truthiness: false is falsy; true is truthy"
#[test]
fn test_3_3_bool_truthiness() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if true 1 2)",
        1,
        &spec_ref("3.3", "Bool", "true is truthy"),
    );
    ctx.assert_int(
        "(if false 1 2)",
        2,
        &spec_ref("3.3", "Bool", "false is falsy"),
    );
}

/// Spec 3.3: "Equality: Booleans equal only themselves"
#[test]
fn test_3_3_bool_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= true true)",
        true,
        &spec_ref("3.3", "Bool", "true equals true"),
    );
    ctx.assert_bool(
        "(= false false)",
        true,
        &spec_ref("3.3", "Bool", "false equals false"),
    );
    ctx.assert_bool(
        "(= true false)",
        false,
        &spec_ref("3.3", "Bool", "true does not equal false"),
    );
    ctx.assert_bool(
        "(= true 1)",
        false,
        &spec_ref("3.3", "Bool", "true does not equal 1 (no type coercion)"),
    );
}

// ============================================================================
// Section 3.4.1: Integer
// Reference: docs/lonala.md#341-integer
// ============================================================================

/// Spec 3.4.1: Integers support all arithmetic operations
#[test]
fn test_3_4_1_integer_arithmetic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(+ 10 5)", 15, &spec_ref("3.4.1", "Integer", "addition"));
    ctx.assert_int("(- 10 5)", 5, &spec_ref("3.4.1", "Integer", "subtraction"));
    ctx.assert_int(
        "(* 10 5)",
        50,
        &spec_ref("3.4.1", "Integer", "multiplication"),
    );
}

/// Spec 3.4.1: Integer equality
#[test]
fn test_3_4_1_integer_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 42 42)",
        true,
        &spec_ref("3.4.1", "Integer", "same integers are equal"),
    );
    ctx.assert_bool(
        "(= 42 43)",
        false,
        &spec_ref("3.4.1", "Integer", "different integers are not equal"),
    );
}

/// Spec 3.4.1: Negative integers
#[test]
fn test_3_4_1_negative_integers() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "-17",
        -17,
        &spec_ref("3.4.1", "Integer", "negative integer literal"),
    );
    ctx.assert_int(
        "(- 0 5)",
        -5,
        &spec_ref("3.4.1", "Integer", "subtraction yields negative"),
    );
}

// ============================================================================
// Section 3.4.2: Float
// Reference: docs/lonala.md#342-float
// ============================================================================

/// Spec 3.4.2: Float arithmetic
#[test]
fn test_3_4_2_float_arithmetic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_float(
        "(+ 1.5 2.5)",
        4.0,
        &spec_ref("3.4.2", "Float", "float addition"),
    );
    ctx.assert_float(
        "(- 3.5 1.5)",
        2.0,
        &spec_ref("3.4.2", "Float", "float subtraction"),
    );
    ctx.assert_float(
        "(* 2.0 3.0)",
        6.0,
        &spec_ref("3.4.2", "Float", "float multiplication"),
    );
}

/// Spec 3.4.2: Float special values
#[test]
fn test_3_4_2_float_infinity() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_float(
        "##Inf",
        f64::INFINITY,
        &spec_ref("3.4.2", "Float", "positive infinity"),
    );
    ctx.assert_float(
        "##-Inf",
        f64::NEG_INFINITY,
        &spec_ref("3.4.2", "Float", "negative infinity"),
    );
}

/// Spec 3.4.2: NaN equality - "(= ##NaN ##NaN) is false per IEEE 754"
#[test]
fn test_3_4_2_nan_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= ##NaN ##NaN)",
        false,
        &spec_ref("3.4.2", "Float", "NaN is not equal to itself per IEEE 754"),
    );
}

// ============================================================================
// Section 3.4.3: Ratio
// Reference: docs/lonala.md#343-ratio
// ============================================================================

/// [IGNORED] Spec 3.4.3: Ratio arithmetic produces exact results
/// Tracking: Ratio literals not yet implemented as direct syntax
#[test]
#[ignore]
fn test_3_4_3_ratio_arithmetic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(+ 1/3 1/3)",
        2,
        3,
        &spec_ref("3.4.3", "Ratio", "ratio addition"),
    );
    ctx.assert_ratio(
        "(- 1/2 1/4)",
        1,
        4,
        &spec_ref("3.4.3", "Ratio", "ratio subtraction"),
    );
    ctx.assert_ratio(
        "(* 1/2 1/3)",
        1,
        6,
        &spec_ref("3.4.3", "Ratio", "ratio multiplication"),
    );
}

/// [IGNORED] Spec 3.4.3: Ratios are automatically normalized
/// Tracking: Ratio literals not yet implemented as direct syntax
#[test]
#[ignore]
fn test_3_4_3_ratio_normalization() {
    let mut ctx = SpecTestContext::new();
    // -2/4 should normalize to -1/2
    ctx.assert_ratio(
        "-2/4",
        -1,
        2,
        &spec_ref("3.4.3", "Ratio", "ratio normalization"),
    );
}
