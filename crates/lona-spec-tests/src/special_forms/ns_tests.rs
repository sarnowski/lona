// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 6.X - Namespace Declaration (ns)
//!
//! Reference: docs/lonala/namespaces.md
//!
//! Tests the `ns` special form for declaring and switching namespaces.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Basic ns form
// ============================================================================

/// The ns form returns the namespace name symbol.
#[test]
fn test_ns_returns_namespace_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(ns foo)",
        "foo",
        &spec_ref("6.X", "ns", "returns namespace symbol"),
    );
}

/// The ns form with dotted namespace names.
#[test]
fn test_ns_dotted_name() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(ns my.app.core)",
        "my.app.core",
        &spec_ref("6.X", "ns", "accepts dotted namespace names"),
    );
}

// ============================================================================
// Namespace affects def qualification
// ============================================================================

/// Def in a namespace creates qualified symbol.
#[test]
fn test_ns_def_creates_qualified_symbol() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(ns foo)").unwrap();
    // def x should create foo/x
    ctx.assert_symbol_eq(
        "(def x 42)",
        "foo/x",
        &spec_ref("6.X", "ns", "def returns qualified symbol"),
    );
}

/// Defined value can be accessed with qualified name.
#[test]
fn test_ns_def_accessible_qualified() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(ns bar)").unwrap();
    let _res = ctx.eval("(def y 100)").unwrap();
    ctx.assert_int(
        "bar/y",
        100,
        &spec_ref("6.X", "ns", "qualified lookup works"),
    );
}

/// Multiple definitions in same namespace.
#[test]
fn test_ns_multiple_defs() {
    let mut ctx = SpecTestContext::new();
    // Define both vars in the myns namespace within a single eval
    // This tests that namespace persists WITHIN a single compilation
    ctx.assert_int(
        "(do (ns myns) (def a 1) (def b 2) (+ a b))",
        3,
        &spec_ref("6.X", "ns", "both defs accessible in same eval"),
    );
}

/// Default namespace is "user".
#[test]
fn test_ns_default_is_user() {
    let mut ctx = SpecTestContext::new();
    // Without explicit ns, def should qualify to user/
    ctx.assert_symbol_eq(
        "(def default-val 99)",
        "user/default-val",
        &spec_ref("6.X", "ns", "default namespace is user"),
    );
}

// ============================================================================
// Namespace switching
// ============================================================================

/// Switching namespaces affects subsequent defs.
#[test]
fn test_ns_switching() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(ns first-ns)").unwrap();
    let _res = ctx.eval("(def val 1)").unwrap();
    let _res = ctx.eval("(ns second-ns)").unwrap();
    let _res = ctx.eval("(def val 2)").unwrap();

    ctx.assert_int(
        "first-ns/val",
        1,
        &spec_ref("6.X", "ns", "first ns value preserved"),
    );
    ctx.assert_int(
        "second-ns/val",
        2,
        &spec_ref("6.X", "ns", "second ns value separate"),
    );
}

// ============================================================================
// Error cases
// ============================================================================

/// ns requires a name.
#[test]
fn test_ns_requires_name() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error("(ns)", &spec_ref("6.X", "ns", "requires namespace name"));
}

/// ns name must be a symbol.
#[test]
fn test_ns_name_must_be_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error("(ns 123)", &spec_ref("6.X", "ns", "name must be symbol"));
    ctx.assert_error(
        "(ns \"string\")",
        &spec_ref("6.X", "ns", "string is not valid name"),
    );
    ctx.assert_error(
        "(ns :keyword)",
        &spec_ref("6.X", "ns", "keyword is not valid name"),
    );
}

// ============================================================================
// Require clause with :as alias
// ============================================================================

/// Require with :as creates namespace alias.
/// Note: Aliases are compile-time only, so we test within same compilation.
#[test]
fn test_ns_require_as_alias() {
    let mut ctx = SpecTestContext::new();
    // First define a value in some.long.namespace
    let _res = ctx.eval("(ns some.long.namespace)").unwrap();
    let _res = ctx.eval("(def x 42)").unwrap();

    // Switch to new namespace and use alias in same compilation
    // (aliases are compile-time, so access must be in same eval)
    ctx.assert_int(
        "(do (ns my.app (:require [some.long.namespace :as short])) short/x)",
        42,
        &spec_ref("6.X", "ns", ":require :as creates alias"),
    );
}

/// Multiple aliases in same require clause.
#[test]
fn test_ns_require_multiple_aliases() {
    let mut ctx = SpecTestContext::new();
    // Define values in two namespaces
    let _res = ctx.eval("(ns ns.one) (def a 1)").unwrap();
    let _res = ctx.eval("(ns ns.two) (def b 2)").unwrap();

    // Require both and use within same compilation
    ctx.assert_int(
        "(do (ns my.app (:require [ns.one :as one] [ns.two :as two])) (+ one/a two/b))",
        3,
        &spec_ref("6.X", "ns", "multiple aliases resolve correctly"),
    );
}

// ============================================================================
// Require clause with :refer
// ============================================================================

