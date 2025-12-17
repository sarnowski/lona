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
    ctx.assert_symbol("'foo", &spec_ref("10.1", "Quote", "quote symbol"));
}

/// Spec 10.1: Quote list
#[test]
fn test_10_1_quote_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list("'(1 2 3)", &spec_ref("10.1", "Quote", "quote list"));
}

/// Spec 10.1: Quote vector
#[test]
fn test_10_1_quote_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector("'[a b c]", &spec_ref("10.1", "Quote", "quote vector"));
}

// ============================================================================
// Section 10.2: Syntax-Quote (`)
// Reference: docs/lonala.md#102-syntax-quote-
// ============================================================================

/// Spec 10.2: "Template quoting that allows selective unquoting"
#[test]
fn test_10_2_syntax_quote_basic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol("`foo", &spec_ref("10.2", "Syntax-Quote", "quoted symbol"));
    ctx.assert_list("`(1 2 3)", &spec_ref("10.2", "Syntax-Quote", "quoted list"));
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
    ctx.assert_list(
        "`(1 ~x 3)",
        &spec_ref("10.3", "Unquote", "unquote evaluates x"),
    );
}

/// Spec 10.3: Unquote with expression
#[test]
fn test_10_3_unquote_expression() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 1)").unwrap();
    let _res = ctx.eval("(def y 2)").unwrap();
    // `(~x ~y ~(+ x y)) should produce (1 2 3)
    ctx.assert_list(
        "`(~x ~y ~(+ x y))",
        &spec_ref("10.3", "Unquote", "unquote evaluates expression"),
    );
}

/// Spec 10.3: Unquote symbol in operator position
#[test]
fn test_10_3_unquote_operator() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def op '+)").unwrap();
    // `(~op 1 2) should produce (+ 1 2)
    ctx.assert_list(
        "`(~op 1 2)",
        &spec_ref("10.3", "Unquote", "unquote in operator position"),
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
    ctx.assert_list(
        "`(1 ~@nums 5)",
        &spec_ref("10.4", "Unquote-Splicing", "splice list elements"),
    );
}

/// Spec 10.4: Empty splice
#[test]
fn test_10_4_unquote_splice_empty() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def empty (list))").unwrap();
    // `(1 ~@empty 2) should produce (1 2)
    ctx.assert_list(
        "`(1 ~@empty 2)",
        &spec_ref("10.4", "Unquote-Splicing", "empty list splices to nothing"),
    );
}

/// Spec 10.4: Difference from unquote - unquote inserts as single element
#[test]
fn test_10_4_unquote_vs_splice() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def xs (list 1 2 3))").unwrap();
    // ~xs inserts the list as one element
    ctx.assert_list(
        "`(a ~xs b)",
        &spec_ref("10.4", "Unquote-Splicing", "unquote inserts as one element"),
    );
    // ~@xs splices the elements
    ctx.assert_list(
        "`(a ~@xs b)",
        &spec_ref(
            "10.4",
            "Unquote-Splicing",
            "splice inserts elements separately",
        ),
    );
}

/// Spec 10.4: Splice for building function calls
#[test]
fn test_10_4_splice_function_call() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def args (list 1 2 3))").unwrap();
    // `(+ ~@args) should produce (+ 1 2 3)
    ctx.assert_list(
        "`(+ ~@args)",
        &spec_ref(
            "10.4",
            "Unquote-Splicing",
            "build function call with splice",
        ),
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
