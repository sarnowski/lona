// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 8 - Functions
//!
//! Reference: docs/lonala.md#8-functions
//!
//! Tests function definition, calling, arity, higher-order functions,
//! and recursion.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 8.1: Defining Functions
// Reference: docs/lonala.md#81-defining-functions
// ============================================================================

/// Spec 8.1: Anonymous function with fn
#[test]
fn test_8_1_anonymous_function() {
    let mut ctx = SpecTestContext::new();
    // Verify it's a function and that it works when called
    ctx.assert_function(
        "(fn [x] (* x x))",
        &spec_ref("8.1", "Defining", "anonymous function is callable"),
    );
    // Also test that the function works correctly
    ctx.assert_int(
        "((fn [x] (* x x)) 5)",
        25,
        &spec_ref("8.1", "Defining", "anonymous function computes 5*5=25"),
    );
}

/// Spec 8.1: Named function (useful for recursion)
#[test]
fn test_8_1_named_function() {
    let mut ctx = SpecTestContext::new();
    // Verify it's a function and that it works when called
    ctx.assert_function(
        "(fn square [x] (* x x))",
        &spec_ref("8.1", "Defining", "named function is callable"),
    );
    // Also test that the function works correctly
    ctx.assert_int(
        "((fn square [x] (* x x)) 4)",
        16,
        &spec_ref("8.1", "Defining", "named function computes 4*4=16"),
    );
}

/// Spec 8.1: "To give a function a global name, combine def and fn"
#[test]
fn test_8_1_def_and_fn() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def square (fn [x] (* x x)))").unwrap();
    ctx.assert_int(
        "(square 5)",
        25,
        &spec_ref("8.1", "Defining", "def + fn creates callable"),
    );
}

// ============================================================================
// Section 8.2: Calling Functions
// Reference: docs/lonala.md#82-calling-functions
// ============================================================================

/// Spec 8.2: "Function calls use list syntax with the function in the first position"
#[test]
fn test_8_2_function_call_syntax() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+ 1 2)",
        3,
        &spec_ref("8.2", "Calling", "list syntax for function call"),
    );
}

/// [IGNORED] Spec 8.2: "Arguments are evaluated left-to-right"
/// Tracking: N-ary arithmetic not yet implemented
#[test]
#[ignore]
fn test_8_2_left_to_right_eval() {
    let mut ctx = SpecTestContext::new();
    // Use side effects to verify left-to-right evaluation
    let _res = ctx.eval("(def order (list))").unwrap();
    let _res = ctx
        .eval("(def track (fn [x] (def order (cons x order)) x))")
        .unwrap();
    // Call with tracked arguments
    let _res = ctx.eval("(+ (track 1) (track 2) (track 3))").unwrap();
    // order should be (3 2 1) because cons prepends and we called 1, then 2, then 3
    // First element should be 3 (last added)
    ctx.assert_int(
        "(first order)",
        3,
        &spec_ref("8.2", "Calling", "arguments evaluated left-to-right"),
    );
}

// ============================================================================
// Section 8.3: Function Arity
// Reference: docs/lonala.md#83-function-arity
// ============================================================================

/// Spec 8.3: "Each function has a fixed arity"
#[test]
fn test_8_3_correct_arity() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def greet (fn [name] name))").unwrap();
    ctx.assert_string(
        "(greet \"Alice\")",
        "Alice",
        &spec_ref("8.3", "Arity", "correct arity succeeds"),
    );
}

/// Spec 8.3: "Calling with wrong number of arguments is a runtime error"
#[test]
fn test_8_3_too_few_args() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def greet (fn [name] name))").unwrap();
    ctx.assert_error(
        "(greet)",
        &spec_ref("8.3", "Arity", "too few args is error"),
    );
}

/// Spec 8.3: Too many arguments
#[test]
fn test_8_3_too_many_args() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def greet (fn [name] name))").unwrap();
    ctx.assert_error(
        "(greet \"A\" \"B\")",
        &spec_ref("8.3", "Arity", "too many args is error"),
    );
}

// ============================================================================
// Section 8.4: Function Bodies
// Reference: docs/lonala.md#84-function-bodies
// ============================================================================

/// Spec 8.4: "Function bodies can contain multiple expressions"
#[test]
fn test_8_4_multiple_expressions() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def counter 0)").unwrap();
    let _res = ctx
        .eval("(def process (fn [x] (def counter (+ counter 1)) (* x 2)))")
        .unwrap();
    ctx.assert_int(
        "(process 5)",
        10,
        &spec_ref("8.4", "Bodies", "returns last expression"),
    );
    ctx.assert_int(
        "counter",
        1,
        &spec_ref("8.4", "Bodies", "earlier expressions executed"),
    );
}

// ============================================================================
// Section 8.5: Higher-Order Functions
// Reference: docs/lonala.md#85-higher-order-functions
// ============================================================================

/// Spec 8.5: "Functions can accept functions as arguments"
#[test]
fn test_8_5_functions_as_args() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def apply-twice (fn [f x] (f (f x))))").unwrap();
    let _res = ctx.eval("(def inc (fn [x] (+ x 1)))").unwrap();
    ctx.assert_int(
        "(apply-twice inc 5)",
        7,
        &spec_ref("8.5", "Higher-Order", "function as argument"),
    );
}

/// Spec 8.5: "Functions can... return functions"
#[test]
fn test_8_5_functions_as_return() {
    let mut ctx = SpecTestContext::new();
    // Note: This requires closures which may not be fully implemented
    // Using a simpler example that doesn't require closure capture
    let _res = ctx
        .eval("(def get-adder (fn [] (fn [x y] (+ x y))))")
        .unwrap();
    let _res = ctx.eval("(def my-add (get-adder))").unwrap();
    ctx.assert_int(
        "(my-add 3 4)",
        7,
        &spec_ref("8.5", "Higher-Order", "function as return value"),
    );
}

/// [IGNORED] Spec 8.5: Closures capture lexical environment
/// Tracking: Closures planned for Phase 5.2
#[test]
#[ignore]
fn test_8_5_closures() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def make-adder (fn [n] (fn [x] (+ x n))))")
        .unwrap();
    let _res = ctx.eval("(def add-5 (make-adder 5))").unwrap();
    ctx.assert_int(
        "(add-5 10)",
        15,
        &spec_ref("8.5", "Higher-Order", "closure captures n"),
    );
}

// ============================================================================
// Section 8.6: Recursion
// Reference: docs/lonala.md#86-recursion
// ============================================================================

/// Spec 8.6: "Named functions can call themselves recursively"
#[test]
fn test_8_6_recursion_via_global() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def sum-to (fn sum-to [n] (if (<= n 0) 0 (+ n (sum-to (- n 1))))))")
        .unwrap();
    ctx.assert_int(
        "(sum-to 5)",
        15,
        &spec_ref("8.6", "Recursion", "recursive sum 5+4+3+2+1"),
    );
}

/// Spec 8.6: Factorial example
#[test]
fn test_8_6_factorial() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def factorial (fn factorial [n] (if (<= n 1) 1 (* n (factorial (- n 1))))))")
        .unwrap();
    ctx.assert_int(
        "(factorial 5)",
        120,
        &spec_ref("8.6", "Recursion", "factorial(5) = 120"),
    );
}

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
