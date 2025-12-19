// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 3 - Data Types
//!
//! Reference: docs/lonala.md#3-data-types
//!
//! Tests the semantic behavior of Lonala data types as specified.

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

// ============================================================================
// Section 3.6: String
// Reference: docs/lonala.md#36-string
// ============================================================================

/// Spec 3.6: String equality
#[test]
fn test_3_6_string_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= \"hello\" \"hello\")",
        true,
        &spec_ref("3.6", "String", "identical strings are equal"),
    );
    ctx.assert_bool(
        "(= \"hello\" \"world\")",
        false,
        &spec_ref("3.6", "String", "different strings are not equal"),
    );
}

/// Spec 3.6: Empty string
#[test]
fn test_3_6_empty_string() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string("\"\"", "", &spec_ref("3.6", "String", "empty string"));
}

// ============================================================================
// Section 3.7: Binary
// Reference: docs/lonala.md#37-binary
// ============================================================================

/// [IGNORED] Spec 3.7: make-binary allocates zeroed buffer
/// Tracking: Binary type not yet implemented
#[test]
#[ignore]
fn test_3_7_make_binary() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_binary(
        "(make-binary 4)",
        &spec_ref("3.7", "Binary", "allocate 4-byte buffer"),
    );
}

/// [IGNORED] Spec 3.7: binary-get retrieves byte at index
/// Tracking: Binary type not yet implemented
#[test]
#[ignore]
fn test_3_7_binary_get() {
    let mut ctx = SpecTestContext::new();
    // Buffer is zeroed by default
    ctx.assert_int(
        "(binary-get (make-binary 4) 0)",
        0,
        &spec_ref("3.7", "Binary", "get byte at index 0"),
    );
}

/// [IGNORED] Spec 3.7: binary-set modifies byte at index
/// Tracking: Binary type not yet implemented
#[test]
#[ignore]
fn test_3_7_binary_set() {
    let mut ctx = SpecTestContext::new();
    // Set byte and read it back
    let _res = ctx.eval("(def buf (make-binary 4))").unwrap();
    let _res = ctx.eval("(binary-set buf 0 0xFF)").unwrap();
    ctx.assert_int(
        "(binary-get buf 0)",
        255,
        &spec_ref("3.7", "Binary", "set byte returns 255"),
    );
}

/// [IGNORED] Spec 3.7: binary-len returns buffer length
/// Tracking: Binary type not yet implemented
#[test]
#[ignore]
fn test_3_7_binary_len() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(binary-len (make-binary 1024))",
        1024,
        &spec_ref("3.7", "Binary", "buffer length"),
    );
}

/// [IGNORED] Spec 3.7: binary-slice creates zero-copy view
/// Tracking: Binary type not yet implemented
#[test]
#[ignore]
fn test_3_7_binary_slice() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def buf (make-binary 20))").unwrap();
    ctx.assert_int(
        "(binary-len (binary-slice buf 10 20))",
        10,
        &spec_ref("3.7", "Binary", "slice length"),
    );
}

/// [IGNORED] Spec 3.7: Binary is mutable (unlike other Lonala types)
/// Tracking: Binary type not yet implemented
#[test]
#[ignore]
fn test_3_7_binary_mutable() {
    let mut ctx = SpecTestContext::new();
    // Create buffer, modify, check modification persists
    let _res = ctx.eval("(def buf (make-binary 4))").unwrap();
    let _res = ctx.eval("(binary-set buf 1 42)").unwrap();
    ctx.assert_int(
        "(binary-get buf 1)",
        42,
        &spec_ref("3.7", "Binary", "mutations persist"),
    );
}

// ============================================================================
// Section 3.8: List
// Reference: docs/lonala.md#38-list
// ============================================================================

/// Spec 3.8: Quoted list is data, not code
#[test]
fn test_3_8_quoted_list() {
    let mut ctx = SpecTestContext::new();
    // Verify quoted list produces exactly (1 2 3)
    ctx.assert_list_len(
        "'(1 2 3)",
        3,
        &spec_ref("3.8", "List", "quoted list has 3 elements"),
    );
    ctx.assert_int(
        "(first '(1 2 3))",
        1,
        &spec_ref("3.8", "List", "first element is 1"),
    );
    ctx.assert_int(
        "(first (rest '(1 2 3)))",
        2,
        &spec_ref("3.8", "List", "second element is 2"),
    );
    ctx.assert_int(
        "(first (rest (rest '(1 2 3))))",
        3,
        &spec_ref("3.8", "List", "third element is 3"),
    );
}

/// Spec 3.8: Empty list
#[test]
fn test_3_8_empty_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_len(
        "'()",
        0,
        &spec_ref("3.8", "List", "empty quoted list has 0 elements"),
    );
}

// ============================================================================
// Section 3.9: Vector
// Reference: docs/lonala.md#39-vector
// ============================================================================

