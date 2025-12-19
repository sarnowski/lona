// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Type Predicates.
//!
//! Section 9.2 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.2: Type Predicates
// Reference: docs/lonala.md#92-type-predicates
// ============================================================================

/// [IGNORED] Spec 9.2: nil? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_nil_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(nil? nil)", true, &spec_ref("9.2", "nil?", "nil is nil"));
    ctx.assert_bool(
        "(nil? false)",
        false,
        &spec_ref("9.2", "nil?", "false is not nil"),
    );
    ctx.assert_bool("(nil? 0)", false, &spec_ref("9.2", "nil?", "0 is not nil"));
}

/// [IGNORED] Spec 9.2: boolean? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_boolean_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(boolean? true)",
        true,
        &spec_ref("9.2", "boolean?", "true is boolean"),
    );
    ctx.assert_bool(
        "(boolean? false)",
        true,
        &spec_ref("9.2", "boolean?", "false is boolean"),
    );
    ctx.assert_bool(
        "(boolean? nil)",
        false,
        &spec_ref("9.2", "boolean?", "nil is not boolean"),
    );
}

/// [IGNORED] Spec 9.2: integer? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_integer_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(integer? 42)",
        true,
        &spec_ref("9.2", "integer?", "42 is integer"),
    );
    ctx.assert_bool(
        "(integer? -17)",
        true,
        &spec_ref("9.2", "integer?", "-17 is integer"),
    );
    ctx.assert_bool(
        "(integer? 3.14)",
        false,
        &spec_ref("9.2", "integer?", "3.14 is not integer"),
    );
}

/// [IGNORED] Spec 9.2: float? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_float_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(float? 3.14)",
        true,
        &spec_ref("9.2", "float?", "3.14 is float"),
    );
    ctx.assert_bool(
        "(float? ##Inf)",
        true,
        &spec_ref("9.2", "float?", "##Inf is float"),
    );
    ctx.assert_bool(
        "(float? ##NaN)",
        true,
        &spec_ref("9.2", "float?", "##NaN is float"),
    );
    ctx.assert_bool(
        "(float? 42)",
        false,
        &spec_ref("9.2", "float?", "42 is not float"),
    );
}

/// [IGNORED] Spec 9.2: ratio? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_ratio_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(ratio? 1/3)",
        true,
        &spec_ref("9.2", "ratio?", "1/3 is ratio"),
    );
    ctx.assert_bool(
        "(ratio? 42)",
        false,
        &spec_ref("9.2", "ratio?", "42 is not ratio"),
    );
    ctx.assert_bool(
        "(ratio? 3.14)",
        false,
        &spec_ref("9.2", "ratio?", "3.14 is not ratio"),
    );
}

/// [IGNORED] Spec 9.2: string? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_string_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(string? \"hello\")",
        true,
        &spec_ref("9.2", "string?", "\"hello\" is string"),
    );
    ctx.assert_bool(
        "(string? \"\")",
        true,
        &spec_ref("9.2", "string?", "empty string is string"),
    );
    ctx.assert_bool(
        "(string? 42)",
        false,
        &spec_ref("9.2", "string?", "42 is not string"),
    );
    ctx.assert_bool(
        "(string? 'foo)",
        false,
        &spec_ref("9.2", "string?", "symbol is not string"),
    );
}

/// [IGNORED] Spec 9.2: symbol? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_symbol_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(symbol? 'foo)",
        true,
        &spec_ref("9.2", "symbol?", "'foo is symbol"),
    );
    ctx.assert_bool(
        "(symbol? \"foo\")",
        false,
        &spec_ref("9.2", "symbol?", "string is not symbol"),
    );
    ctx.assert_bool(
        "(symbol? :foo)",
        false,
        &spec_ref("9.2", "symbol?", "keyword is not symbol"),
    );
}

/// [IGNORED] Spec 9.2: keyword? predicate
/// Tracking: Requires keyword type
#[test]
#[ignore]
fn test_9_2_keyword_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(keyword? :foo)",
        true,
        &spec_ref("9.2", "keyword?", ":foo is keyword"),
    );
    ctx.assert_bool(
        "(keyword? 'foo)",
        false,
        &spec_ref("9.2", "keyword?", "symbol is not keyword"),
    );
    ctx.assert_bool(
        "(keyword? \"foo\")",
        false,
        &spec_ref("9.2", "keyword?", "string is not keyword"),
    );
}

