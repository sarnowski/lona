// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 11 - Macros
//!
//! Reference: docs/lonala.md#11-macros
//!
//! Tests macro definition and introspection.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 11.1: Defining Macros (defmacro)
// Reference: docs/lonala.md#111-defining-macros
// ============================================================================

/// Spec 11.1: "Returns: The symbol name"
#[test]
fn test_11_1_defmacro_returns_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(defmacro identity [x] x)",
        "identity",
        &spec_ref("11.1", "defmacro", "returns symbol 'identity'"),
    );
}

/// Spec 11.1: Macro is stored and can be used
#[test]
fn test_11_1_defmacro_stored_in_registry() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(defmacro id [x] x)").unwrap();
    // Using the macro should work
    ctx.assert_int(
        "(id 42)",
        42,
        &spec_ref("11.1", "defmacro", "macro can be called"),
    );
}

/// Spec 11.1: Macro with quasiquote body
#[test]
fn test_11_1_defmacro_with_quasiquote() {
    let mut ctx = SpecTestContext::new();
    // Define unless macro: (unless test body) -> (if (not test) body nil)
    let _res = ctx
        .eval("(defmacro unless [test body] `(if (not ~test) ~body nil))")
        .unwrap();
    ctx.assert_int(
        "(unless false 42)",
        42,
        &spec_ref("11.1", "defmacro", "macro with quasiquote"),
    );
    ctx.assert_nil(
        "(unless true 42)",
        &spec_ref("11.1", "defmacro", "unless with true test"),
    );
}

/// Spec 11.1: Macro arity mismatch produces error
#[test]
fn test_11_1_defmacro_arity_error() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(defmacro single [x] x)").unwrap();
    ctx.assert_error_contains(
        "(single 1 2)",
        "expects 1 arguments",
        &spec_ref("11.1", "defmacro", "arity mismatch error"),
    );
}

// ============================================================================
// Section 11.2: Macro Introspection
// Reference: docs/lonala.md#112-macro-introspection
// ============================================================================

/// Spec 11.2: "macro? returns true for macros"
#[test]
fn test_11_2_macro_predicate_true() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(defmacro my-macro [x] x)").unwrap();
    ctx.assert_bool(
        "(macro? 'my-macro)",
        true,
        &spec_ref("11.2", "macro?", "returns true for defined macro"),
    );
}

/// Spec 11.2: "macro? returns false for non-macros"
#[test]
fn test_11_2_macro_predicate_false_special_form() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(macro? 'if)",
        false,
        &spec_ref("11.2", "macro?", "returns false for special form"),
    );
}

/// Spec 11.2: macro? returns false for undefined
#[test]
fn test_11_2_macro_predicate_false_undefined() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(macro? 'undefined-symbol)",
        false,
        &spec_ref("11.2", "macro?", "returns false for undefined symbol"),
    );
}

/// Spec 11.2: macro? returns false for regular functions
#[test]
fn test_11_2_macro_predicate_false_function() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def my-fn (fn [x] x))").unwrap();
    ctx.assert_bool(
        "(macro? 'my-fn)",
        false,
        &spec_ref("11.2", "macro?", "returns false for regular function"),
    );
}

/// Spec 11.2: macroexpand-1 expands exactly once
#[test]
fn test_11_2_macroexpand_1() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(defmacro unless [test body] `(if (not ~test) ~body nil))")
        .unwrap();
    // macroexpand-1 should return the expanded form: (if (not false) 42 nil)
    // First element should be 'if'
    ctx.assert_symbol_eq(
        "(first (macroexpand-1 '(unless false 42)))",
        "if",
        &spec_ref("11.2", "macroexpand-1", "expands to if-form"),
    );
    // Should have 4 elements: if, (not false), 42, nil
    ctx.assert_list_len(
        "(macroexpand-1 '(unless false 42))",
        4,
        &spec_ref("11.2", "macroexpand-1", "expanded form has 4 elements"),
    );
}

/// Spec 11.2: macroexpand-1 on non-macro returns unchanged
#[test]
fn test_11_2_macroexpand_1_non_macro() {
    let mut ctx = SpecTestContext::new();
    // Expanding a non-macro call should return the form unchanged: (+ 1 2)
    ctx.assert_list_eq(
        "(macroexpand-1 '(+ 1 2))",
        "'(+ 1 2)",
        &spec_ref(
            "11.2",
            "macroexpand-1",
            "non-macro returns (+ 1 2) unchanged",
        ),
    );
}

/// Spec 11.2: macroexpand keeps expanding until top-level is not a macro
#[test]
fn test_11_2_macroexpand_full() {
    let mut ctx = SpecTestContext::new();
    // Define two macros where one expands to another
    let _res = ctx
        .eval("(defmacro when [test body] `(if ~test ~body nil))")
        .unwrap();
    let _res = ctx
        .eval("(defmacro unless [test body] `(when (not ~test) ~body))")
        .unwrap();
    // macroexpand should fully expand: unless -> when -> if
    // Final form should start with 'if'
    ctx.assert_symbol_eq(
        "(first (macroexpand '(unless false 42)))",
        "if",
        &spec_ref("11.2", "macroexpand", "fully expands to if-form"),
    );
}