/// [IGNORED] Spec 3.9: Vector literals
/// Tracking: Vector literal compilation planned
#[test]
#[ignore]
fn test_3_9_vector_literal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_eq(
        "[1 2 3]",
        "[1 2 3]",
        &spec_ref("3.9", "Vector", "vector literal yields [1 2 3]"),
    );
    ctx.assert_vector_eq(
        "[]",
        "[]",
        &spec_ref("3.9", "Vector", "empty vector literal yields []"),
    );
}

// ============================================================================
// Section 3.10: Map
// Reference: docs/lonala.md#310-map
// ============================================================================

/// [IGNORED] Spec 3.10: Map literals
/// Tracking: Map literal compilation planned
#[test]
#[ignore]
fn test_3_10_map_literal() {
    let mut _ctx = SpecTestContext::new();
    // Map literal tests when implemented
}

// ============================================================================
// Section 3.11: Function
// Reference: docs/lonala.md#311-function
// ============================================================================

/// Spec 3.11: Functions are first-class values
#[test]
fn test_3_11_function_first_class() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_function(
        "(fn [x] x)",
        &spec_ref("3.11", "Function", "fn creates a function value"),
    );
}

// ============================================================================
// Section 3.12: Truthiness
// Reference: docs/lonala.md#312-truthiness
// ============================================================================

/// Spec 3.12: "nil is falsy, false is falsy, everything else is truthy"
#[test]
fn test_3_12_truthiness_nil_false() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if nil 1 2)",
        2,
        &spec_ref("3.12", "Truthiness", "nil is falsy"),
    );
    ctx.assert_int(
        "(if false 1 2)",
        2,
        &spec_ref("3.12", "Truthiness", "false is falsy"),
    );
}

/// Spec 3.12: "0 is truthy, empty string is truthy, empty collections are truthy"
#[test]
fn test_3_12_truthiness_zero_and_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if 0 1 2)",
        1,
        &spec_ref("3.12", "Truthiness", "0 is truthy"),
    );
    ctx.assert_int(
        "(if \"\" 1 2)",
        1,
        &spec_ref("3.12", "Truthiness", "empty string is truthy"),
    );
    ctx.assert_int(
        "(if '() 1 2)",
        1,
        &spec_ref("3.12", "Truthiness", "empty list is truthy"),
    );
}

// ============================================================================
// Section 3.13: Equality
// Reference: docs/lonala.md#313-equality
// ============================================================================

/// Spec 3.13: Structural equality for most types
#[test]
fn test_3_13_structural_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 1 1)",
        true,
        &spec_ref("3.13", "Equality", "integer equality"),
    );
    ctx.assert_bool(
        "(= \"abc\" \"abc\")",
        true,
        &spec_ref("3.13", "Equality", "string equality"),
    );
}

/// Spec 3.13: "Numbers of different types can be equal if they represent the same value"
#[test]
fn test_3_13_cross_type_numeric_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= 1 1.0)",
        true,
        &spec_ref("3.13", "Equality", "integer equals float when same value"),
    );
}

// ============================================================================
// Section 3.14: Metadata
// Reference: docs/lonala.md#314-metadata
// ============================================================================

/// [IGNORED] Spec 3.14: meta returns nil for values without metadata
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_14_meta_returns_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(meta [1 2 3])",
        &spec_ref("3.14", "Metadata", "meta returns nil when no metadata"),
    );
}

/// [IGNORED] Spec 3.14: with-meta attaches metadata
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_14_with_meta() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def v (with-meta [1 2 3] {:source \"test\"}))")
        .unwrap();
    ctx.assert_map(
        "(meta v)",
        &spec_ref("3.14", "Metadata", "with-meta attaches map"),
    );
}

/// [IGNORED] Spec 3.14: Metadata does NOT affect equality
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_14_metadata_equality() {
    let mut ctx = SpecTestContext::new();
    // Two values that differ only in metadata should be equal
    ctx.assert_bool(
        "(= [1 2 3] (with-meta [1 2 3] {:foo :bar}))",
        true,
        &spec_ref("3.14", "Metadata", "metadata does not affect equality"),
    );
}

/// [IGNORED] Spec 3.14: vary-meta transforms metadata
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_14_vary_meta() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def v (with-meta [1 2 3] {:a 1}))").unwrap();
    let _res = ctx.eval("(def v2 (vary-meta v assoc :b 2))").unwrap();
    // v2's metadata should have both :a and :b
    ctx.assert_map(
        "(meta v2)",
        &spec_ref("3.14", "Metadata", "vary-meta transforms metadata"),
    );
}

/// [IGNORED] Spec 3.14: Primitives do not support metadata
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_14_primitives_no_metadata() {
    let mut ctx = SpecTestContext::new();
    // Numbers, strings, nil, booleans cannot have metadata
    ctx.assert_error(
        "(with-meta 42 {:foo :bar})",
        &spec_ref("3.14", "Metadata", "integers cannot have metadata"),
    );
}
