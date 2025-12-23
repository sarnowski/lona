// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 8.7 - Multi-Arity Functions
//!
//! Reference: docs/lonala.md#87-multi-arity-functions

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 8.7: Multi-Arity Functions
// Reference: docs/lonala.md#87-multi-arity-functions
// ============================================================================

/// Spec 8.7: Multi-arity function with two arities
#[test]
fn test_8_7_multi_arity_basic() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def f (fn ([x] 1) ([x y] 2)))").unwrap();
    ctx.assert_int(
        "(f 10)",
        1,
        &spec_ref("8.7", "Multi-arity", "1-arity returns 1"),
    );
    ctx.assert_int(
        "(f 10 20)",
        2,
        &spec_ref("8.7", "Multi-arity", "2-arity returns 2"),
    );
}

/// Spec 8.7: Multi-arity function with arity error
#[test]
fn test_8_7_multi_arity_error() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def f (fn ([x] 1) ([x y] 2)))").unwrap();
    ctx.assert_error("(f)", &spec_ref("8.7", "Multi-arity", "0 args is error"));
    ctx.assert_error(
        "(f 1 2 3)",
        &spec_ref("8.7", "Multi-arity", "3 args is error"),
    );
}

/// Spec 8.7: Multi-arity with rest parameter
#[test]
fn test_8_7_multi_arity_with_rest() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def f (fn ([x] x) ([x y & z] z)))").unwrap();
    ctx.assert_int(
        "(f 1)",
        1,
        &spec_ref("8.7", "Multi-arity", "1-arity exact match"),
    );
    ctx.assert_list_len(
        "(f 1 2)",
        0,
        &spec_ref("8.7", "Multi-arity", "2-arity, empty rest"),
    );
    ctx.assert_list_len(
        "(f 1 2 3 4)",
        2,
        &spec_ref("8.7", "Multi-arity", "4-arity, rest has 2"),
    );
}

/// Spec 8.7: Exact arity match beats variadic
#[test]
fn test_8_7_exact_beats_variadic() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def f (fn ([x] 1) ([x & r] 2)))").unwrap();
    ctx.assert_int(
        "(f 10)",
        1,
        &spec_ref("8.7", "Multi-arity", "exact match wins over variadic"),
    );
    ctx.assert_int(
        "(f 10 20)",
        2,
        &spec_ref("8.7", "Multi-arity", "variadic for 2+ args"),
    );
}

/// Spec 8.7: Single body in list form (equivalent to vector form)
#[test]
fn test_8_7_single_body_list_form() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn ([x] x)) 42)",
        42,
        &spec_ref("8.7", "Multi-arity", "single arity in list form"),
    );
}

/// Spec 8.7: Named multi-arity function with recursion
#[test]
fn test_8_7_named_recursion() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def fact (fn fact ([n] (fact n 1)) ([n acc] (if (<= n 1) acc (fact (- n 1) (* n acc))))))")
        .unwrap();
    ctx.assert_int(
        "(fact 5)",
        120,
        &spec_ref("8.7", "Multi-arity", "recursive multi-arity factorial"),
    );
}

/// Spec 8.7: Compile error for duplicate arity
#[test]
fn test_8_7_duplicate_arity_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(fn ([x] 1) ([y] 2))",
        &spec_ref("8.7", "Multi-arity", "duplicate arity is error"),
    );
}

/// Spec 8.7: Compile error for fixed arity > variadic arity
#[test]
fn test_8_7_invalid_variadic_error() {
    let mut ctx = SpecTestContext::new();
    // Fixed arity 1 > variadic's fixed 0, so this is an error
    ctx.assert_error(
        "(fn ([x] 1) ([& z] 2))",
        &spec_ref("8.7", "Multi-arity", "fixed > variadic is error"),
    );
    // Fixed arity 2 > variadic's fixed 1, so this is also an error
    ctx.assert_error(
        "(fn ([x y] 1) ([x & r] 2))",
        &spec_ref("8.7", "Multi-arity", "fixed 2 > variadic fixed 1 is error"),
    );
}

/// Spec 8.7: Compile error for multiple variadic arities
#[test]
fn test_8_7_multiple_variadic_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(fn ([& x] 1) ([& y] 2))",
        &spec_ref("8.7", "Multi-arity", "multiple variadic is error"),
    );
}

/// Spec 8.7: Zero-arity function
#[test]
fn test_8_7_zero_arity() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn ([] 42)))",
        42,
        &spec_ref("8.7", "Multi-arity", "zero-arity function"),
    );
}

/// Spec 8.7: Zero-arity with variadic (both valid, exact match beats variadic)
#[test]
fn test_8_7_zero_arity_with_variadic() {
    let mut ctx = SpecTestContext::new();
    // Fixed arity 0 equals variadic fixed 0, which is allowed.
    // The exact match (0-arity) beats variadic for 0 args.
    let _res = ctx.eval("(def f (fn ([] 0) ([& r] 1)))").unwrap();
    ctx.assert_int(
        "(f)",
        0,
        &spec_ref("8.7", "Multi-arity", "0-arity exact match"),
    );
    ctx.assert_int(
        "(f 1)",
        1,
        &spec_ref("8.7", "Multi-arity", "variadic for 1+ args"),
    );
}

/// Spec 8.7: Many arities
#[test]
fn test_8_7_many_arities() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def multi (fn ([] 0) ([a] 1) ([a b] 2) ([a b c] 3) ([a b c d] 4)))")
        .unwrap();
    ctx.assert_int("(multi)", 0, &spec_ref("8.7", "Multi-arity", "0-arity"));
    ctx.assert_int("(multi 1)", 1, &spec_ref("8.7", "Multi-arity", "1-arity"));
    ctx.assert_int("(multi 1 2)", 2, &spec_ref("8.7", "Multi-arity", "2-arity"));
    ctx.assert_int(
        "(multi 1 2 3)",
        3,
        &spec_ref("8.7", "Multi-arity", "3-arity"),
    );
    ctx.assert_int(
        "(multi 1 2 3 4)",
        4,
        &spec_ref("8.7", "Multi-arity", "4-arity"),
    );
}

/// Spec 8.7: Empty function (fn) should error
#[test]
fn test_8_7_empty_fn_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error("(fn)", &spec_ref("8.7", "Multi-arity", "empty fn is error"));
}
