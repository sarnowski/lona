// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 5 - Symbols and Evaluation
//!
//! Reference: docs/lonala.md#5-symbols-and-evaluation
//!
//! Tests symbol resolution and evaluation rules.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 5.1: Evaluation Rules
// Reference: docs/lonala.md#51-evaluation-rules
// ============================================================================

/// Spec 5.1: "Self-evaluating values: Numbers... evaluate to themselves"
#[test]
fn test_5_1_numbers_self_evaluate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "42",
        42,
        &spec_ref("5.1", "Evaluation", "integers self-evaluate"),
    );
    ctx.assert_float(
        "3.14",
        3.14,
        &spec_ref("5.1", "Evaluation", "floats self-evaluate"),
    );
}

/// Spec 5.1: "Self-evaluating values: ...strings... evaluate to themselves"
#[test]
fn test_5_1_strings_self_evaluate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "\"hello\"",
        "hello",
        &spec_ref("5.1", "Evaluation", "strings self-evaluate"),
    );
}

/// Spec 5.1: "Self-evaluating values: ...booleans, nil... evaluate to themselves"
#[test]
fn test_5_1_booleans_nil_self_evaluate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "true",
        true,
        &spec_ref("5.1", "Evaluation", "true self-evaluates"),
    );
    ctx.assert_bool(
        "false",
        false,
        &spec_ref("5.1", "Evaluation", "false self-evaluates"),
    );
    ctx.assert_nil("nil", &spec_ref("5.1", "Evaluation", "nil self-evaluates"));
}

/// Spec 5.1: "Symbols: Look up the symbol's value in the current environment"
#[test]
fn test_5_1_symbol_lookup() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 100)").unwrap();
    ctx.assert_int("x", 100, &spec_ref("5.1", "Evaluation", "symbol lookup"));
}

/// Spec 5.1: "Lists: Treat the first element as a function/special form"
#[test]
fn test_5_1_list_as_function_call() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+ 1 2)",
        3,
        &spec_ref("5.1", "Evaluation", "list as function call"),
    );
}

// ============================================================================
// Section 5.2: Symbol Resolution
// Reference: docs/lonala.md#52-symbol-resolution
// ============================================================================

/// Spec 5.2: "Local bindings: Parameters and let-bound variables"
#[test]
fn test_5_2_local_bindings_let() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [y 20] y)",
        20,
        &spec_ref("5.2", "Resolution", "let-bound variable"),
    );
}

/// Spec 5.2: "Local bindings: Parameters..."
#[test]
fn test_5_2_local_bindings_fn_params() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def add-ten (fn [n] (+ n 10)))").unwrap();
    ctx.assert_int(
        "(add-ten 5)",
        15,
        &spec_ref("5.2", "Resolution", "function parameter binding"),
    );
}

/// Spec 5.2: "Global definitions: Values bound with def"
#[test]
fn test_5_2_global_bindings() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def global-var 42)").unwrap();
    ctx.assert_int(
        "global-var",
        42,
        &spec_ref("5.2", "Resolution", "global definition"),
    );
}

/// Spec 5.2: Local bindings shadow global
#[test]
fn test_5_2_local_shadows_global() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def x 10)").unwrap();
    ctx.assert_int(
        "(let [x 20] x)",
        20,
        &spec_ref("5.2", "Resolution", "local shadows global"),
    );
    // Global still accessible after let scope
    ctx.assert_int(
        "x",
        10,
        &spec_ref("5.2", "Resolution", "global unchanged after let"),
    );
}

// ============================================================================
// Section 5.3: Qualified Symbols
// Reference: docs/lonala.md#53-qualified-symbols
// ============================================================================

/// [IGNORED] Spec 5.3: Full namespace support planned
/// Tracking: Phase 6 implementation
#[test]
#[ignore]
fn test_5_3_qualified_symbols() {
    let mut _ctx = SpecTestContext::new();
    // Qualified symbol tests when namespaces implemented
}

// ============================================================================
// Section 5.4: Preventing Evaluation
// Reference: docs/lonala.md#54-preventing-evaluation
// ============================================================================

/// Spec 5.4: "Use quote to prevent evaluation"
#[test]
fn test_5_4_quote_prevents_evaluation() {
    let mut ctx = SpecTestContext::new();
    // Quoting a symbol returns the symbol, not its value
    ctx.assert_symbol("'foo", &spec_ref("5.4", "Quote", "quote returns symbol"));
    // Quoting a list returns the list, not the result of calling it
    ctx.assert_list(
        "'(+ 1 2)",
        &spec_ref("5.4", "Quote", "quote returns list unevaluated"),
    );
}

/// Spec 5.4: "' reader macro is shorthand for (quote form)"
#[test]
fn test_5_4_quote_reader_macro() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol("'foo", &spec_ref("5.4", "Quote", "quote reader macro"));
}
