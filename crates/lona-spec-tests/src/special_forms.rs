// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 6 - Special Forms
//!
//! Reference: docs/lonala.md#6-special-forms
//!
//! Tests fundamental language constructs with evaluation rules that differ
//! from normal function calls.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 6.1: def
// Reference: docs/lonala.md#61-def
// ============================================================================

/// Spec 6.1: "Returns: The symbol name"
#[test]
fn test_6_1_def_returns_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(def x 42)",
        "x",
        &spec_ref("6.1", "def", "returns the symbol 'x'"),
    );
}

/// Spec 6.1: "Evaluates value and binds the result to name"
#[test]
fn test_6_1_def_creates_global() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def greeting \"Hello, World!\")").unwrap();
    ctx.assert_string(
        "greeting",
        "Hello, World!",
        &spec_ref("6.1", "def", "creates global binding"),
    );
}

/// Spec 6.1: "If name is already defined, it is rebound to the new value"
#[test]
fn test_6_1_def_rebinding() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 1)").unwrap();
    let _res = ctx.eval("(def x 2)").unwrap();
    ctx.assert_int("x", 2, &spec_ref("6.1", "def", "rebinding updates value"));
}

/// Spec 6.1: def can bind function values
#[test]
fn test_6_1_def_function() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def square (fn [n] (* n n)))").unwrap();
    ctx.assert_int(
        "(square 5)",
        25,
        &spec_ref("6.1", "def", "can bind functions"),
    );
}

// ============================================================================
// Section 6.2: let
// Reference: docs/lonala.md#62-let
// ============================================================================

/// Spec 6.2: Single binding
#[test]
fn test_6_2_let_single_binding() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [x 10] x)",
        10,
        &spec_ref("6.2", "let", "single binding"),
    );
}

/// Spec 6.2: Multiple bindings
#[test]
fn test_6_2_let_multiple_bindings() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [x 10 y 20] (+ x y))",
        30,
        &spec_ref("6.2", "let", "multiple bindings"),
    );
}

/// Spec 6.2: "Each binding can refer to previously bound names"
#[test]
fn test_6_2_let_forward_reference() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [x 10 y (* x 2)] y)",
        20,
        &spec_ref("6.2", "let", "forward reference in bindings"),
    );
}

/// Spec 6.2: Inner let shadows outer
#[test]
fn test_6_2_let_shadowing() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [x 1] (let [x 2] x))",
        2,
        &spec_ref("6.2", "let", "inner let shadows outer"),
    );
}

/// Spec 6.2: "Returns: The value of the last body expression, or nil if body is empty"
#[test]
fn test_6_2_let_empty_body_returns_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(let [x 1])",
        &spec_ref("6.2", "let", "empty body returns nil"),
    );
}

// ============================================================================
// Section 6.3: if
// Reference: docs/lonala.md#63-if
// ============================================================================

/// Spec 6.3: "If the result is truthy... evaluates and returns then"
#[test]
fn test_6_3_if_true_branch() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if true 1 2)",
        1,
        &spec_ref("6.3", "if", "true condition returns then branch"),
    );
}

/// Spec 6.3: "Otherwise, evaluates and returns else"
#[test]
fn test_6_3_if_false_branch() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if false 1 2)",
        2,
        &spec_ref("6.3", "if", "false condition returns else branch"),
    );
    ctx.assert_int(
        "(if nil 1 2)",
        2,
        &spec_ref("6.3", "if", "nil condition returns else branch"),
    );
}

/// Spec 6.3: "(or nil if else is omitted)"
#[test]
fn test_6_3_if_no_else_returns_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(if false 1)",
        &spec_ref("6.3", "if", "no else branch returns nil"),
    );
}

/// Spec 6.3: "0 is truthy" (in if context)
#[test]
fn test_6_3_if_zero_is_truthy() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(if 0 \"yes\" \"no\")",
        "yes",
        &spec_ref("6.3", "if", "0 is truthy"),
    );
}

/// Spec 6.3: Empty collections are truthy
#[test]
fn test_6_3_if_empty_collection_truthy() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(if '() \"yes\" \"no\")",
        "yes",
        &spec_ref("6.3", "if", "empty list is truthy"),
    );
}

// ============================================================================
// Section 6.4: do
// Reference: docs/lonala.md#64-do
// ============================================================================

/// Spec 6.4: "Returns: The value of the last expression, or nil if empty"
#[test]
fn test_6_4_do_empty_returns_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil("(do)", &spec_ref("6.4", "do", "empty do returns nil"));
}

/// Spec 6.4: Returns last expression
#[test]
fn test_6_4_do_returns_last() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(do 1 2 3)",
        3,
        &spec_ref("6.4", "do", "returns last expression"),
    );
}

/// Spec 6.4: Evaluates all expressions in order
#[test]
fn test_6_4_do_evaluates_all() {
    let mut ctx = SpecTestContext::new();
    // Use def to verify all expressions are evaluated
    let _res = ctx.eval("(do (def a 1) (def b 2) (def c 3))").unwrap();
    ctx.assert_int("a", 1, &spec_ref("6.4", "do", "first expression evaluated"));
    ctx.assert_int(
        "b",
        2,
        &spec_ref("6.4", "do", "second expression evaluated"),
    );
    ctx.assert_int("c", 3, &spec_ref("6.4", "do", "third expression evaluated"));
}

// ============================================================================
// Section 6.5: fn
// Reference: docs/lonala.md#65-fn
// ============================================================================

/// Spec 6.5: Anonymous function
#[test]
fn test_6_5_fn_anonymous() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_function(
        "(fn [x] (* x x))",
        &spec_ref("6.5", "fn", "creates anonymous function"),
    );
}

