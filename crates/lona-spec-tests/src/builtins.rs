// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 9 - Built-in Functions
//!
//! Reference: docs/lonala.md#9-built-in-functions
//!
//! Tests built-in functions (primitives/natives) implemented in Rust.

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
        "(integer? 3.14)",
        false,
        &spec_ref("9.2", "integer?", "3.14 is not integer"),
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
        "(list? [1 2])",
        false,
        &spec_ref("9.2", "list?", "vector is not list"),
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
        "(fn? 42)",
        false,
        &spec_ref("9.2", "fn?", "42 is not a function"),
    );
}

// ============================================================================
// Section 9.3: Collection Primitives (Implemented)
// Reference: docs/lonala.md#93-collection-primitives
// ============================================================================

/// Spec 9.3.1: cons - Prepend element to list
#[test]
fn test_9_3_1_cons() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(cons 1 '(2 3))",
        "'(1 2 3)",
        &spec_ref("9.3.1", "cons", "prepend to list yields (1 2 3)"),
    );
    ctx.assert_list_eq(
        "(cons 1 '())",
        "'(1)",
        &spec_ref("9.3.1", "cons", "prepend to empty list yields (1)"),
    );
}

/// Spec 9.3.1: first - Get first element
#[test]
fn test_9_3_1_first() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(first '(1 2 3))",
        1,
        &spec_ref("9.3.1", "first", "first of list"),
    );
}

/// Spec 9.3.1: first - Empty list returns nil
#[test]
fn test_9_3_1_first_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(first '())",
        &spec_ref("9.3.1", "first", "first of empty list is nil"),
    );
}

/// Spec 9.3.1: rest - Get all but first element
#[test]
fn test_9_3_1_rest() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(rest '(1 2 3))",
        "'(2 3)",
        &spec_ref("9.3.1", "rest", "rest of list yields (2 3)"),
    );
}

/// Spec 9.3.1: rest - Returns empty list for single-element list
#[test]
fn test_9_3_1_rest_single() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(rest '(1))",
        "'()",
        &spec_ref("9.3.1", "rest", "rest of single-element list yields ()"),
    );
}

/// Spec 9.3.1: rest - Returns empty list for empty list
#[test]
fn test_9_3_1_rest_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(rest '())",
        "'()",
        &spec_ref("9.3.1", "rest", "rest of empty list yields ()"),
    );
}

/// Spec 9.15: list - Create a list (standard library function)
#[test]
fn test_9_15_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(list 1 2 3)",
        "'(1 2 3)",
        &spec_ref("9.15", "list", "create list yields (1 2 3)"),
    );
    ctx.assert_list_eq(
        "(list)",
        "'()",
        &spec_ref("9.15", "list", "create empty list yields ()"),
    );
}

/// Spec 9.15: concat - Concatenate lists (standard library function)
#[test]
fn test_9_15_concat() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(concat '(1 2) '(3 4))",
        "'(1 2 3 4)",
        &spec_ref("9.15", "concat", "concatenate two lists yields (1 2 3 4)"),
    );
}

/// Spec 9.15: concat - With empty lists
#[test]
fn test_9_15_concat_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(concat '() '(1 2))",
        "'(1 2)",
        &spec_ref("9.15", "concat", "concat empty with non-empty yields (1 2)"),
    );
    ctx.assert_list_eq(
        "(concat '(1 2) '())",
        "'(1 2)",
        &spec_ref("9.15", "concat", "concat non-empty with empty yields (1 2)"),
    );
}

/// Spec 9.15: vector - Create a vector (standard library function)
#[test]
fn test_9_15_vector() {
    let mut ctx = SpecTestContext::new();
    // Use (vector ...) for both source and expected since vector literals aren't compiled yet
    ctx.assert_vector_eq(
        "(vector 1 2 3)",
        "(vector 1 2 3)",
        &spec_ref("9.15", "vector", "create vector yields [1 2 3]"),
    );
    ctx.assert_vector_len(
        "(vector 1 2 3)",
        3,
        &spec_ref("9.15", "vector", "vector has 3 elements"),
    );
    ctx.assert_vector_len(
        "(vector)",
        0,
        &spec_ref("9.15", "vector", "empty vector has 0 elements"),
    );
}

/// Spec 9.15: vec - Convert to vector (standard library function)
#[test]
fn test_9_15_vec() {
    let mut ctx = SpecTestContext::new();
    // Use (vector ...) for expected since vector literals aren't compiled yet
    ctx.assert_vector_eq(
        "(vec '(1 2 3))",
        "(vector 1 2 3)",
        &spec_ref("9.15", "vec", "convert list to vector yields [1 2 3]"),
    );
}

/// Spec 9.15: hash-map - Create a map (standard library function)
#[test]
fn test_9_15_hash_map() {
    let mut ctx = SpecTestContext::new();
    // hash-map takes alternating key-value pairs - verify type and length
    ctx.assert_map_len(
        "(hash-map 'a 1 'b 2)",
        2,
        &spec_ref("9.15", "hash-map", "creates map with 2 entries"),
    );
    // Test equality with another map containing the same contents
    ctx.assert_bool(
        "(= (hash-map 'a 1 'b 2) (hash-map 'a 1 'b 2))",
        true,
        &spec_ref("9.15", "hash-map", "maps with same content are equal"),
    );
    ctx.assert_bool(
        "(= (hash-map 'a 1 'b 2) (hash-map 'a 1))",
        false,
        &spec_ref(
            "9.15",
            "hash-map",
            "maps with different content are not equal",
        ),
    );
}

