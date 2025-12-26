// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 10 - Reader Macros
//!
//! Reference: docs/lonala.md#10-reader-macros
//!
//! Tests reader macros that transform syntax during the read phase.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 10.1: Quote (')
// Reference: docs/lonala.md#101-quote-
// ============================================================================

/// Spec 10.1: "Expands to: (quote form)"
#[test]
fn test_10_1_quote_expands_to_quote() {
    let mut ctx = SpecTestContext::new();
    // 'foo should be equivalent to (quote foo)
    ctx.assert_symbol_eq(
        "'foo",
        "foo",
        &spec_ref("10.1", "Quote", "quote symbol 'foo'"),
    );
}

/// Spec 10.1: Quote list
#[test]
fn test_10_1_quote_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "'(1 2 3)",
        "'(1 2 3)",
        &spec_ref("10.1", "Quote", "quote list yields (1 2 3)"),
    );
}

/// Spec 10.1: Quote vector
#[test]
fn test_10_1_quote_vector() {
    let mut ctx = SpecTestContext::new();
    // Verify the quoted vector has 3 symbol elements
    ctx.assert_vector_len(
        "'[a b c]",
        3,
        &spec_ref("10.1", "Quote", "quoted vector has 3 elements"),
    );
}

// ============================================================================
// Section 10.2: Syntax-Quote (`)
// Reference: docs/lonala.md#102-syntax-quote-
// ============================================================================

/// Spec 10.2: "Template quoting that allows selective unquoting"
#[test]
fn test_10_2_syntax_quote_basic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "`foo",
        "foo",
        &spec_ref("10.2", "Syntax-Quote", "quoted symbol 'foo'"),
    );
    ctx.assert_list_eq(
        "`(1 2 3)",
        "'(1 2 3)",
        &spec_ref("10.2", "Syntax-Quote", "quoted list yields (1 2 3)"),
    );
}

/// Spec 10.2: Syntax-quote preserves type
#[test]
fn test_10_2_syntax_quote_preserves_type() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "`42",
        42,
        &spec_ref("10.2", "Syntax-Quote", "integer passes through"),
    );
    ctx.assert_string(
        "`\"hello\"",
        "hello",
        &spec_ref("10.2", "Syntax-Quote", "string passes through"),
    );
}

// ============================================================================
// Section 10.3: Unquote (~)
// Reference: docs/lonala.md#103-unquote-
// ============================================================================

/// Spec 10.3: "Evaluates form and inserts the result into the surrounding template"
#[test]
fn test_10_3_unquote_evaluates() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 10)").unwrap();
    // Result should be a list containing (1 10 3)
    ctx.assert_list_eq(
        "`(1 ~x 3)",
        "'(1 10 3)",
        &spec_ref("10.3", "Unquote", "unquote evaluates x to produce (1 10 3)"),
    );
}

/// Spec 10.3: Unquote with expression
#[test]
fn test_10_3_unquote_expression() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 1)").unwrap();
    let _res = ctx.eval("(def y 2)").unwrap();
    // `(~x ~y ~(+ x y)) should produce (1 2 3)
    ctx.assert_list_eq(
        "`(~x ~y ~(+ x y))",
        "'(1 2 3)",
        &spec_ref("10.3", "Unquote", "unquote expression yields (1 2 3)"),
    );
}

/// Spec 10.3: Unquote symbol in operator position
#[test]
fn test_10_3_unquote_operator() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def op '+)").unwrap();
    // `(~op 1 2) should produce (+ 1 2)
    ctx.assert_list_len(
        "`(~op 1 2)",
        3,
        &spec_ref("10.3", "Unquote", "operator position list has 3 elements"),
    );
    ctx.assert_symbol_eq(
        "(first `(~op 1 2))",
        "+",
        &spec_ref("10.3", "Unquote", "first element is symbol '+'"),
    );
}

// ============================================================================
// Section 10.4: Unquote-Splicing (~@)
// Reference: docs/lonala.md#104-unquote-splicing-
// ============================================================================

/// Spec 10.4: "Evaluates form (which must be a sequence) and splices its elements"
#[test]
fn test_10_4_unquote_splice_basic() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def nums (list 2 3 4))").unwrap();
    // `(1 ~@nums 5) should produce (1 2 3 4 5)
    ctx.assert_list_eq(
        "`(1 ~@nums 5)",
        "'(1 2 3 4 5)",
        &spec_ref("10.4", "Unquote-Splicing", "splice yields (1 2 3 4 5)"),
    );
}

/// Spec 10.4: Empty splice
#[test]
fn test_10_4_unquote_splice_empty() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def empty (list))").unwrap();
    // `(1 ~@empty 2) should produce (1 2)
    ctx.assert_list_eq(
        "`(1 ~@empty 2)",
        "'(1 2)",
        &spec_ref("10.4", "Unquote-Splicing", "empty splice yields (1 2)"),
    );
}

