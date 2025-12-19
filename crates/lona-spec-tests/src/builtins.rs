// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 9 - Built-in Functions
//!
//! Reference: docs/lonala.md#9-built-in-functions
//!
//! Tests built-in functions (primitives/natives) implemented in Rust.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.1.1: print
// Reference: docs/lonala.md#911-print
// ============================================================================

/// [IGNORED] Spec 9.1.1: "Returns: nil"
/// Tracking: print function not yet implemented
#[test]
#[ignore]
fn test_9_1_1_print_returns_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(print \"hello\")",
        &spec_ref("9.1.1", "print", "returns nil"),
    );
}

/// [IGNORED] Spec 9.1.1: "Parameters: Zero or more values to print"
/// Tracking: print function not yet implemented
#[test]
#[ignore]
fn test_9_1_1_print_variadic() {
    let mut ctx = SpecTestContext::new();
    // Multiple arguments should work
    ctx.assert_nil(
        "(print 1 2 3)",
        &spec_ref("9.1.1", "print", "variadic arguments"),
    );
    // Zero arguments should work
    ctx.assert_nil("(print)", &spec_ref("9.1.1", "print", "zero arguments"));
}

// ============================================================================
// Section 9.2: Collection Functions (Implemented)
// Reference: docs/lonala.md#92-planned-built-in-functions
// ============================================================================

/// cons - Prepend element to list
#[test]
fn test_9_2_cons() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list(
        "(cons 1 '(2 3))",
        &spec_ref("9.2", "cons", "prepend to list"),
    );
    ctx.assert_list(
        "(cons 1 '())",
        &spec_ref("9.2", "cons", "prepend to empty list"),
    );
}

/// first - Get first element
#[test]
fn test_9_2_first() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(first '(1 2 3))",
        1,
        &spec_ref("9.2", "first", "first of list"),
    );
}

/// first - Empty list returns nil
#[test]
fn test_9_2_first_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(first '())",
        &spec_ref("9.2", "first", "first of empty list is nil"),
    );
}

/// rest - Get all but first element
#[test]
fn test_9_2_rest() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list("(rest '(1 2 3))", &spec_ref("9.2", "rest", "rest of list"));
}

/// rest - Returns empty list for single-element list
#[test]
fn test_9_2_rest_single() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list(
        "(rest '(1))",
        &spec_ref("9.2", "rest", "rest of single-element list"),
    );
}

/// rest - Returns empty list for empty list
#[test]
fn test_9_2_rest_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list(
        "(rest '())",
        &spec_ref("9.2", "rest", "rest of empty list is empty list"),
    );
}

/// list - Create a list
#[test]
fn test_9_2_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list("(list 1 2 3)", &spec_ref("9.2", "list", "create list"));
    ctx.assert_list("(list)", &spec_ref("9.2", "list", "create empty list"));
}

/// concat - Concatenate lists
#[test]
fn test_9_2_concat() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list(
        "(concat '(1 2) '(3 4))",
        &spec_ref("9.2", "concat", "concatenate two lists"),
    );
}

/// concat - With empty lists
#[test]
fn test_9_2_concat_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list(
        "(concat '() '(1 2))",
        &spec_ref("9.2", "concat", "concat empty with non-empty"),
    );
    ctx.assert_list(
        "(concat '(1 2) '())",
        &spec_ref("9.2", "concat", "concat non-empty with empty"),
    );
}

/// vector - Create a vector
#[test]
fn test_9_2_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector(
        "(vector 1 2 3)",
        &spec_ref("9.2", "vector", "create vector"),
    );
    ctx.assert_vector(
        "(vector)",
        &spec_ref("9.2", "vector", "create empty vector"),
    );
}

/// vec - Convert to vector
#[test]
fn test_9_2_vec() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector(
        "(vec '(1 2 3))",
        &spec_ref("9.2", "vec", "convert list to vector"),
    );
}

/// hash-map - Create a map
#[test]
fn test_9_2_hash_map() {
    let mut ctx = SpecTestContext::new();
    // hash-map takes alternating key-value pairs
    let result = ctx.eval("(hash-map 'a 1 'b 2)").unwrap();
    match result {
        lona_core::value::Value::Map(_map) => {
            // Success - got a map
        }
        _ => panic!("[Spec 9.2 hash-map] expected map"),
    }
}

/// hash-map - Empty map
#[test]
fn test_9_2_hash_map_empty() {
    let mut ctx = SpecTestContext::new();
    let result = ctx.eval("(hash-map)").unwrap();
    match result {
        lona_core::value::Value::Map(_map) => {
            // Success - got a map
        }
        _ => panic!("[Spec 9.2 hash-map] expected empty map"),
    }
}

// ============================================================================
// Section 9.2: Planned Built-in Functions
// Reference: docs/lonala.md#92-planned-built-in-functions
// ============================================================================

/// [IGNORED] Type predicates - planned
#[test]
#[ignore]
fn test_9_2_type_predicates() {
    let mut _ctx = SpecTestContext::new();
    // nil?, boolean?, number?, integer?, float?, ratio?
    // string?, symbol?, keyword?, list?, vector?, map?, fn?
}

/// [IGNORED] String functions - planned
#[test]
#[ignore]
fn test_9_2_string_functions() {
    let mut _ctx = SpecTestContext::new();
    // str, subs, string/join, string/split
}

/// [IGNORED] Numeric functions - planned
#[test]
#[ignore]
fn test_9_2_numeric_functions() {
    let mut _ctx = SpecTestContext::new();
    // inc, dec, abs, min, max
}
