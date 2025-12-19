// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Collection Primitives and Operations.
//!
//! Sections 9.3 and 9.16 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.3.1: List Operations
// Reference: docs/lonala.md#931-list-operations
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

// ============================================================================
// Section 9.3.2: Vector Operations
// Reference: docs/lonala.md#932-vector-operations
// ============================================================================

/// [IGNORED] Spec 9.3.2: nth - Get element at index
/// Tracking: nth not yet implemented
#[test]
#[ignore]
fn test_9_3_2_nth() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(nth (vector 1 2 3) 0)",
        1,
        &spec_ref("9.3.2", "nth", "first element at index 0"),
    );
    ctx.assert_int(
        "(nth (vector 1 2 3) 1)",
        2,
        &spec_ref("9.3.2", "nth", "second element at index 1"),
    );
    ctx.assert_int(
        "(nth (vector 1 2 3) 2)",
        3,
        &spec_ref("9.3.2", "nth", "third element at index 2"),
    );
}

/// [IGNORED] Spec 9.3.2: nth - Out of bounds returns nil or error
/// Tracking: nth not yet implemented
#[test]
#[ignore]
fn test_9_3_2_nth_out_of_bounds() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(nth (vector 1 2 3) 5)",
        &spec_ref("9.3.2", "nth", "out of bounds is error"),
    );
}

/// [IGNORED] Spec 9.3.2: nth with default value
/// Tracking: nth not yet implemented
#[test]
#[ignore]
fn test_9_3_2_nth_default() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(nth (vector 1 2 3) 5 42)",
        42,
        &spec_ref("9.3.2", "nth", "out of bounds returns default"),
    );
}

/// [IGNORED] Spec 9.3.2: conj - Add element to collection
/// Tracking: conj not yet implemented
#[test]
#[ignore]
fn test_9_3_2_conj_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_eq(
        "(conj (vector 1 2) 3)",
        "(vector 1 2 3)",
        &spec_ref("9.3.2", "conj", "add to vector appends at end"),
    );
}

/// [IGNORED] Spec 9.3.2: conj on list prepends
/// Tracking: conj not yet implemented
#[test]
#[ignore]
fn test_9_3_2_conj_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(conj '(2 3) 1)",
        "'(1 2 3)",
        &spec_ref("9.3.2", "conj", "add to list prepends"),
    );
}

/// [IGNORED] Spec 9.3.2: count - Get collection size
/// Tracking: count not yet implemented
#[test]
#[ignore]
fn test_9_3_2_count() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(count (vector 1 2 3))",
        3,
        &spec_ref("9.3.2", "count", "vector has 3 elements"),
    );
    ctx.assert_int(
        "(count '(1 2 3 4))",
        4,
        &spec_ref("9.3.2", "count", "list has 4 elements"),
    );
    ctx.assert_int(
        "(count (hash-map 'a 1 'b 2))",
        2,
        &spec_ref("9.3.2", "count", "map has 2 entries"),
    );
    ctx.assert_int(
        "(count \"hello\")",
        5,
        &spec_ref("9.3.2", "count", "string has 5 characters"),
    );
}

/// [IGNORED] Spec 9.3.2: count of nil is 0
/// Tracking: count not yet implemented
#[test]
#[ignore]
fn test_9_3_2_count_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(count nil)",
        0,
        &spec_ref("9.3.2", "count", "nil has count 0"),
    );
}

// ============================================================================
// Section 9.3.3: Map Operations
// Reference: docs/lonala.md#933-map-operations
// ============================================================================

/// [IGNORED] Spec 9.3.3: get - Get value for key
/// Tracking: get not yet implemented
#[test]
#[ignore]
fn test_9_3_3_get() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(get (hash-map 'a 1 'b 2) 'a)",
        1,
        &spec_ref("9.3.3", "get", "get value for key 'a"),
    );
    ctx.assert_int(
        "(get (hash-map 'a 1 'b 2) 'b)",
        2,
        &spec_ref("9.3.3", "get", "get value for key 'b"),
    );
}