/// Require with :refer imports specific symbols.
/// Note: Refers are compile-time only, so we test within same compilation.
#[test]
fn test_ns_require_refer() {
    let mut ctx = SpecTestContext::new();
    // Define values in source namespace
    let _res = ctx
        .eval("(ns source.ns) (def foo 10) (def bar 20)")
        .unwrap();

    // Require with :refer and use symbol in same compilation
    ctx.assert_int(
        "(do (ns my.app (:require [source.ns :refer [foo]])) foo)",
        10,
        &spec_ref("6.X", "ns", ":refer makes symbol available unqualified"),
    );
}

/// Require with :refer imports multiple symbols.
#[test]
fn test_ns_require_refer_multiple() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(ns source.ns) (def x 1) (def y 2) (def z 3)")
        .unwrap();

    ctx.assert_int(
        "(do (ns my.app (:require [source.ns :refer [x y z]])) (+ x (+ y z)))",
        6,
        &spec_ref("6.X", "ns", ":refer with multiple symbols"),
    );
}

// ============================================================================
// Combined :as and :refer
// ============================================================================

/// Require with both :as and :refer.
#[test]
fn test_ns_require_as_and_refer() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(ns some.ns) (def a 1) (def b 2)").unwrap();

    // Test both :refer and :as in same compilation
    ctx.assert_int(
        "(do (ns my.app (:require [some.ns :as s :refer [a]])) (+ a s/b))",
        3,
        &spec_ref("6.X", "ns", ":as and :refer work together"),
    );
}

// ============================================================================
// Clause error cases
// ============================================================================

/// Clause must be a list.
#[test]
fn test_ns_clause_must_be_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(ns foo :require)",
        &spec_ref("6.X", "ns", "clause must be list"),
    );
}

/// Clause must start with keyword.
#[test]
fn test_ns_clause_must_start_with_keyword() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(ns foo (require [bar]))",
        &spec_ref("6.X", "ns", "clause must start with :require or :use"),
    );
}

/// Unknown clause type.
#[test]
fn test_ns_unknown_clause_type() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(ns foo (:import [java.util Date]))",
        &spec_ref("6.X", "ns", "unknown clause type"),
    );
}

/// :refer must be a vector.
#[test]
fn test_ns_refer_must_be_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(ns foo (:require [bar :refer (a b)]))",
        &spec_ref("6.X", "ns", ":refer value must be vector"),
    );
}

/// :as requires symbol.
#[test]
fn test_ns_as_requires_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(ns foo (:require [bar :as 123]))",
        &spec_ref("6.X", "ns", ":as value must be symbol"),
    );
}

/// Require vector first element must be namespace symbol.
#[test]
fn test_ns_require_vector_needs_ns() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(ns foo (:require [123 :as x]))",
        &spec_ref(
            "6.X",
            "ns",
            "require vector first element must be namespace",
        ),
    );
}

// ============================================================================
// :use clause (marks for future loading)
// ============================================================================

/// :use clause is accepted (actual loading deferred to Task 1.3.4).
#[test]
fn test_ns_use_clause_accepted() {
    let mut ctx = SpecTestContext::new();
    // This should compile without error, even though loading is deferred
    ctx.assert_symbol_eq(
        "(ns foo (:use bar))",
        "foo",
        &spec_ref("6.X", "ns", ":use clause accepted"),
    );
}

/// :use with multiple namespaces.
#[test]
fn test_ns_use_multiple() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_symbol_eq(
        "(ns foo (:use bar baz qux))",
        "foo",
        &spec_ref("6.X", "ns", ":use with multiple namespaces"),
    );
}

/// :use requires symbol.
#[test]
fn test_ns_use_requires_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(ns foo (:use 123))",
        &spec_ref("6.X", "ns", ":use expects namespace symbols"),
    );
}

// ============================================================================
// var special form with namespace resolution
// ============================================================================

/// var resolves aliases for qualified symbols.
/// If alias resolution fails, we'd get an UndefinedGlobal error.
#[test]
fn test_var_resolves_alias() {
    let mut ctx = SpecTestContext::new();
    // Define a var in source namespace
    let _res = ctx.eval("(ns source.ns) (def my-var 42)").unwrap();

    // Use alias to access the var - if resolution fails, this errors
    let result = ctx.eval("(do (ns my.app (:require [source.ns :as s])) #'s/my-var)");
    assert!(
        result.is_ok(),
        "[Spec 6.X var] #' should resolve aliases: {:?}",
        result.err()
    );
}

/// var resolves referred symbols.
/// If refer resolution fails, we'd get an UndefinedGlobal error.
#[test]
fn test_var_resolves_refer() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(ns source.ns) (def my-fn 99)").unwrap();

    // Refer the symbol and use #' on unqualified name
    let result = ctx.eval("(do (ns my.app (:require [source.ns :refer [my-fn]])) #'my-fn)");
    assert!(
        result.is_ok(),
        "[Spec 6.X var] #' should resolve refers: {:?}",
        result.err()
    );
}
