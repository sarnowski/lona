// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 4 - Literals
//!
//! Reference: docs/lonala.md#4-literals
//!
//! Tests the syntax for writing literal values in source code.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 4.1.1: Integer Literals
// Reference: docs/lonala.md#411-integer-literals
// ============================================================================

/// Spec 4.1.1: Decimal integer literals
#[test]
fn test_4_1_1_decimal_integer() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("42", 42, &spec_ref("4.1.1", "Integer", "decimal literal"));
    ctx.assert_int("0", 0, &spec_ref("4.1.1", "Integer", "zero"));
    ctx.assert_int(
        "-17",
        -17,
        &spec_ref("4.1.1", "Integer", "negative decimal"),
    );
}

/// Spec 4.1.1: Hexadecimal integer literals
#[test]
fn test_4_1_1_hex_integer() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "0xFF",
        255,
        &spec_ref("4.1.1", "Integer", "hexadecimal uppercase"),
    );
    ctx.assert_int(
        "0xff",
        255,
        &spec_ref("4.1.1", "Integer", "hexadecimal lowercase"),
    );
    ctx.assert_int(
        "0x10",
        16,
        &spec_ref("4.1.1", "Integer", "hexadecimal 0x10"),
    );
}

/// Spec 4.1.1: Binary integer literals
#[test]
fn test_4_1_1_binary_integer() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "0b1010",
        10,
        &spec_ref("4.1.1", "Integer", "binary literal"),
    );
    ctx.assert_int(
        "0b11111111",
        255,
        &spec_ref("4.1.1", "Integer", "binary 255"),
    );
}

/// Spec 4.1.1: Octal integer literals
#[test]
fn test_4_1_1_octal_integer() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("0o755", 493, &spec_ref("4.1.1", "Integer", "octal literal"));
    ctx.assert_int("0o10", 8, &spec_ref("4.1.1", "Integer", "octal 8"));
}

// ============================================================================
// Section 4.1.2: Float Literals
// Reference: docs/lonala.md#412-float-literals
// ============================================================================

/// Spec 4.1.2: Decimal float literals
#[test]
fn test_4_1_2_decimal_float() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_float("3.14", 3.14, &spec_ref("4.1.2", "Float", "decimal float"));
    ctx.assert_float("-0.5", -0.5, &spec_ref("4.1.2", "Float", "negative float"));
    ctx.assert_float(
        "0.5",
        0.5,
        &spec_ref("4.1.2", "Float", "leading zero float"),
    );
}

/// Spec 4.1.2: Scientific notation float literals
#[test]
fn test_4_1_2_scientific_notation() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_float(
        "1e10",
        1e10,
        &spec_ref("4.1.2", "Float", "scientific notation"),
    );
    ctx.assert_float(
        "1E10",
        1e10,
        &spec_ref("4.1.2", "Float", "scientific notation uppercase"),
    );
    ctx.assert_float(
        "1.5e-3",
        0.0015,
        &spec_ref("4.1.2", "Float", "scientific notation negative exp"),
    );
}

/// Spec 4.1.2: Special float values
#[test]
fn test_4_1_2_special_floats() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_float(
        "##Inf",
        f64::INFINITY,
        &spec_ref("4.1.2", "Float", "positive infinity"),
    );
    ctx.assert_float(
        "##-Inf",
        f64::NEG_INFINITY,
        &spec_ref("4.1.2", "Float", "negative infinity"),
    );
    // NaN requires special handling since NaN != NaN
    let result = ctx.eval("##NaN").unwrap();
    match result {
        lona_core::value::Value::Float(float_val) => {
            assert!(float_val.is_nan(), "[Spec 4.1.2 Float] ##NaN should be NaN");
        }
        _ => panic!("[Spec 4.1.2 Float] ##NaN should evaluate to a float"),
    }
}

// ============================================================================
// Section 4.1.3: Ratio Literals
// Reference: docs/lonala.md#413-ratio-literals
// ============================================================================