/// Spec 10.4: Difference from unquote - unquote inserts as single element
#[test]
fn test_10_4_unquote_vs_splice() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def xs (list 1 2 3))").unwrap();
    // ~xs inserts the list as one element: (a (1 2 3) b)
    ctx.assert_list_len(
        "`(a ~xs b)",
        3,
        &spec_ref(
            "10.4",
            "Unquote-Splicing",
            "unquote produces 3-element list",
        ),
    );
    // ~@xs splices the elements: (a 1 2 3 b)
    ctx.assert_list_eq(
        "`(a ~@xs b)",
        "'(a 1 2 3 b)",
        &spec_ref("10.4", "Unquote-Splicing", "splice yields (a 1 2 3 b)"),
    );
}

/// Spec 10.4: Splice for building function calls
#[test]
fn test_10_4_splice_function_call() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def args (list 1 2 3))").unwrap();
    // `(+ ~@args) should produce (+ 1 2 3)
    ctx.assert_list_len(
        "`(+ ~@args)",
        4,
        &spec_ref(
            "10.4",
            "Unquote-Splicing",
            "function call list has 4 elements",
        ),
    );
    ctx.assert_symbol_eq(
        "(first `(+ ~@args))",
        "+",
        &spec_ref("10.4", "Unquote-Splicing", "first element is '+'"),
    );
}

// ============================================================================
// Section 10.5: Nested Syntax-Quote
// Reference: docs/lonala.md#105-nested-syntax-quote
// ============================================================================

/// [IGNORED] Spec 10.5: Nested syntax-quote preserves inner quote
/// Tracking: Complex nested quoting needs additional work
#[test]
#[ignore]
fn test_10_5_nested_syntax_quote_inner_literal() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 10)").unwrap();
    // `(a `(b ~x)) - inner ~x should NOT be evaluated, it's quoted
    ctx.assert_list_len(
        "`(a `(b ~x))",
        2,
        &spec_ref("10.5", "Nested Quote", "outer list has 2 elements"),
    );
}

/// [IGNORED] Spec 10.5: Double unquote ~~x
/// Tracking: Complex nested quoting needs additional work
#[test]
#[ignore]
fn test_10_5_double_unquote() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 10)").unwrap();
    // `(a `(b ~~x)) - double unquote evaluates in outer context
    // Result should have the evaluated value
    ctx.assert_list_len(
        "`(a `(b ~~x))",
        2,
        &spec_ref("10.5", "Nested Quote", "double unquote"),
    );
}

/// [IGNORED] Spec 10.5: Nested unquote-splicing
/// Tracking: Complex nested quoting needs additional work
#[test]
#[ignore]
fn test_10_5_nested_unquote_splice() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def xs '(1 2 3))").unwrap();
    // Nested splicing behavior
    ctx.assert_list_len(
        "`(a `(b ~@xs))",
        2,
        &spec_ref("10.5", "Nested Quote", "nested unquote-splice"),
    );
}

/// [IGNORED] Spec 10.5: Three levels of nesting
/// Tracking: Complex nested quoting needs additional work
#[test]
#[ignore]
fn test_10_5_three_levels() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 10)").unwrap();
    // ```(~~~x) - three levels deep
    ctx.assert_list_len(
        "```(~~~x)",
        1,
        &spec_ref("10.5", "Nested Quote", "three levels of nesting"),
    );
}

// ============================================================================
// Section 10.6: Anonymous Function
// Reference: docs/lonala.md#106-anonymous-function
// ============================================================================

/// Spec 10.6: #() creates anonymous function
#[test]
fn test_10_6_anonymous_fn_basic() {
    let mut ctx = SpecTestContext::new();
    // #(+ % 1) is sugar for (fn [p1] (+ p1 1))
    ctx.assert_int(
        "(#(+ % 1) 5)",
        6,
        &spec_ref("10.6", "#()", "basic anonymous function"),
    );
}

/// Spec 10.6: Multiple arguments with %1, %2
#[test]
fn test_10_6_anonymous_fn_multiple_args() {
    let mut ctx = SpecTestContext::new();
    // #(+ %1 %2) is sugar for (fn [p1 p2] (+ p1 p2))
    ctx.assert_int(
        "(#(+ %1 %2) 3 4)",
        7,
        &spec_ref("10.6", "#()", "multiple arguments"),
    );
}

/// Spec 10.6: Rest arguments with %&
#[test]
fn test_10_6_anonymous_fn_rest() {
    let mut ctx = SpecTestContext::new();
    // #(list %&) is sugar for (fn [& rest] (list rest))
    // The result is a list containing one element (the rest list)
    ctx.assert_list_len(
        "(#(list %&) 1 2 3)",
        1,
        &spec_ref("10.6", "#()", "rest arguments"),
    );
}

// ============================================================================
// Section 10.7: Var Quote
// Reference: docs/lonala/reader-macros.md#var-quote
// ============================================================================

/// Spec 10.7: #'symbol returns the var
#[test]
fn test_10_7_var_quote() {
    use lona_core::value::Value;

    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def my-var 42)").unwrap();
    // #'my-var expands to (var my-var) and returns the var object
    let result = ctx.eval("#'my-var");
    assert!(
        matches!(result, Ok(Value::Var(_))),
        "var quote should return a Var value: {:?}",
        result
    );
}