/// [IGNORED] Spec 9.2: list? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_list_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(list? '(1 2 3))",
        true,
        &spec_ref("9.2", "list?", "quoted list is list"),
    );
    ctx.assert_bool(
        "(list? '())",
        true,
        &spec_ref("9.2", "list?", "empty list is list"),
    );
    ctx.assert_bool(
        "(list? (vector 1 2))",
        false,
        &spec_ref("9.2", "list?", "vector is not list"),
    );
}

/// [IGNORED] Spec 9.2: vector? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_vector_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(vector? (vector 1 2 3))",
        true,
        &spec_ref("9.2", "vector?", "vector is vector"),
    );
    ctx.assert_bool(
        "(vector? (vector))",
        true,
        &spec_ref("9.2", "vector?", "empty vector is vector"),
    );
    ctx.assert_bool(
        "(vector? '(1 2))",
        false,
        &spec_ref("9.2", "vector?", "list is not vector"),
    );
}

/// [IGNORED] Spec 9.2: map? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_map_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(map? (hash-map 'a 1))",
        true,
        &spec_ref("9.2", "map?", "hash-map is map"),
    );
    ctx.assert_bool(
        "(map? (hash-map))",
        true,
        &spec_ref("9.2", "map?", "empty map is map"),
    );
    ctx.assert_bool(
        "(map? (vector 1 2))",
        false,
        &spec_ref("9.2", "map?", "vector is not map"),
    );
}

/// [IGNORED] Spec 9.2: set? predicate
/// Tracking: Requires set type
#[test]
#[ignore]
fn test_9_2_set_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(set? #{1 2 3})",
        true,
        &spec_ref("9.2", "set?", "set is set"),
    );
    ctx.assert_bool(
        "(set? #{})",
        true,
        &spec_ref("9.2", "set?", "empty set is set"),
    );
    ctx.assert_bool(
        "(set? (vector 1 2))",
        false,
        &spec_ref("9.2", "set?", "vector is not set"),
    );
}

/// [IGNORED] Spec 9.2: fn? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_fn_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool("(fn? +)", true, &spec_ref("9.2", "fn?", "+ is a function"));
    ctx.assert_bool(
        "(fn? (fn [x] x))",
        true,
        &spec_ref("9.2", "fn?", "lambda is a function"),
    );
    ctx.assert_bool(
        "(fn? 42)",
        false,
        &spec_ref("9.2", "fn?", "42 is not a function"),
    );
}

/// [IGNORED] Spec 9.2: binary? predicate
/// Tracking: Requires binary type
#[test]
#[ignore]
fn test_9_2_binary_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(binary? (make-binary 4))",
        true,
        &spec_ref("9.2", "binary?", "binary is binary"),
    );
    ctx.assert_bool(
        "(binary? \"hello\")",
        false,
        &spec_ref("9.2", "binary?", "string is not binary"),
    );
}

/// [IGNORED] Spec 9.2: coll? predicate - true for any collection
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_coll_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(coll? '(1 2 3))",
        true,
        &spec_ref("9.2", "coll?", "list is collection"),
    );
    ctx.assert_bool(
        "(coll? (vector 1 2))",
        true,
        &spec_ref("9.2", "coll?", "vector is collection"),
    );
    ctx.assert_bool(
        "(coll? (hash-map 'a 1))",
        true,
        &spec_ref("9.2", "coll?", "map is collection"),
    );
    ctx.assert_bool(
        "(coll? #{1 2})",
        true,
        &spec_ref("9.2", "coll?", "set is collection"),
    );
    ctx.assert_bool(
        "(coll? 42)",
        false,
        &spec_ref("9.2", "coll?", "integer is not collection"),
    );
}

/// [IGNORED] Spec 9.2: seq? predicate - true for sequences
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_9_2_seq_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(seq? '(1 2 3))",
        true,
        &spec_ref("9.2", "seq?", "list is sequence"),
    );
    ctx.assert_bool(
        "(seq? 42)",
        false,
        &spec_ref("9.2", "seq?", "integer is not sequence"),
    );
}
