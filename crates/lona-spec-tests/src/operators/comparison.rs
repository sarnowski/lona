// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7.2 - Comparison Operators
//!
//! Reference: docs/lonala/operators.md
//!
//! Tests equality (=) and ordering (<, >, <=, >=) operators,
//! including cross-type numeric comparisons, string comparisons,
//! and the logical not operator.

use crate::{SpecTestContext, spec_ref};

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

/// Spec 7.2.1: Cross-type numeric in collections (deep structural equality)
#[test]
fn test_7_2_1_equality_collection_cross_type_numeric() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (vector 1) (vector 1.0))",
        true,
        &spec_ref("7.2.1", "=", "vector with cross-type numeric elements"),
    );
    ctx.assert_bool(
        "(= (vector 1 2) (vector 1.0 2.0))",
        true,
        &spec_ref("7.2.1", "=", "vector with multiple cross-type elements"),
    );
    ctx.assert_bool(
        "(= '(1) '(1.0))",
        true,
        &spec_ref("7.2.1", "=", "list with cross-type numeric elements"),
    );
}

/// Spec 7.2.1: List vs Vector equality (sequential partition)
#[test]
fn test_7_2_1_equality_list_vs_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (vector 1 2) '(1 2))",
        true,
        &spec_ref("7.2.1", "=", "vector equals list with same elements"),
    );
    ctx.assert_bool(
        "(= '(1 2 3) (vector 1 2 3))",
        true,
        &spec_ref("7.2.1", "=", "list equals vector with same elements"),
    );
}

/// Spec 7.2.1: Nested collection equality
#[test]
fn test_7_2_1_equality_nested_collections() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (vector (vector 1)) (vector (vector 1.0)))",
        true,
        &spec_ref("7.2.1", "=", "nested vectors with cross-type elements"),
    );
    ctx.assert_bool(
        "(= (vector 1 (vector 2 3)) (vector 1.0 (vector 2.0 3.0)))",
        true,
        &spec_ref("7.2.1", "=", "deeply nested cross-type"),
    );
}

/// Spec 7.2.1: Map equality with semantic values
#[test]
fn test_7_2_1_equality_map_values() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (hash-map 1 2) (hash-map 1 2.0))",
        true,
        &spec_ref("7.2.1", "=", "map with cross-type numeric values"),
    );
}

/// Spec 7.2.1: NaN in collections
#[test]
fn test_7_2_1_equality_nan_in_collections() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (vector ##NaN) (vector ##NaN))",
        false,
        &spec_ref("7.2.1", "=", "NaN in vectors not equal"),
    );
}

/// Spec 7.2.1: Different length collections
#[test]
fn test_7_2_1_equality_different_lengths() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (vector 1 2) (vector 1 2 3))",
        false,
        &spec_ref("7.2.1", "=", "different length vectors"),
    );
    ctx.assert_bool(
        "(= '(1) '(1 2))",
        false,
        &spec_ref("7.2.1", "=", "different length lists"),
    );
}

/// Spec 7.2.1: Zero-argument comparisons return true (vacuously)
#[test]
fn test_7_2_1_comparison_zero_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(=)",
        true,
        &spec_ref("7.2.1", "=", "zero args returns true"),
    );
    ctx.assert_bool(
        "(<)",
        true,
        &spec_ref("7.2.1", "<", "zero args returns true"),
    );
    ctx.assert_bool(
        "(>)",
        true,
        &spec_ref("7.2.1", ">", "zero args returns true"),
    );
    ctx.assert_bool(
        "(<=)",
        true,
        &spec_ref("7.2.1", "<=", "zero args returns true"),
    );
    ctx.assert_bool(
        "(>=)",
        true,
        &spec_ref("7.2.1", ">=", "zero args returns true"),
    );
}

/// Spec 7.2.1: One-argument comparisons return true (vacuously)
#[test]
fn test_7_2_1_comparison_one_arg() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 42)",
        true,
        &spec_ref("7.2.1", "=", "one arg returns true"),
    );
    ctx.assert_bool(
        "(< 42)",
        true,
        &spec_ref("7.2.1", "<", "one arg returns true"),
    );
    ctx.assert_bool(
        "(> 42)",
        true,
        &spec_ref("7.2.1", ">", "one arg returns true"),
    );
    ctx.assert_bool(
        "(<= 42)",
        true,
        &spec_ref("7.2.1", "<=", "one arg returns true"),
    );
    ctx.assert_bool(
        "(>= 42)",
        true,
        &spec_ref("7.2.1", ">=", "one arg returns true"),
    );
}

