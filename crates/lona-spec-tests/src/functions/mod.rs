// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 6.5 - Functions
//!
//! Reference: docs/lonala/special-forms.md#65-fn
//!
//! Tests function definition, calling, arity, higher-order functions,
//! and recursion.
//!
//! This module is split into submodules:
//! - `multi_arity`: Multi-arity functions
//! - `destructuring`: Parameter destructuring (sequential and map patterns)

mod destructuring;
mod multi_arity;

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
    // This example returns a function that doesn't capture variables,
    // so it works without closure VM support
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

/// Spec 8.5: Closures capture lexical environment
#[test]
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

/// Spec 8.5: Closure captures multiple variables
#[test]
fn test_8_5_closure_multiple_captures() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def make-fn (fn [a b] (fn [x] (+ x a b))))")
        .unwrap();
    let _res = ctx.eval("(def f (make-fn 10 20))").unwrap();
    ctx.assert_int(
        "(f 5)",
        35,
        &spec_ref("8.5", "Higher-Order", "closure captures multiple vars"),
    );
}

/// Spec 8.5: Nested closure (grandparent capture)
#[test]
fn test_8_5_closure_nested_transitive() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def outer (fn [a] (fn [] (fn [] a))))").unwrap();
    let _res = ctx.eval("(def middle (outer 42))").unwrap();
    let _res = ctx.eval("(def inner (middle))").unwrap();
    ctx.assert_int(
        "(inner)",
        42,
        &spec_ref(
            "8.5",
            "Higher-Order",
            "transitive capture through middle fn",
        ),
    );
}

/// Spec 8.5: Closure captures value at creation time (copy semantics)
#[test]
fn test_8_5_closure_copy_semantics() {
    let mut ctx = SpecTestContext::new();
    // Using let shadowing to verify copy-at-creation
    let _res = ctx
        .eval("(def result (let [x 1 f (fn [] x) x 2] (f)))")
        .unwrap();
    ctx.assert_int(
        "result",
        1,
        &spec_ref("8.5", "Higher-Order", "closure captures value, not binding"),
    );
}

/// Spec 8.5: Multi-arity closure shares captured values
#[test]
fn test_8_5_closure_multi_arity() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def make-fn (fn [x] (fn ([a] (+ a x)) ([a b] (+ a b x)))))")
        .unwrap();
    let _res = ctx.eval("(def f (make-fn 100))").unwrap();
    ctx.assert_int(
        "(f 1)",
        101,
        &spec_ref("8.5", "Higher-Order", "1-arity uses capture"),
    );
    ctx.assert_int(
        "(f 1 2)",
        103,
        &spec_ref("8.5", "Higher-Order", "2-arity uses same capture"),
    );
}

/// Spec 8.5: Closure returned from function (counter factory)
#[test]
fn test_8_5_closure_counter_factory() {
    let mut ctx = SpecTestContext::new();
    // Each call to make-counter creates independent closure
    let _res = ctx
        .eval("(def make-counter (fn [start] (fn [inc] (+ start inc))))")
        .unwrap();
    let _res = ctx.eval("(def counter-10 (make-counter 10))").unwrap();
    let _res = ctx.eval("(def counter-100 (make-counter 100))").unwrap();
    ctx.assert_int(
        "(counter-10 5)",
        15,
        &spec_ref("8.5", "Higher-Order", "first counter"),
    );
    ctx.assert_int(
        "(counter-100 5)",
        105,
        &spec_ref("8.5", "Higher-Order", "second counter independent"),
    );
}

/// Spec 8.5: Closure equality is identity-based
#[test]
fn test_8_5_closure_identity_equality() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def make-adder (fn [n] (fn [x] (+ x n))))")
        .unwrap();
    // Same code, same captured value, but different closure instances
    ctx.assert_bool(
        "(= (make-adder 5) (make-adder 5))",
        false,
        &spec_ref("8.5", "Higher-Order", "different closures are not equal"),
    );
    // Same closure instance
    let _res = ctx.eval("(def f (make-adder 5))").unwrap();
    ctx.assert_bool(
        "(= f f)",
        true,
        &spec_ref("8.5", "Higher-Order", "same closure is equal to itself"),
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
