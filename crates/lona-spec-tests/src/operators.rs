// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7 - Operators
//!
//! Reference: docs/lonala.md#7-operators
//!
//! Tests arithmetic, comparison, and logical operators.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 7.1.1: Addition (+)
// Reference: docs/lonala.md#711-addition-
// ============================================================================

/// [IGNORED] Spec 7.1.1: "With no arguments, returns 0"
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_7_1_1_addition_zero_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+)",
        0,
        &spec_ref("7.1.1", "+", "zero arguments returns 0"),
    );
}

/// [IGNORED] Spec 7.1.1: One argument
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_7_1_1_addition_one_arg() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+ 5)",
        5,
        &spec_ref("7.1.1", "+", "one argument returns itself"),
    );
}

/// Spec 7.1.1: Two arguments
#[test]
fn test_7_1_1_addition_two_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(+ 1 2)", 3, &spec_ref("7.1.1", "+", "two arguments"));
}

/// [IGNORED] Spec 7.1.1: Variadic
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_7_1_1_addition_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+ 1 2 3 4)",
        10,
        &spec_ref("7.1.1", "+", "variadic addition"),
    );
}

/// [IGNORED] Spec 7.1.1: Mixed types
/// Tracking: Ratio literals not yet implemented
#[test]
#[ignore]
fn test_7_1_1_addition_mixed_types() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_float(
        "(+ 1 2.0)",
        3.0,
        &spec_ref("7.1.1", "+", "int + float = float"),
    );
    ctx.assert_ratio(
        "(+ 1 1/2)",
        3,
        2,
        &spec_ref("7.1.1", "+", "int + ratio = ratio"),
    );
}

// ============================================================================
// Section 7.1.2: Subtraction (-)
// Reference: docs/lonala.md#712-subtraction--
// ============================================================================

/// Spec 7.1.2: "With one argument, returns its negation"
#[test]
fn test_7_1_2_subtraction_negation() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(- 5)", -5, &spec_ref("7.1.2", "-", "one argument negates"));
    ctx.assert_float("(- 1.5)", -1.5, &spec_ref("7.1.2", "-", "float negation"));
}

/// Spec 7.1.2: Two arguments - subtraction
#[test]
fn test_7_1_2_subtraction_two_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(- 10 3)", 7, &spec_ref("7.1.2", "-", "two arguments"));
}

/// [IGNORED] Spec 7.1.2: Variadic - subtracts subsequent from first
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_7_1_2_subtraction_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(- 10 3 2)",
        5,
        &spec_ref("7.1.2", "-", "variadic subtraction"),
    );
}

// ============================================================================
// Section 7.1.3: Multiplication (*)
// Reference: docs/lonala.md#713-multiplication-
// ============================================================================

/// [IGNORED] Spec 7.1.3: "With no arguments, returns 1"
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_7_1_3_multiplication_zero_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(*)",
        1,
        &spec_ref("7.1.3", "*", "zero arguments returns 1"),
    );
}

/// [IGNORED] Spec 7.1.3: One argument
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_7_1_3_multiplication_one_arg() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(* 5)",
        5,
        &spec_ref("7.1.3", "*", "one argument returns itself"),
    );
}

/// Spec 7.1.3: Two arguments
#[test]
fn test_7_1_3_multiplication_two_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(* 2 3)", 6, &spec_ref("7.1.3", "*", "two arguments"));
}

/// [IGNORED] Spec 7.1.3: Variadic
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_7_1_3_multiplication_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(* 2 3 4)",
        24,
        &spec_ref("7.1.3", "*", "variadic multiplication"),
    );
}

/// [IGNORED] Spec 7.1.3: Ratio multiplication
/// Tracking: Ratio literals not yet implemented
#[test]
#[ignore]
fn test_7_1_3_multiplication_ratio() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(* 1/2 1/3)",
        1,
        6,
        &spec_ref("7.1.3", "*", "ratio multiplication"),
    );
}