/// Spec 7.2.1: One-argument comparisons evaluate the argument for side effects
#[test]
fn test_7_2_1_comparison_one_arg_evaluates() {
    let mut ctx = SpecTestContext::new();
    // Verify that the argument is evaluated (def has side effect of creating binding)
    // After evaluating (= (def x 42)), x should be defined with value 42
    ctx.eval("(= (def x 42))").unwrap();
    ctx.assert_int(
        "x",
        42,
        &spec_ref("7.2.1", "=", "one arg evaluates argument"),
    );
}

/// Spec 7.2.1: Multi-argument equality with all equal values
#[test]
fn test_7_2_1_equality_multi_arg_all_equal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 1 1 1)",
        true,
        &spec_ref("7.2.1", "=", "three equal integers"),
    );
    ctx.assert_bool(
        "(= 1 1 1 1 1)",
        true,
        &spec_ref("7.2.1", "=", "five equal integers"),
    );
}

/// Spec 7.2.1: Multi-argument equality with not all equal
#[test]
fn test_7_2_1_equality_multi_arg_not_equal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(= 1 1 2)", false, &spec_ref("7.2.1", "=", "last differs"));
    ctx.assert_bool(
        "(= 1 2 1)",
        false,
        &spec_ref("7.2.1", "=", "middle differs"),
    );
}

/// Spec 7.2.1: Multi-argument equality with cross-type numeric
#[test]
fn test_7_2_1_equality_multi_arg_cross_type() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 1 1.0 1)",
        true,
        &spec_ref("7.2.1", "=", "mixed int and float all equal"),
    );
}

// ============================================================================
// Section 7.2.2-7.2.5: Ordering Comparison Operators (<, >, <=, >=)
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
// Section 7.2.6: String Ordering Comparison
// Reference: docs/lonala/operators.md#722-less-than-
// ============================================================================

/// Spec 7.2.2: Less than for strings (lexicographic)
#[test]
fn test_7_2_2_less_than_strings() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(< \"a\" \"b\")",
        true,
        &spec_ref("7.2.2", "<", "a < b lexicographic"),
    );
    ctx.assert_bool(
        "(< \"A\" \"a\")",
        true,
        &spec_ref("7.2.2", "<", "A < a (UTF-8 byte order)"),
    );
    ctx.assert_bool(
        "(< \"apple\" \"banana\")",
        true,
        &spec_ref("7.2.2", "<", "apple < banana"),
    );
    ctx.assert_bool(
        "(< \"a\" \"a\")",
        false,
        &spec_ref("7.2.2", "<", "a < a is false"),
    );
    ctx.assert_bool(
        "(< \"b\" \"a\")",
        false,
        &spec_ref("7.2.2", "<", "b < a is false"),
    );
    ctx.assert_bool(
        "(< \"\" \"a\")",
        true,
        &spec_ref("7.2.2", "<", "empty string < a"),
    );
}

/// Spec 7.2.3: Greater than for strings (lexicographic)
#[test]
fn test_7_2_3_greater_than_strings() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(> \"b\" \"a\")",
        true,
        &spec_ref("7.2.3", ">", "b > a lexicographic"),
    );
    ctx.assert_bool(
        "(> \"a\" \"A\")",
        true,
        &spec_ref("7.2.3", ">", "a > A (UTF-8 byte order)"),
    );
    ctx.assert_bool(
        "(> \"banana\" \"apple\")",
        true,
        &spec_ref("7.2.3", ">", "banana > apple"),
    );
    ctx.assert_bool(
        "(> \"a\" \"a\")",
        false,
        &spec_ref("7.2.3", ">", "a > a is false"),
    );
    ctx.assert_bool(
        "(> \"a\" \"\")",
        true,
        &spec_ref("7.2.3", ">", "a > empty string"),
    );
}