/// [IGNORED] Spec 4.1.3: Basic ratio literals
/// Tracking: Ratio literals not yet implemented as direct syntax
#[test]
#[ignore]
fn test_4_1_3_basic_ratio() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio("1/3", 1, 3, &spec_ref("4.1.3", "Ratio", "basic ratio"));
    ctx.assert_ratio(
        "22/7",
        22,
        7,
        &spec_ref("4.1.3", "Ratio", "approximation of pi"),
    );
}

/// [IGNORED] Spec 4.1.3: Negative ratio
/// Tracking: Ratio literals not yet implemented as direct syntax
#[test]
#[ignore]
fn test_4_1_3_negative_ratio() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio("-1/2", -1, 2, &spec_ref("4.1.3", "Ratio", "negative ratio"));
}

/// [IGNORED] Spec 4.1.3: Ratio normalization - "4/2 normalizes to 2 (integer)"
/// Tracking: Ratio literals not yet implemented as direct syntax
#[test]
#[ignore]
fn test_4_1_3_ratio_normalizes_to_integer() {
    let mut ctx = SpecTestContext::new();
    // 4/2 should normalize to integer 2, not ratio
    ctx.assert_int(
        "4/2",
        2,
        &spec_ref("4.1.3", "Ratio", "4/2 normalizes to integer 2"),
    );
}

// ============================================================================
// Section 4.2: String Literals
// Reference: docs/lonala.md#42-string-literals
// ============================================================================

/// Spec 4.2: Basic string literals
#[test]
fn test_4_2_basic_string() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "\"hello world\"",
        "hello world",
        &spec_ref("4.2", "String", "basic string"),
    );
}

/// Spec 4.2: Escape sequences
#[test]
fn test_4_2_escape_sequences() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "\"line1\\nline2\"",
        "line1\nline2",
        &spec_ref("4.2", "String", "newline escape"),
    );
    ctx.assert_string(
        "\"tab\\there\"",
        "tab\there",
        &spec_ref("4.2", "String", "tab escape"),
    );
    ctx.assert_string(
        "\"quote: \\\"hi\\\"\"",
        "quote: \"hi\"",
        &spec_ref("4.2", "String", "quote escape"),
    );
    ctx.assert_string(
        "\"backslash: \\\\\"",
        "backslash: \\",
        &spec_ref("4.2", "String", "backslash escape"),
    );
}

/// Spec 4.2: Empty string
#[test]
fn test_4_2_empty_string() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string("\"\"", "", &spec_ref("4.2", "String", "empty string"));
}

// ============================================================================
// Section 4.3: Boolean Literals
// Reference: docs/lonala.md#43-boolean-literals
// ============================================================================

/// Spec 4.3: Boolean literals
#[test]
fn test_4_3_boolean_literals() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("true", true, &spec_ref("4.3", "Boolean", "true literal"));
    ctx.assert_bool("false", false, &spec_ref("4.3", "Boolean", "false literal"));
}

// ============================================================================
// Section 4.4: Nil Literal
// Reference: docs/lonala.md#44-nil-literal
// ============================================================================

/// Spec 4.4: Nil literal
#[test]
fn test_4_4_nil_literal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil("nil", &spec_ref("4.4", "Nil", "nil literal"));
}

// ============================================================================
// Section 4.5.1: List Literals
// Reference: docs/lonala.md#451-list-literals
// ============================================================================

/// Spec 4.5.1: Quoted list literals
#[test]
fn test_4_5_1_quoted_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list("'()", &spec_ref("4.5.1", "List", "empty quoted list"));
    ctx.assert_list(
        "'(1 2 3)",
        &spec_ref("4.5.1", "List", "quoted list of integers"),
    );
}

// ============================================================================
// Section 4.5.2: Vector Literals
// Reference: docs/lonala.md#452-vector-literals
// ============================================================================

/// Spec 4.5.2: Empty vector literal
#[test]
fn test_4_5_2_empty_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector("[]", &spec_ref("4.5.2", "Vector", "empty vector"));
}

/// Spec 4.5.2: Vector literal with elements
#[test]
fn test_4_5_2_vector_with_elements() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector(
        "[1 2 3]",
        &spec_ref("4.5.2", "Vector", "vector of integers"),
    );
}

