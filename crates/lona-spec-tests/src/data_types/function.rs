// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Function type.
//!
//! Section 3.13 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.13: Function
// Reference: docs/lonala.md#313-function
// ============================================================================

/// Spec 3.13: Functions are first-class values
#[test]
fn test_3_13_function_first_class() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_function(
        "(fn [x] x)",
        &spec_ref("3.13", "Function", "fn creates a function value"),
    );
}

/// Spec 3.13: Functions can be passed as arguments
#[test]
fn test_3_13_function_as_argument() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def apply-fn (fn [f x] (f x)))").unwrap();
    let _res = ctx.eval("(def double (fn [x] (* x 2)))").unwrap();
    ctx.assert_int(
        "(apply-fn double 5)",
        10,
        &spec_ref("3.13", "Function", "function can be passed as argument"),
    );
}

/// Spec 3.13: Functions can be stored in data structures
#[test]
fn test_3_13_function_in_data_structure() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def fns (list (fn [x] (+ x 1)) (fn [x] (* x 2))))")
        .unwrap();
    ctx.assert_int(
        "((first fns) 5)",
        6,
        &spec_ref("3.13", "Function", "function from list can be called"),
    );
}

/// Spec 3.13: "Functions are compared by identity (same object), not structure"
/// Two structurally identical fn forms should NOT be equal
#[test]
fn test_3_13_function_identity_comparison() {
    let mut ctx = SpecTestContext::new();
    // Create two structurally identical functions
    let _res = ctx.eval("(def f1 (fn [x] x))").unwrap();
    let _res = ctx.eval("(def f2 (fn [x] x))").unwrap();
    // They should NOT be equal because functions compare by identity
    ctx.assert_bool(
        "(= f1 f2)",
        false,
        &spec_ref(
            "3.13",
            "Function",
            "structurally identical functions are not equal",
        ),
    );
}

/// Spec 3.13: Same function value is equal to itself
#[test]
fn test_3_13_function_same_reference() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def f (fn [x] x))").unwrap();
    // Same function should be equal to itself
    ctx.assert_bool(
        "(= f f)",
        true,
        &spec_ref("3.13", "Function", "function is equal to itself"),
    );
}

/// Spec 3.13: Named function for recursion
#[test]
fn test_3_13_named_function() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_function(
        "(fn fact [n] (if (<= n 1) 1 (* n (fact (- n 1)))))",
        &spec_ref("3.13", "Function", "named function for recursion"),
    );
}

/// [IGNORED] Spec 3.13: Closures capture lexical environment
/// Tracking: Task 1.2.2 - Compiler complete (Phase 2), VM pending (Phase 3)
#[test]
#[ignore]
fn test_3_13_closure() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def make-adder (fn [n] (fn [x] (+ x n))))")
        .unwrap();
    let _res = ctx.eval("(def add-5 (make-adder 5))").unwrap();
    ctx.assert_int(
        "(add-5 10)",
        15,
        &spec_ref(
            "3.13",
            "Function",
            "closure captures n from enclosing scope",
        ),
    );
}

/// [IGNORED] Spec 3.13: fn? predicate
/// Tracking: Type predicates not yet exposed as callable functions
#[test]
#[ignore]
fn test_3_13_fn_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(fn? (fn [x] x))",
        true,
        &spec_ref("3.13", "Function", "fn? returns true for function"),
    );
    ctx.assert_bool(
        "(fn? 42)",
        false,
        &spec_ref("3.13", "Function", "fn? returns false for integer"),
    );
}