/// Spec 7.2.4: Less than or equal for strings
#[test]
fn test_7_2_4_less_than_or_equal_strings() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(<= \"a\" \"b\")", true, &spec_ref("7.2.4", "<=", "a <= b"));
    ctx.assert_bool(
        "(<= \"a\" \"a\")",
        true,
        &spec_ref("7.2.4", "<=", "a <= a (equal)"),
    );
    ctx.assert_bool(
        "(<= \"b\" \"a\")",
        false,
        &spec_ref("7.2.4", "<=", "b <= a is false"),
    );
}

/// Spec 7.2.5: Greater than or equal for strings
#[test]
fn test_7_2_5_greater_than_or_equal_strings() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(>= \"b\" \"a\")", true, &spec_ref("7.2.5", ">=", "b >= a"));
    ctx.assert_bool(
        "(>= \"a\" \"a\")",
        true,
        &spec_ref("7.2.5", ">=", "a >= a (equal)"),
    );
    ctx.assert_bool(
        "(>= \"a\" \"b\")",
        false,
        &spec_ref("7.2.5", ">=", "a >= b is false"),
    );
}

/// Spec 7.2: Multi-argument string chaining
#[test]
fn test_7_2_comparison_chained_strings() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(< \"a\" \"b\" \"c\")",
        true,
        &spec_ref("7.2", "<", "a < b < c chain"),
    );
    ctx.assert_bool(
        "(< \"a\" \"c\" \"b\")",
        false,
        &spec_ref("7.2", "<", "a < c < b breaks chain (c > b)"),
    );
    ctx.assert_bool(
        "(> \"c\" \"b\" \"a\")",
        true,
        &spec_ref("7.2", ">", "c > b > a chain"),
    );
    ctx.assert_bool(
        "(<= \"a\" \"a\" \"b\")",
        true,
        &spec_ref("7.2", "<=", "a <= a <= b chain"),
    );
    ctx.assert_bool(
        "(>= \"c\" \"b\" \"b\")",
        true,
        &spec_ref("7.2", ">=", "c >= b >= b chain"),
    );
}

/// Spec 7.2: Cross-type numeric ordering
#[test]
fn test_7_2_comparison_cross_type_numeric() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(< 1 1.5)", true, &spec_ref("7.2", "<", "int < float"));
    ctx.assert_bool(
        "(< 1 1.5 2)",
        true,
        &spec_ref("7.2", "<", "int < float < int chain"),
    );
    ctx.assert_bool("(> 2.0 1)", true, &spec_ref("7.2", ">", "float > int"));
    ctx.assert_bool(
        "(<= 1 1.0)",
        true,
        &spec_ref("7.2", "<=", "int <= float (equal value)"),
    );
}

/// Spec 7.2: Type error for mixed comparable types
#[test]
fn test_7_2_comparison_type_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error_contains(
        "(< 1 \"a\")",
        "TypeError",
        &spec_ref("7.2", "<", "number vs string error"),
    );
    ctx.assert_error_contains(
        "(< \"a\" 1)",
        "TypeError",
        &spec_ref("7.2", "<", "string vs number error"),
    );
    ctx.assert_error_contains(
        "(< 1 2 \"a\")",
        "TypeError",
        &spec_ref("7.2", "<", "mixed types in chain error"),
    );
    ctx.assert_error_contains(
        "(< nil 1)",
        "TypeError",
        &spec_ref("7.2", "<", "nil not comparable"),
    );
}

// ============================================================================
// Section 7.4: Logical Operators
// Reference: docs/lonala.md#74-logical-operators
// ============================================================================

/// Spec 7.4.1: "Returns true if x is falsy, false otherwise"
#[test]
fn test_7_4_1_not_falsy_values() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(not false)",
        true,
        &spec_ref("7.4.1", "not", "not false is true"),
    );
    ctx.assert_bool(
        "(not nil)",
        true,
        &spec_ref("7.4.1", "not", "not nil is true"),
    );
}

/// Spec 7.4.1: Truthy values
#[test]
fn test_7_4_1_not_truthy_values() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(not true)",
        false,
        &spec_ref("7.4.1", "not", "not true is false"),
    );
    ctx.assert_bool(
        "(not 0)",
        false,
        &spec_ref("7.4.1", "not", "not 0 is false (0 is truthy)"),
    );
    ctx.assert_bool(
        "(not \"\")",
        false,
        &spec_ref("7.4.1", "not", "not \"\" is false (empty string is truthy)"),
    );
}