/// Spec 9.15: hash-map - Empty map (standard library function)
#[test]
fn test_9_15_hash_map_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_len(
        "(hash-map)",
        0,
        &spec_ref("9.15", "hash-map", "creates empty map with 0 entries"),
    );
}

// ============================================================================
// Section 9.4: Binary Operations
// Reference: docs/lonala.md#94-binary-operations
// ============================================================================

/// [IGNORED] Spec 9.4: make-binary allocates zeroed buffer
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_make_binary() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_binary(
        "(make-binary 4)",
        &spec_ref("9.4", "make-binary", "allocate zeroed buffer"),
    );
}

/// [IGNORED] Spec 9.4: binary-len returns buffer length
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_binary_len() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(binary-len (make-binary 4))",
        4,
        &spec_ref("9.4", "binary-len", "get buffer length"),
    );
}

/// [IGNORED] Spec 9.4: binary-get/binary-set operations
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_binary_get_set() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def buf (make-binary 4))").unwrap();
    let _res = ctx.eval("(binary-set buf 0 0xFF)").unwrap();
    ctx.assert_int(
        "(binary-get buf 0)",
        255,
        &spec_ref("9.4", "binary-get/set", "get byte at index"),
    );
}

/// [IGNORED] Spec 9.4: binary-slice zero-copy view
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_binary_slice() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def buf (make-binary 20))").unwrap();
    ctx.assert_int(
        "(binary-len (binary-slice buf 10 20))",
        10,
        &spec_ref("9.4", "binary-slice", "zero-copy slice"),
    );
}

/// [IGNORED] Spec 9.4: binary-copy! copies bytes
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_binary_copy() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def src (make-binary 4))").unwrap();
    let _res = ctx.eval("(def dst (make-binary 4))").unwrap();
    let _res = ctx.eval("(binary-set src 0 42)").unwrap();
    let _res = ctx.eval("(binary-copy! dst 0 src 0 1)").unwrap();
    ctx.assert_int(
        "(binary-get dst 0)",
        42,
        &spec_ref("9.4", "binary-copy!", "copy bytes between buffers"),
    );
}

// ============================================================================
// Section 9.5: Symbol Operations
// Reference: docs/lonala.md#95-symbol-operations
// ============================================================================

/// [IGNORED] Spec 9.5: symbol creates/interns a symbol
/// Tracking: Symbol operations not fully exposed yet
#[test]
#[ignore]
fn test_9_5_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(symbol \"foo\")",
        "foo",
        &spec_ref("9.5", "symbol", "create symbol 'foo' from string"),
    );
}

/// [IGNORED] Spec 9.5: gensym generates unique symbol
/// Tracking: gensym not yet implemented
#[test]
#[ignore]
fn test_9_5_gensym() {
    let mut ctx = SpecTestContext::new();
    // gensym should return a symbol
    ctx.assert_symbol(
        "(gensym)",
        &spec_ref("9.5", "gensym", "generate unique symbol"),
    );
}

/// [IGNORED] Spec 9.5: gensym with prefix
/// Tracking: gensym not yet implemented
#[test]
#[ignore]
fn test_9_5_gensym_prefix() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol(
        "(gensym \"temp\")",
        &spec_ref("9.5", "gensym", "generate symbol with prefix"),
    );
}

// ============================================================================
// Section 9.6: Metadata Operations
// Reference: docs/lonala.md#96-metadata-operations
// ============================================================================

/// [IGNORED] Spec 9.6: meta returns nil when no metadata
/// Tracking: Metadata operations not yet implemented
#[test]
#[ignore]
fn test_9_6_meta_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(meta [1 2 3])",
        &spec_ref("9.6", "meta", "returns nil when no metadata"),
    );
}

/// [IGNORED] Spec 9.6: with-meta attaches metadata
/// Tracking: Metadata operations not yet implemented
#[test]
#[ignore]
fn test_9_6_with_meta() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def v (with-meta [1 2 3] {:source \"test\"}))")
        .unwrap();
    ctx.assert_map(
        "(meta v)",
        &spec_ref("9.6", "with-meta", "attaches metadata map"),
    );
}

/// [IGNORED] Spec 9.6: vary-meta transforms metadata
/// Tracking: Metadata operations not yet implemented
#[test]
#[ignore]
fn test_9_6_vary_meta() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def v (with-meta [1 2 3] {:a 1}))").unwrap();
    let _res = ctx.eval("(def v2 (vary-meta v assoc :b 2))").unwrap();
    ctx.assert_map(
        "(meta v2)",
        &spec_ref("9.6", "vary-meta", "transforms existing metadata"),
    );
}

// ============================================================================
// Section 9.12: I/O (print)
// Reference: docs/lonala.md#912-io
// ============================================================================

/// [IGNORED] Spec 9.12: print returns nil
/// Tracking: print function behavior
#[test]
#[ignore]
fn test_9_12_print_returns_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(print \"hello\")",
        &spec_ref("9.12", "print", "returns nil"),
    );
}

/// [IGNORED] Spec 9.12: print with multiple arguments
/// Tracking: print function behavior
#[test]
#[ignore]
fn test_9_12_print_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(print 1 2 3)",
        &spec_ref("9.12", "print", "variadic arguments"),
    );
}