/// [IGNORED] Spec 9.3.3: get - Missing key returns nil
/// Tracking: get not yet implemented
#[test]
#[ignore]
fn test_9_3_3_get_missing() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(get (hash-map 'a 1) 'b)",
        &spec_ref("9.3.3", "get", "missing key returns nil"),
    );
}

/// [IGNORED] Spec 9.3.3: get - With default value
/// Tracking: get not yet implemented
#[test]
#[ignore]
fn test_9_3_3_get_default() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(get (hash-map 'a 1) 'b 42)",
        42,
        &spec_ref("9.3.3", "get", "missing key returns default"),
    );
}

/// [IGNORED] Spec 9.3.3: assoc - Associate key with value
/// Tracking: assoc not yet implemented
#[test]
#[ignore]
fn test_9_3_3_assoc() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(get (assoc (hash-map 'a 1) 'b 2) 'b)",
        2,
        &spec_ref("9.3.3", "assoc", "add new key-value pair"),
    );
}

/// [IGNORED] Spec 9.3.3: assoc - Update existing key
/// Tracking: assoc not yet implemented
#[test]
#[ignore]
fn test_9_3_3_assoc_update() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(get (assoc (hash-map 'a 1) 'a 99) 'a)",
        99,
        &spec_ref("9.3.3", "assoc", "update existing key"),
    );
}

/// [IGNORED] Spec 9.3.3: dissoc - Remove key
/// Tracking: dissoc not yet implemented
#[test]
#[ignore]
fn test_9_3_3_dissoc() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(get (dissoc (hash-map 'a 1 'b 2) 'a) 'a)",
        &spec_ref("9.3.3", "dissoc", "removed key returns nil"),
    );
    ctx.assert_int(
        "(get (dissoc (hash-map 'a 1 'b 2) 'a) 'b)",
        2,
        &spec_ref("9.3.3", "dissoc", "other keys remain"),
    );
}

/// [IGNORED] Spec 9.3.3: keys - Get sequence of keys
/// Tracking: keys not yet implemented
#[test]
#[ignore]
fn test_9_3_3_keys() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(count (keys (hash-map 'a 1 'b 2)))",
        2,
        &spec_ref("9.3.3", "keys", "keys sequence has 2 elements"),
    );
}

/// [IGNORED] Spec 9.3.3: vals - Get sequence of values
/// Tracking: vals not yet implemented
#[test]
#[ignore]
fn test_9_3_3_vals() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(count (vals (hash-map 'a 1 'b 2)))",
        2,
        &spec_ref("9.3.3", "vals", "vals sequence has 2 elements"),
    );
}

// ============================================================================
// Section 9.3.4: Set Operations (Planned)
// Reference: docs/lonala.md#934-set-operations
//
// Note: Set constructors (hash-set, set) and higher-level operations
// (union, intersection, difference, subset?, superset?) are implemented
// in Lonala, not as native primitives. Only disj and contains? are native.
// ============================================================================

/// [IGNORED] Spec 9.3.4: disj - Remove element from set
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_9_3_4_disj() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(contains? (disj #{1 2 3} 2) 2)",
        false,
        &spec_ref("9.3.4", "disj", "element is removed"),
    );
    ctx.assert_bool(
        "(contains? (disj #{1 2 3} 2) 1)",
        true,
        &spec_ref("9.3.4", "disj", "other elements remain"),
    );
}

/// [IGNORED] Spec 9.3.4: contains? - Check membership
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_9_3_4_contains() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(contains? #{1 2 3} 2)",
        true,
        &spec_ref("9.3.4", "contains?", "element is member"),
    );
    ctx.assert_bool(
        "(contains? #{1 2 3} 5)",
        false,
        &spec_ref("9.3.4", "contains?", "element is not member"),
    );
}