/// Spec 11.2: macroexpand on non-macro returns unchanged
#[test]
fn test_11_2_macroexpand_non_macro() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(macroexpand '(+ 1 2))",
        "'(+ 1 2)",
        &spec_ref("11.2", "macroexpand", "non-macro returns (+ 1 2) unchanged"),
    );
}

// ============================================================================
// Section 11.3: Common Macro Patterns
// Reference: docs/lonala.md#113-common-macro-patterns
// ============================================================================

/// Spec 11.3: when macro with rest args
/// Tests that rest arguments (&) work in macro parameter lists
#[test]
fn test_11_3_when_with_rest_args() {
    let mut ctx = SpecTestContext::new();
    // (defmacro when [test & body] `(if ~test (do ~@body) nil))
    let _res = ctx
        .eval("(defmacro when [test & body] `(if ~test (do ~@body) nil))")
        .unwrap();
    ctx.assert_int(
        "(when true 1 2 3)",
        3,
        &spec_ref("11.3", "when", "multiple body expressions"),
    );
}

/// Spec 11.3: defn macro
/// Tests that defn macro works correctly with rest args
#[test]
fn test_11_3_defn_macro() {
    let mut ctx = SpecTestContext::new();
    // (defmacro defn [name params & body] `(def ~name (fn ~name ~params ~@body)))
    let _res = ctx
        .eval("(defmacro defn [name params & body] `(def ~name (fn ~name ~params ~@body)))")
        .unwrap();
    // Define a function using defn
    let _res = ctx.eval("(defn add [a b] (+ a b))").unwrap();
    // Test that the function works
    ctx.assert_int(
        "(add 1 2)",
        3,
        &spec_ref("11.3", "defn", "defines function"),
    );
}

/// Spec 11.3: Function rest args
/// Tests that rest arguments (&) work in function parameter lists
#[test]
fn test_11_3_fn_rest_args() {
    let mut ctx = SpecTestContext::new();
    // Define a function with rest args
    let _res = ctx.eval("(def sum-list (fn [& args] args))").unwrap();
    // Test with various numbers of arguments
    ctx.assert_list_len(
        "(sum-list)",
        0,
        &spec_ref("11.3", "fn rest", "empty rest args yields ()"),
    );
    ctx.assert_list_eq(
        "(sum-list 1 2 3)",
        "'(1 2 3)",
        &spec_ref("11.3", "fn rest", "multiple rest args yields (1 2 3)"),
    );
}

/// Spec 11.3: Function with fixed and rest args
/// Tests that fixed and rest parameters work together
#[test]
fn test_11_3_fn_fixed_and_rest_args() {
    let mut ctx = SpecTestContext::new();
    // Define a function with one fixed param and rest args
    let _res = ctx.eval("(def take-first-rest (fn [x & rest] x))").unwrap();
    ctx.assert_int(
        "(take-first-rest 1 2 3)",
        1,
        &spec_ref("11.3", "fn rest", "fixed arg returned"),
    );
    // Define a function that returns the rest
    let _res = ctx.eval("(def get-rest (fn [x & rest] rest))").unwrap();
    ctx.assert_list_eq(
        "(get-rest 1 2 3)",
        "'(2 3)",
        &spec_ref("11.3", "fn rest", "rest args yields (2 3)"),
    );
}
