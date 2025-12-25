// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7.6-7.7 - First-Class Operators
//!
//! Reference: docs/lonala/operators.md
//!
//! Tests that arithmetic and comparison operators can be used as first-class
//! values - bound to variables, passed to functions, and called indirectly.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 7.6: First-Class Arithmetic Operators
// ============================================================================

/// `+` can be bound to a variable and called
#[test]
fn test_7_6_addition_as_value() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do (def plus +) (plus 1 2))",
        3,
        &spec_ref(
            "7.6",
            "First-class",
            "+ can be bound to variable and called",
        ),
    );
}

/// `-` can be bound to a variable and called
#[test]
fn test_7_6_subtraction_as_value() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do (def minus -) (minus 10 3))",
        7,
        &spec_ref(
            "7.6",
            "First-class",
            "- can be bound to variable and called",
        ),
    );
}

/// `+` can be passed to a user-defined higher-order function
#[test]
fn test_7_6_addition_passed_to_function() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def apply-op (fn [op a b] (op a b)))").unwrap();
    ctx.assert_int(
        "(apply-op + 3 4)",
        7,
        &spec_ref(
            "7.6",
            "First-class",
            "+ can be passed as argument to function",
        ),
    );
}

/// `-` can be passed to a user-defined higher-order function
#[test]
fn test_7_6_subtraction_passed_to_function() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def apply-op (fn [op a b] (op a b)))").unwrap();
    ctx.assert_int(
        "(apply-op - 10 3)",
        7,
        &spec_ref(
            "7.6",
            "First-class",
            "- can be passed as argument to function",
        ),
    );
}

/// Bound arithmetic operators work with variadic calls
#[test]
fn test_7_6_bound_operators_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do (def plus +) (plus 1 2 3 4))",
        10,
        &spec_ref("7.6", "First-class", "bound + works with variadic args"),
    );
    ctx.assert_int(
        "(do (def minus -) (minus 20 5 3 2))",
        10,
        &spec_ref("7.6", "First-class", "bound - works with variadic args"),
    );
}

/// Bound arithmetic operators work with edge arities
#[test]
fn test_7_6_bound_operators_edge_arities() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do (def plus +) (plus))",
        0,
        &spec_ref("7.6", "First-class", "bound + with zero args returns 0"),
    );
    ctx.assert_int(
        "(do (def plus +) (plus 42))",
        42,
        &spec_ref("7.6", "First-class", "bound + with one arg returns arg"),
    );
    ctx.assert_int(
        "(do (def minus -) (minus 5))",
        -5,
        &spec_ref("7.6", "First-class", "bound - with one arg negates"),
    );
}

/// `*` can be bound to a variable and called
#[test]
fn test_7_6_multiplication_as_value() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do (def times *) (times 2 3))",
        6,
        &spec_ref(
            "7.6",
            "First-class",
            "* can be bound to variable and called",
        ),
    );
}

/// `*` can be passed to a user-defined higher-order function
#[test]
fn test_7_6_multiplication_passed_to_function() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def apply-op (fn [op a b] (op a b)))").unwrap();
    ctx.assert_int(
        "(apply-op * 3 4)",
        12,
        &spec_ref(
            "7.6",
            "First-class",
            "* can be passed as argument to function",
        ),
    );
}

/// Bound * works with edge arities
#[test]
fn test_7_6_multiplication_edge_arities() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do (def times *) (times))",
        1,
        &spec_ref("7.6", "First-class", "bound * with zero args returns 1"),
    );
    ctx.assert_int(
        "(do (def times *) (times 42))",
        42,
        &spec_ref("7.6", "First-class", "bound * with one arg returns arg"),
    );
    ctx.assert_int(
        "(do (def times *) (times 2 3 4))",
        24,
        &spec_ref("7.6", "First-class", "bound * with variadic args"),
    );
}

/// `/` can be bound to a variable and called
#[test]
fn test_7_6_division_as_value() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do (def divide /) (divide 10 2))",
        5,
        &spec_ref(
            "7.6",
            "First-class",
            "/ can be bound to variable and called",
        ),
    );
}

/// Bound / returns reciprocal with one argument
#[test]
fn test_7_6_division_reciprocal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(do (def divide /) (divide 2))",
        1,
        2,
        &spec_ref(
            "7.6",
            "First-class",
            "bound / with one arg returns reciprocal",
        ),
    );
    ctx.assert_int(
        "(do (def divide /) (divide 1))",
        1,
        &spec_ref("7.6", "First-class", "bound / of 1 returns 1"),
    );
    ctx.assert_int(
        "(do (def divide /) (divide -1))",
        -1,
        &spec_ref("7.6", "First-class", "bound / of -1 returns -1"),
    );
}

/// `/` can be passed to a user-defined higher-order function
#[test]
fn test_7_6_division_passed_to_function() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def apply-op (fn [op a b] (op a b)))").unwrap();
    ctx.assert_int(
        "(apply-op / 12 3)",
        4,
        &spec_ref(
            "7.6",
            "First-class",
            "/ can be passed as argument to function",
        ),
    );
}

/// Bound / with variadic args
#[test]
fn test_7_6_division_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do (def divide /) (divide 24 2 3))",
        4,
        &spec_ref("7.6", "First-class", "bound / with variadic args"),
    );
}

/// `mod` can be bound to a variable and called
#[test]
fn test_7_6_modulo_as_value() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do (def modulo mod) (modulo 10 3))",
        1,
        &spec_ref(
            "7.6",
            "First-class",
            "mod can be bound to variable and called",
        ),
    );
}