/// Spec 4.5.2: Nested vector literal
#[test]
fn test_4_5_2_nested_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector("[1 [2 3] 4]", &spec_ref("4.5.2", "Vector", "nested vector"));
}

// ============================================================================
// Section 4.5.3: Map Literals
// Reference: docs/lonala.md#453-map-literals
// ============================================================================

/// Spec 4.5.3: Empty map literal
#[test]
fn test_4_5_3_empty_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map("{}", &spec_ref("4.5.3", "Map", "empty map"));
}

/// Spec 4.5.3: Map literal with entries
#[test]
fn test_4_5_3_map_with_entries() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map(
        "{:a 1 :b 2}",
        &spec_ref("4.5.3", "Map", "map with keyword keys"),
    );
}

/// Spec 4.5.3: Nested map literal
#[test]
fn test_4_5_3_nested_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map("{:a {:b 1}}", &spec_ref("4.5.3", "Map", "nested map"));
}

/// Spec 4.5.3: Map with mixed key types
#[test]
fn test_4_5_3_map_mixed_keys() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map(
        "{:key 1 \"str\" 2 42 3}",
        &spec_ref("4.5.3", "Map", "map with mixed key types"),
    );
}

// ============================================================================
// Section 4.5.4: Set Literals
// Reference: docs/lonala.md#454-set-literals
// ============================================================================

/// Spec 4.5.4: Empty set literal
#[test]
fn test_4_5_4_empty_set() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_set("#{}", &spec_ref("4.5.4", "Set", "empty set"));
}

/// Spec 4.5.4: Set literal with elements
#[test]
fn test_4_5_4_set_with_elements() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_set("#{1 2 3}", &spec_ref("4.5.4", "Set", "set of integers"));
}

/// Spec 4.5.4: Set literal with mixed types
#[test]
fn test_4_5_4_set_mixed_types() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_set(
        "#{:a \"b\" 3}",
        &spec_ref("4.5.4", "Set", "set with mixed types"),
    );
}

// ============================================================================
// Section 4.5.5: Mixed Collection Literals
// Reference: Combined test of collection interoperability
// ============================================================================

/// Collection literals can be nested
#[test]
fn test_4_5_5_nested_collections() {
    let mut ctx = SpecTestContext::new();
    // Vector containing a map
    ctx.assert_vector(
        "[{:a 1}]",
        &spec_ref("4.5", "Collections", "vector containing map"),
    );
    // Map containing a vector
    ctx.assert_map(
        "{:v [1 2 3]}",
        &spec_ref("4.5", "Collections", "map containing vector"),
    );
    // Vector containing a set
    ctx.assert_vector(
        "[#{1 2}]",
        &spec_ref("4.5", "Collections", "vector containing set"),
    );
}

// ============================================================================
// Section 4.6: Symbol Literals
// Reference: docs/lonala.md#46-symbol-literals
// ============================================================================

/// Spec 4.6: Quoted symbol literals
#[test]
fn test_4_6_quoted_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol("'foo", &spec_ref("4.6", "Symbol", "quoted simple symbol"));
    ctx.assert_symbol("'+", &spec_ref("4.6", "Symbol", "quoted operator symbol"));
}

// ============================================================================
// Section 4.7: Keyword Literals
// Reference: docs/lonala.md#47-keyword-literals
// ============================================================================

/// Spec 4.7: Simple keyword literal
#[test]
fn test_4_7_keyword_literal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_keyword(":foo", &spec_ref("4.7", "Keyword", "simple keyword"));
}

/// Spec 4.7: Keyword with hyphens
#[test]
fn test_4_7_keyword_with_hyphens() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_keyword(
        ":my-key",
        &spec_ref("4.7", "Keyword", "keyword with hyphens"),
    );
}

/// Spec 4.7: Keywords are self-evaluating
#[test]
fn test_4_7_keyword_self_evaluating() {
    let mut ctx = SpecTestContext::new();
    // Keywords evaluate to themselves
    ctx.assert_keyword_eq(
        ":test",
        "test",
        &spec_ref("4.7", "Keyword", "self-evaluating"),
    );
}
