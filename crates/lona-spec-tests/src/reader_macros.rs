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

/// [IGNORED] Spec 10.5: Complex nested quoting
/// Tracking: Complex nested quoting needs additional work
#[test]
#[ignore]
fn test_10_5_nested_syntax_quote() {
    let mut _ctx = SpecTestContext::new();
    // Nested syntax-quote tests when fully supported
}