// ============================================================================
// Section 7.1.4: Division (/)
// Reference: docs/lonala.md#714-division-
// ============================================================================

/// [IGNORED] Spec 7.1.4: "With one argument, returns its reciprocal"
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_7_1_4_division_reciprocal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(/ 2)",
        1,
        2,
        &spec_ref("7.1.4", "/", "one argument returns reciprocal"),
    );
}

/// Spec 7.1.4: Two arguments - division
#[test]
fn test_7_1_4_division_two_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(/ 10 2)",
        5,
        &spec_ref("7.1.4", "/", "exact division yields integer"),
    );
}

/// [IGNORED] Spec 7.1.4: Variadic
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_7_1_4_division_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(/ 10 2 5)",
        1,
        &spec_ref("7.1.4", "/", "variadic division"),
    );
}

/// Spec 7.1.4: "Division of integers that doesn't produce a whole number yields a Ratio"
#[test]
fn test_7_1_4_division_yields_ratio() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(/ 1 3)",
        1,
        3,
        &spec_ref("7.1.4", "/", "inexact division yields ratio"),
    );
}

/// Spec 7.1.4: Float division
#[test]
fn test_7_1_4_division_float() {
    let mut ctx = SpecTestContext::new();
    // Using approximate comparison for floating point
    let result = ctx.eval("(/ 1.0 3)").unwrap();
    match result {
        lona_core::value::Value::Float(float_val) => {
            let expected = 1.0 / 3.0;
            assert!(
                (float_val - expected).abs() < 1e-10,
                "[Spec 7.1.4 /] float division: expected {}, got {}",
                expected,
                float_val
            );
        }
        _ => panic!("[Spec 7.1.4 /] expected float"),
    }
}

// ============================================================================
// Section 7.1.5: Modulo (mod)
// Reference: docs/lonala.md#715-modulo-mod
// ============================================================================

/// Spec 7.1.5: Basic modulo
#[test]
fn test_7_1_5_mod_basic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(mod 10 3)", 1, &spec_ref("7.1.5", "mod", "basic modulo"));
    ctx.assert_int("(mod 10 5)", 0, &spec_ref("7.1.5", "mod", "exact divisor"));
}

/// Spec 7.1.5: Negative modulo
#[test]
fn test_7_1_5_mod_negative() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(mod -10 3)",
        -1,
        &spec_ref("7.1.5", "mod", "negative dividend"),
    );
}

// ============================================================================
// Section 7.2.1: Equality (=)
// Reference: docs/lonala.md#721-equality-
// ============================================================================

/// Spec 7.2.1: Same values are equal
#[test]
fn test_7_2_1_equality_same_values() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(= 1 1)", true, &spec_ref("7.2.1", "=", "same integers"));
    ctx.assert_bool(
        "(= \"a\" \"a\")",
        true,
        &spec_ref("7.2.1", "=", "same strings"),
    );
}

/// Spec 7.2.1: Different values are not equal
#[test]
fn test_7_2_1_equality_different_values() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 1 2)",
        false,
        &spec_ref("7.2.1", "=", "different integers"),
    );
}

/// Spec 7.2.1: Cross-type numeric equality
#[test]
fn test_7_2_1_equality_cross_type() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 1 1.0)",
        true,
        &spec_ref("7.2.1", "=", "integer equals float when same value"),
    );
}

// ============================================================================
// Section 7.2.2-7.2.5: Comparison Operators (<, >, <=, >=)
// Reference: docs/lonala.md#722-less-than-
// ============================================================================

/// Spec 7.2.2: Less than
#[test]
fn test_7_2_2_less_than() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(< 1 2)", true, &spec_ref("7.2.2", "<", "1 < 2"));
    ctx.assert_bool("(< 2 1)", false, &spec_ref("7.2.2", "<", "2 < 1 is false"));
    ctx.assert_bool("(< 1 1)", false, &spec_ref("7.2.2", "<", "1 < 1 is false"));
}