/// Spec 10.7: #' with qualified symbol
#[test]
fn test_10_7_var_quote_qualified() {
    use lona_core::value::Value;

    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def foo/bar 100)").unwrap();
    // #'foo/bar expands to (var foo/bar)
    let result = ctx.eval("#'foo/bar");
    assert!(
        matches!(result, Ok(Value::Var(_))),
        "var quote with qualified symbol should return a Var value: {:?}",
        result
    );
}

/// Spec 10.7: var-get retrieves the value bound to a var
#[test]
fn test_10_7_var_get() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 42)").unwrap();
    ctx.assert_int(
        "(var-get #'x)",
        42_i64,
        &spec_ref("10.7", "#'", "var-get retrieves var value"),
    );
}

/// Spec 10.7: var-set! updates the value bound to a var
#[test]
fn test_10_7_var_set() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def y 1)").unwrap();
    // var-set! returns the new value
    ctx.assert_int(
        "(var-set! #'y 100)",
        100_i64,
        &spec_ref("10.7", "#'", "var-set! returns new value"),
    );
    // The var is now updated
    ctx.assert_int(
        "y",
        100_i64,
        &spec_ref("10.7", "#'", "var-set! updates var"),
    );
}

/// Spec 10.7: var-get with non-var produces type error
#[test]
fn test_10_7_var_get_type_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(var-get 42)",
        &spec_ref("10.7", "#'", "var-get requires Var argument"),
    );
}

/// Spec 10.7: var-set! with non-var produces type error
#[test]
fn test_10_7_var_set_type_error() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(var-set! 42 100)",
        &spec_ref("10.7", "#'", "var-set! requires Var argument"),
    );
}

// ============================================================================
// Section 10.8: Discard
// Reference: docs/lonala/reader-macros.md#discard
// ============================================================================

/// Spec 10.8: #_ discards the next form
#[test]
fn test_10_8_discard() {
    let mut ctx = SpecTestContext::new();
    // #_form reads and discards the form
    ctx.assert_int(
        "(+ 1 #_2 3)",
        4,
        &spec_ref("10.8", "#_", "discards form, result is 1+3"),
    );
}

/// Spec 10.8: Discard in collection
#[test]
fn test_10_8_discard_in_collection() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_len(
        "[1 #_2 3 4]",
        3,
        &spec_ref("10.8", "#_", "vector has 3 elements after discard"),
    );
}

/// Spec 10.8: Chained discards
#[test]
fn test_10_8_discard_chained() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_len(
        "[1 #_#_2 3 4]",
        2,
        &spec_ref("10.8", "#_", "chained discards remove both 2 and 3"),
    );
}

/// Spec 10.8: Discard complex form
#[test]
fn test_10_8_discard_complex() {
    let mut ctx = SpecTestContext::new();
    // (+ 1 #_(* 2 3) 4) discards (* 2 3), evaluates (+ 1 4) = 5
    ctx.assert_int(
        "(+ 1 #_(* 2 3) 4)",
        5,
        &spec_ref("10.8", "#_", "discards entire nested form"),
    );
}

// ============================================================================
// Section 10.9: Regex Literal (Planned)
// Reference: docs/lonala.md#109-regex-literal
// ============================================================================

/// [IGNORED] Spec 10.9: #"pattern" creates compiled regex
/// Tracking: Regex literal reader macro not yet implemented
#[test]
#[ignore]
fn test_10_9_regex_literal() {
    let mut ctx = SpecTestContext::new();
    // #"\\d+" is sugar for (re-pattern "\\d+")
    ctx.assert_string(
        "(re-find #\"\\d+\" \"abc123\")",
        "123",
        &spec_ref("10.9", "#\"\"", "regex literal matches digits"),
    );
}

// ============================================================================
// Section 10.10: Metadata (Planned)
// Reference: docs/lonala.md#1010-metadata
// ============================================================================

/// [IGNORED] Spec 10.10: ^{:key val} attaches metadata map
/// Tracking: Metadata reader macro not yet implemented
#[test]
#[ignore]
fn test_10_10_metadata_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map(
        "(meta ^{:doc \"A vector\"} [1 2 3])",
        &spec_ref("10.10", "^", "metadata map attached"),
    );
}

/// [IGNORED] Spec 10.10: ^:keyword shorthand for {:keyword true}
/// Tracking: Metadata reader macro not yet implemented
#[test]
#[ignore]
fn test_10_10_metadata_keyword() {
    let mut ctx = SpecTestContext::new();
    // ^:private expands to ^{:private true}
    ctx.assert_bool(
        "(get (meta ^:private 'my-var) :private)",
        true,
        &spec_ref("10.10", "^:", "keyword shorthand sets true"),
    );
}

/// [IGNORED] Spec 10.10: Multiple metadata items
/// Tracking: Metadata reader macro not yet implemented
#[test]
#[ignore]
fn test_10_10_metadata_multiple() {
    let mut ctx = SpecTestContext::new();
    // ^:private ^:dynamic my-var => ^{:private true :dynamic true} my-var
    ctx.assert_map(
        "(meta ^:private ^:dynamic 'my-var)",
        &spec_ref("10.10", "^:", "multiple metadata items merge"),
    );
}
