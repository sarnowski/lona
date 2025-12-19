// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for collection types: List, Vector, and Map.
//!
//! Sections 3.9-3.11 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.9: List
// Reference: docs/lonala.md#39-list
// ============================================================================

/// Spec 3.9: Quoted list is data, not code
#[test]
fn test_3_9_quoted_list() {
    let mut ctx = SpecTestContext::new();
    // Verify quoted list produces exactly (1 2 3)
    ctx.assert_list_len(
        "'(1 2 3)",
        3,
        &spec_ref("3.9", "List", "quoted list has 3 elements"),
    );
    ctx.assert_int(
        "(first '(1 2 3))",
        1,
        &spec_ref("3.9", "List", "first element is 1"),
    );
    ctx.assert_int(
        "(first (rest '(1 2 3)))",
        2,
        &spec_ref("3.9", "List", "second element is 2"),
    );
    ctx.assert_int(
        "(first (rest (rest '(1 2 3))))",
        3,
        &spec_ref("3.9", "List", "third element is 3"),
    );
}

/// Spec 3.9: Empty list
#[test]
fn test_3_9_empty_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_len(
        "'()",
        0,
        &spec_ref("3.9", "List", "empty quoted list has 0 elements"),
    );
}

/// Spec 3.9: List equality
#[test]
fn test_3_9_list_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= '(1 2 3) '(1 2 3))",
        true,
        &spec_ref("3.9", "List", "lists with same elements are equal"),
    );
    ctx.assert_bool(
        "(= '(1 2) '(1 2 3))",
        false,
        &spec_ref("3.9", "List", "lists with different lengths are not equal"),
    );
    ctx.assert_bool(
        "(= '(1 2 3) '(1 2 4))",
        false,
        &spec_ref("3.9", "List", "lists with different elements are not equal"),
    );
}

/// Spec 3.9: Nested lists
#[test]
fn test_3_9_nested_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_len(
        "'((1 2) (3 4))",
        2,
        &spec_ref("3.9", "List", "nested list has 2 outer elements"),
    );
}

// ============================================================================
// Section 3.10: Vector
// Reference: docs/lonala.md#310-vector
// ============================================================================

/// Spec 3.10: Vector creation via function
#[test]
fn test_3_10_vector_creation() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_len(
        "(vector 1 2 3)",
        3,
        &spec_ref(
            "3.10",
            "Vector",
            "vector function creates vector with 3 elements",
        ),
    );
}

/// Spec 3.10: Empty vector
#[test]
fn test_3_10_empty_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_len(
        "(vector)",
        0,
        &spec_ref("3.10", "Vector", "empty vector has 0 elements"),
    );
}

/// Spec 3.10: Vector equality
#[test]
fn test_3_10_vector_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (vector 1 2 3) (vector 1 2 3))",
        true,
        &spec_ref("3.10", "Vector", "vectors with same elements are equal"),
    );
    ctx.assert_bool(
        "(= (vector 1 2) (vector 1 2 3))",
        false,
        &spec_ref(
            "3.10",
            "Vector",
            "vectors with different lengths are not equal",
        ),
    );
}

/// [IGNORED] Spec 3.10: Vector literals
/// Tracking: Vector literal compilation planned
#[test]
#[ignore]
fn test_3_10_vector_literal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_eq(
        "[1 2 3]",
        "[1 2 3]",
        &spec_ref("3.10", "Vector", "vector literal yields [1 2 3]"),
    );
    ctx.assert_vector_eq(
        "[]",
        "[]",
        &spec_ref("3.10", "Vector", "empty vector literal yields []"),
    );
}

/// Spec 3.10: vec converts collection to vector
#[test]
fn test_3_10_vec_from_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_eq(
        "(vec '(1 2 3))",
        "(vector 1 2 3)",
        &spec_ref("3.10", "Vector", "vec converts list to vector"),
    );
}

// ============================================================================
// Section 3.11: Map
// Reference: docs/lonala.md#311-map
// ============================================================================

/// Spec 3.11: Map creation via hash-map
#[test]
fn test_3_11_map_creation() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_len(
        "(hash-map 'a 1 'b 2)",
        2,
        &spec_ref("3.11", "Map", "hash-map creates map with 2 entries"),
    );
}

/// Spec 3.11: Empty map
#[test]
fn test_3_11_empty_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_len(
        "(hash-map)",
        0,
        &spec_ref("3.11", "Map", "empty hash-map has 0 entries"),
    );
}

/// Spec 3.11: Map equality
#[test]
fn test_3_11_map_equality() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (hash-map 'a 1 'b 2) (hash-map 'a 1 'b 2))",
        true,
        &spec_ref("3.11", "Map", "maps with same entries are equal"),
    );
    ctx.assert_bool(
        "(= (hash-map 'a 1 'b 2) (hash-map 'a 1))",
        false,
        &spec_ref("3.11", "Map", "maps with different entries are not equal"),
    );
}

/// [IGNORED] Spec 3.11: Map literals
/// Tracking: Map literal compilation planned
#[test]
#[ignore]
fn test_3_11_map_literal() {
    let mut _ctx = SpecTestContext::new();
    // Map literal tests when implemented
    // {:a 1 :b 2} syntax
}

/// Spec 3.11: Any key type - maps can use any value as key
#[test]
fn test_3_11_map_any_key_type() {
    let mut ctx = SpecTestContext::new();
    // Integer keys
    ctx.assert_map_len(
        "(hash-map 1 \"one\" 2 \"two\")",
        2,
        &spec_ref("3.11", "Map", "map with integer keys"),
    );
    // String keys
    ctx.assert_map_len(
        "(hash-map \"name\" \"Alice\")",
        1,
        &spec_ref("3.11", "Map", "map with string keys"),
    );
}