/// Spec 7.2.3: Greater than
#[test]
fn test_7_2_3_greater_than() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(> 2 1)", true, &spec_ref("7.2.3", ">", "2 > 1"));
    ctx.assert_bool("(> 1 2)", false, &spec_ref("7.2.3", ">", "1 > 2 is false"));
    ctx.assert_bool("(> 1 1)", false, &spec_ref("7.2.3", ">", "1 > 1 is false"));
}

/// Spec 7.2.4: Less than or equal
#[test]
fn test_7_2_4_less_than_or_equal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(<= 1 2)", true, &spec_ref("7.2.4", "<=", "1 <= 2"));
    ctx.assert_bool("(<= 1 1)", true, &spec_ref("7.2.4", "<=", "1 <= 1"));
    ctx.assert_bool(
        "(<= 2 1)",
        false,
        &spec_ref("7.2.4", "<=", "2 <= 1 is false"),
    );
}

/// Spec 7.2.5: Greater than or equal
#[test]
fn test_7_2_5_greater_than_or_equal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(>= 2 1)", true, &spec_ref("7.2.5", ">=", "2 >= 1"));
    ctx.assert_bool("(>= 1 1)", true, &spec_ref("7.2.5", ">=", "1 >= 1"));
    ctx.assert_bool(
        "(>= 1 2)",
        false,
        &spec_ref("7.2.5", ">=", "1 >= 2 is false"),
    );
}

// ============================================================================
// Section 7.3.1: Logical Not (not)
// Reference: docs/lonala.md#731-logical-not-not
// ============================================================================

/// Spec 7.3.1: "Returns true if x is falsy, false otherwise"
#[test]
fn test_7_3_1_not_falsy_values() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(not false)",
        true,
        &spec_ref("7.3.1", "not", "not false is true"),
    );
    ctx.assert_bool(
        "(not nil)",
        true,
        &spec_ref("7.3.1", "not", "not nil is true"),
    );
}

/// Spec 7.3.1: Truthy values
#[test]
fn test_7_3_1_not_truthy_values() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(not true)",
        false,
        &spec_ref("7.3.1", "not", "not true is false"),
    );
    ctx.assert_bool(
        "(not 0)",
        false,
        &spec_ref("7.3.1", "not", "not 0 is false (0 is truthy)"),
    );
    ctx.assert_bool(
        "(not \"\")",
        false,
        &spec_ref("7.3.1", "not", "not \"\" is false (empty string is truthy)"),
    );
}

// ============================================================================
// Section 7.4: Numeric Type Coercion
// Reference: docs/lonala.md#74-numeric-type-coercion
// ============================================================================

/// Spec 7.4: Integer + Integer = Integer
#[test]
fn test_7_4_coercion_int_int() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+ 1 2)",
        3,
        &spec_ref("7.4", "Coercion", "int + int = int"),
    );
}

/// Spec 7.4: Integer + Float = Float
#[test]
fn test_7_4_coercion_int_float() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_float(
        "(+ 1 2.0)",
        3.0,
        &spec_ref("7.4", "Coercion", "int + float = float"),
    );
}

/// [IGNORED] Spec 7.4: Integer + Ratio = Ratio
/// Tracking: Ratio literals not yet implemented
#[test]
#[ignore]
fn test_7_4_coercion_int_ratio() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(+ 1 1/2)",
        3,
        2,
        &spec_ref("7.4", "Coercion", "int + ratio = ratio"),
    );
}

/// Spec 7.4: Integer / Integer (exact) = Integer
#[test]
fn test_7_4_coercion_division_exact() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(/ 6 2)",
        3,
        &spec_ref("7.4", "Coercion", "int / int exact = int"),
    );
}

/// Spec 7.4: Integer / Integer (inexact) = Ratio
#[test]
fn test_7_4_coercion_division_inexact() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(/ 5 2)",
        5,
        2,
        &spec_ref("7.4", "Coercion", "int / int inexact = ratio"),
    );
}