/// Spec 6.5: Named function (for recursion)
#[test]
fn test_6_5_fn_named() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_function(
        "(fn fact [n] (if (<= n 1) 1 (* n (fact (- n 1)))))",
        &spec_ref("6.5", "fn", "creates named function"),
    );
}

/// Spec 6.5: No parameters
#[test]
fn test_6_5_fn_no_params() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [] 42))",
        42,
        &spec_ref("6.5", "fn", "function with no parameters"),
    );
}

/// Spec 6.5: Multiple parameters
#[test]
fn test_6_5_fn_multiple_params() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [a b c] (+ a (+ b c))) 1 2 3)",
        6,
        &spec_ref("6.5", "fn", "function with multiple parameters"),
    );
}

/// Spec 6.5: Empty body returns nil implicitly
#[test]
fn test_6_5_fn_empty_body() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "((fn []))",
        &spec_ref("6.5", "fn", "empty body returns nil"),
    );
}

/// Spec 6.5: Multiple body expressions
#[test]
fn test_6_5_fn_multiple_body_exprs() {
    let mut ctx = SpecTestContext::new();
    // Define with side effect to prove all are evaluated
    let _res = ctx.eval("(def counter 0)").unwrap();
    let _res = ctx
        .eval("(def inc-and-return (fn [x] (def counter (+ counter 1)) x))")
        .unwrap();
    ctx.assert_int(
        "(inc-and-return 42)",
        42,
        &spec_ref("6.5", "fn", "returns last body expression"),
    );
    ctx.assert_int(
        "counter",
        1,
        &spec_ref("6.5", "fn", "evaluates all body expressions"),
    );
}

/// Spec 6.5: Arity error
#[test]
fn test_6_5_fn_arity_error() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def one-arg (fn [x] x))").unwrap();
    ctx.assert_error(
        "(one-arg)",
        &spec_ref("6.5", "fn", "too few arguments is an error"),
    );
    ctx.assert_error(
        "(one-arg 1 2)",
        &spec_ref("6.5", "fn", "too many arguments is an error"),
    );
}

// ============================================================================
// Section 6.6: quote
// Reference: docs/lonala.md#66-quote
// ============================================================================

/// Spec 6.6: Quote symbol
#[test]
fn test_6_6_quote_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(quote foo)",
        "foo",
        &spec_ref("6.6", "quote", "returns symbol 'foo' unevaluated"),
    );
}

/// Spec 6.6: Quote list - verify structure
#[test]
fn test_6_6_quote_list() {
    let mut ctx = SpecTestContext::new();
    // Verify the quoted list (+ 1 2) has correct structure
    ctx.assert_list_len(
        "(quote (+ 1 2))",
        3,
        &spec_ref("6.6", "quote", "quoted list has 3 elements"),
    );
    ctx.assert_symbol_eq(
        "(first (quote (+ 1 2)))",
        "+",
        &spec_ref("6.6", "quote", "first element is symbol '+'"),
    );
    ctx.assert_int(
        "(first (rest (quote (+ 1 2))))",
        1,
        &spec_ref("6.6", "quote", "second element is 1"),
    );
    ctx.assert_int(
        "(first (rest (rest (quote (+ 1 2)))))",
        2,
        &spec_ref("6.6", "quote", "third element is 2"),
    );
}

/// Spec 6.6: Quote prevents evaluation
#[test]
fn test_6_6_quote_prevents_evaluation() {
    let mut ctx = SpecTestContext::new();
    // (+ 1 2) would be 3 if evaluated, but quote returns the list with symbol + as first element
    ctx.assert_symbol_eq(
        "(first '(+ 1 2))",
        "+",
        &spec_ref("6.6", "quote", "quoted list contains symbol, not evaluated"),
    );
}

// ============================================================================
// Section 6.7: syntax-quote
// Reference: docs/lonala.md#67-syntax-quote
// ============================================================================

/// Spec 6.7: Syntax-quote on literals
#[test]
fn test_6_7_syntax_quote_literals() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "`42",
        42,
        &spec_ref("6.7", "syntax-quote", "integer passes through"),
    );
    ctx.assert_string(
        "`\"hello\"",
        "hello",
        &spec_ref("6.7", "syntax-quote", "string passes through"),
    );
}

/// Spec 6.7: Syntax-quote on symbols
#[test]
fn test_6_7_syntax_quote_symbols() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "`foo",
        "foo",
        &spec_ref("6.7", "syntax-quote", "symbol 'foo' is quoted"),
    );
}

/// Spec 6.7: Syntax-quote on lists
#[test]
fn test_6_7_syntax_quote_lists() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "`(1 2 3)",
        "'(1 2 3)",
        &spec_ref("6.7", "syntax-quote", "list (1 2 3) is quoted"),
    );
}

/// Spec 6.7: Unquote evaluates within syntax-quote
#[test]
fn test_6_7_unquote() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 10)").unwrap();
    // The list should be (1 10 3) with evaluated x
    ctx.assert_list_eq(
        "`(1 ~x 3)",
        "'(1 10 3)",
        &spec_ref(
            "6.7",
            "syntax-quote",
            "unquote evaluates x to produce (1 10 3)",
        ),
    );
}

/// Spec 6.7: Unquote-splicing splices sequence elements
#[test]
fn test_6_7_unquote_splicing() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def nums (list 2 3 4))").unwrap();
    ctx.assert_list_eq(
        "`(1 ~@nums 5)",
        "'(1 2 3 4 5)",
        &spec_ref("6.7", "syntax-quote", "unquote-splicing yields (1 2 3 4 5)"),
    );
}