/// `mod` can be passed to a user-defined higher-order function
#[test]
fn test_7_6_modulo_passed_to_function() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def apply-op (fn [op a b] (op a b)))").unwrap();
    ctx.assert_int(
        "(apply-op mod 10 3)",
        1,
        &spec_ref(
            "7.6",
            "First-class",
            "mod can be passed as argument to function",
        ),
    );
}

// ============================================================================
// Section 7.6: First-Class Comparison Operators
// ============================================================================

/// `=` can be bound to a variable and called
#[test]
fn test_7_6_equality_as_value() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(do (def eq =) (eq 1 1))",
        true,
        &spec_ref("7.6", "First-class", "= can be bound and called"),
    );
}

/// `=` can be passed to a user-defined higher-order function
#[test]
fn test_7_6_equality_passed_to_function() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def apply-pred (fn [p a b] (p a b)))").unwrap();
    ctx.assert_bool(
        "(apply-pred = 1 1)",
        true,
        &spec_ref("7.6", "First-class", "= passed as argument"),
    );
}

/// Bound `=` works with multi-argument calls
#[test]
fn test_7_6_equality_bound_multi_arg() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(do (def eq =) (eq 1 1 1))",
        true,
        &spec_ref("7.6", "First-class", "bound = with three args"),
    );
    ctx.assert_bool(
        "(do (def eq =) (eq 1 1 2))",
        false,
        &spec_ref("7.6", "First-class", "bound = fails when last differs"),
    );
}

/// Other comparison operators can be bound and used as first-class values
#[test]
fn test_7_6_comparison_operators_as_values() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(do (def lt <) (lt 1 2))",
        true,
        &spec_ref("7.6", "First-class", "< can be bound and called"),
    );
    ctx.assert_bool(
        "(do (def gt >) (gt 2 1))",
        true,
        &spec_ref("7.6", "First-class", "> can be bound and called"),
    );
    ctx.assert_bool(
        "(do (def le <=) (le 1 1))",
        true,
        &spec_ref("7.6", "First-class", "<= can be bound and called"),
    );
    ctx.assert_bool(
        "(do (def ge >=) (ge 2 2))",
        true,
        &spec_ref("7.6", "First-class", ">= can be bound and called"),
    );
}

/// Multi-argument comparison operators work as first-class functions
#[test]
fn test_7_6_comparison_multi_arg_first_class() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(do (def lt <) (lt 1 2 3))",
        true,
        &spec_ref("7.6", "First-class", "bound < with three args (ascending)"),
    );
    ctx.assert_bool(
        "(do (def lt <) (lt 1 3 2))",
        false,
        &spec_ref(
            "7.6",
            "First-class",
            "bound < with three args (not ascending)",
        ),
    );
    ctx.assert_bool(
        "(do (def gt >) (gt 3 2 1))",
        true,
        &spec_ref("7.6", "First-class", "bound > with three args (descending)"),
    );
}

/// Spec 7.6: String comparison operators as first-class values
#[test]
fn test_7_6_string_comparison_first_class() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(do (def lt <) (lt \"a\" \"b\"))",
        true,
        &spec_ref("7.6", "First-class", "bound < with strings"),
    );
    ctx.assert_bool(
        "(do (def gt >) (gt \"z\" \"a\"))",
        true,
        &spec_ref("7.6", "First-class", "bound > with strings"),
    );
}

// ============================================================================
// Section 7.7: Error Argument Attribution
// Reference: Type errors in arithmetic operators should correctly identify
// which argument caused the error.
// ============================================================================

/// Type error in first argument correctly identifies argument 1
#[test]
fn test_7_7_error_attribution_first_arg() {
    let mut ctx = SpecTestContext::new();
    // When first argument is wrong, error should have arg_index: 0
    ctx.assert_error_contains(
        "(do (def plus +) (plus true 1))",
        "arg_index: 0",
        &spec_ref("7.7", "Error", "+ first arg error attribution"),
    );
    ctx.assert_error_contains(
        "(do (def times *) (times true 1))",
        "arg_index: 0",
        &spec_ref("7.7", "Error", "* first arg error attribution"),
    );
}

/// Type error in second argument correctly identifies argument 2
#[test]
fn test_7_7_error_attribution_second_arg() {
    let mut ctx = SpecTestContext::new();
    // When second argument is wrong, error should have arg_index: 1
    ctx.assert_error_contains(
        "(do (def plus +) (plus 1 true))",
        "arg_index: 1",
        &spec_ref("7.7", "Error", "+ second arg error attribution"),
    );
    ctx.assert_error_contains(
        "(do (def times *) (times 1 true))",
        "arg_index: 1",
        &spec_ref("7.7", "Error", "* second arg error attribution"),
    );
}

/// Type error in third argument correctly identifies argument 3
#[test]
fn test_7_7_error_attribution_third_arg() {
    let mut ctx = SpecTestContext::new();
    // When third argument is wrong, error should have arg_index: 2
    ctx.assert_error_contains(
        "(do (def plus +) (plus 1 2 true))",
        "arg_index: 2",
        &spec_ref("7.7", "Error", "+ third arg error attribution"),
    );
    ctx.assert_error_contains(
        "(do (def times *) (times 1 2 true))",
        "arg_index: 2",
        &spec_ref("7.7", "Error", "* third arg error attribution"),
    );
}
