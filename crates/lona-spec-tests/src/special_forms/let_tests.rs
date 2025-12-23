// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 6.2 - let with destructuring
//!
//! Reference: docs/lonala.md#62-let

use crate::{SpecTestContext, spec_ref};

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
// Section 6.2.1: let with sequential destructuring
// Reference: docs/lonala.md#62-let (destructuring subsection)
// ============================================================================

/// Spec 6.2.1: Simple destructuring with fixed elements
#[test]
fn test_6_2_1_let_destructure_fixed_elements() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[a b c] [1 2 3]] (+ a (+ b c)))",
        6,
        &spec_ref("6.2.1", "let", "destructure fixed elements [a b c]"),
    );
}

/// Spec 6.2.1: Destructuring with rest binding
#[test]
fn test_6_2_1_let_destructure_rest_binding() {
    let mut ctx = SpecTestContext::new();
    // Rest binding produces a list
    ctx.assert_list_eq(
        "(let [[a b & r] [1 2 3 4]] r)",
        "'(3 4)",
        &spec_ref("6.2.1", "let", "rest binding collects remaining as list"),
    );
}

/// Spec 6.2.1: Destructuring with ignore (_)
#[test]
fn test_6_2_1_let_destructure_ignore() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[a _ c] [1 2 3]] c)",
        3,
        &spec_ref("6.2.1", "let", "_ ignores position"),
    );
}

/// Spec 6.2.1: Destructuring with :as whole binding
#[test]
fn test_6_2_1_let_destructure_as_binding() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_eq(
        "(let [[a :as all] [1 2]] all)",
        "[1 2]",
        &spec_ref("6.2.1", "let", ":as binds original collection"),
    );
}

/// Spec 6.2.1: Nested destructuring
#[test]
fn test_6_2_1_let_destructure_nested() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[[a b] c] [[1 2] 3]] (+ a (+ b c)))",
        6,
        &spec_ref("6.2.1", "let", "nested destructuring [[a b] c]"),
    );
}

/// Spec 6.2.1: Destructuring nil returns nil for elements
#[test]
fn test_6_2_1_let_destructure_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(let [[a] nil] a)",
        &spec_ref("6.2.1", "let", "destructuring nil yields nil"),
    );
}

/// Spec 6.2.1: Destructuring short collection yields nil for missing
#[test]
fn test_6_2_1_let_destructure_short_collection() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(let [[a b] [1]] b)",
        &spec_ref("6.2.1", "let", "missing elements are nil"),
    );
}

/// Spec 6.2.1: Destructuring with both :as and & rest
#[test]
fn test_6_2_1_let_destructure_as_and_rest() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_eq(
        "(let [[a & r :as all] [1 2 3]] all)",
        "[1 2 3]",
        &spec_ref("6.2.1", "let", ":as with rest still binds original"),
    );
    let mut ctx2 = SpecTestContext::new();
    ctx2.assert_list_eq(
        "(let [[a & r :as all] [1 2 3]] r)",
        "'(2 3)",
        &spec_ref("6.2.1", "let", "rest binding with :as"),
    );
}

/// Spec 6.2.1: Destructuring in complex expression
#[test]
fn test_6_2_1_let_destructure_in_expression() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[x y] [10 20]] (let [[a b] [x y]] (+ a b)))",
        30,
        &spec_ref("6.2.1", "let", "nested let with destructuring"),
    );
}

// ============================================================================
// Section 6.2.2: let with associative (map) destructuring
// Reference: docs/lonala/special-forms.md#associative-destructuring
// ============================================================================

/// Spec 6.2.2: Basic :keys destructuring
#[test]
fn test_6_2_2_let_map_destructure_keys() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{:keys [a b]} {:a 1 :b 2}] (+ a b))",
        3,
        &spec_ref("6.2.2", "let", ":keys extracts keyword keys"),
    );
}

/// Spec 6.2.2: :strs destructuring (string keys)
#[test]
fn test_6_2_2_let_map_destructure_strs() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(let [{:strs [name]} {\"name\" \"Alice\"}] name)",
        "Alice",
        &spec_ref("6.2.2", "let", ":strs extracts string keys"),
    );
}

/// Spec 6.2.2: :syms destructuring (symbol keys)
#[test]
fn test_6_2_2_let_map_destructure_syms() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{:syms [x]} {'x 42}] x)",
        42,
        &spec_ref("6.2.2", "let", ":syms extracts symbol keys"),
    );
}

/// Spec 6.2.2: :or provides default values
#[test]
fn test_6_2_2_let_map_destructure_or() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{:keys [a b] :or {b 0}} {:a 1}] (+ a b))",
        1,
        &spec_ref("6.2.2", "let", ":or provides default for missing key"),
    );
}

/// Spec 6.2.2: :or default applies only on nil, not false
#[test]
fn test_6_2_2_let_map_destructure_or_nil_only() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(let [{:keys [a] :or {a true}} {:a false}] a)",
        false,
        &spec_ref("6.2.2", "let", ":or default only on nil, not false"),
    );
}

/// Spec 6.2.2: :as binds whole map
#[test]
fn test_6_2_2_let_map_destructure_as() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(let [{:keys [a] :as m} {:a 1 :b 2}] m)",
        "{:a 1 :b 2}",
        &spec_ref("6.2.2", "let", ":as binds entire map"),
    );
}

/// Spec 6.2.2: Explicit key binding
#[test]
fn test_6_2_2_let_map_destructure_explicit() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{x :foo} {:foo 42}] x)",
        42,
        &spec_ref("6.2.2", "let", "explicit binding {x :key}"),
    );
}

/// Spec 6.2.2: Missing key returns nil
#[test]
fn test_6_2_2_let_map_destructure_missing_key() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(let [{:keys [a]} {}] a)",
        &spec_ref("6.2.2", "let", "missing key returns nil"),
    );
}

/// Spec 6.2.2: Destructuring nil map
#[test]
fn test_6_2_2_let_map_destructure_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(let [{:keys [a]} nil] a)",
        &spec_ref("6.2.2", "let", "nil map yields nil for keys"),
    );
}

/// Spec 6.2.2: Combined patterns (:keys, :or, :as)
#[test]
fn test_6_2_2_let_map_destructure_combined() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_eq(
        "(let [{:keys [a b] :or {b 100} :as m} {:a 1}] [a b])",
        "[1 100]",
        &spec_ref("6.2.2", "let", "combined :keys, :or, :as"),
    );
}

// ============================================================================
// Section 6.2.3: let with nested destructuring (sequential → map)
// Reference: docs/lonala/special-forms.md#nested-destructuring
// ============================================================================

/// Spec 6.2.3: Sequential → Map nesting with :keys
#[test]
fn test_6_2_3_let_destructure_seq_to_map_keys() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[{:keys [a]} b] [{:a 1} 2]] (+ a b))",
        3,
        &spec_ref("6.2.3", "let", "sequential → map nesting with :keys"),
    );
}

/// Spec 6.2.3: Sequential → Map nesting with explicit binding
#[test]
fn test_6_2_3_let_destructure_seq_to_map_explicit() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[{x :foo}] [{:foo 42}]] x)",
        42,
        &spec_ref(
            "6.2.3",
            "let",
            "sequential → map nesting with explicit binding",
        ),
    );
}

/// Spec 6.2.3: Sequential → Map with :or default
#[test]
fn test_6_2_3_let_destructure_seq_to_map_or() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[{:keys [a] :or {a 100}}] [{}]] a)",
        100,
        &spec_ref("6.2.3", "let", "sequential → map with :or default"),
    );
}

/// Spec 6.2.3: Sequential → Map with :as binding
#[test]
fn test_6_2_3_let_destructure_seq_to_map_as() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(let [[{:keys [a] :as m}] [{:a 1 :b 2}]] m)",
        "{:a 1 :b 2}",
        &spec_ref("6.2.3", "let", "sequential → map with :as binding"),
    );
}

/// Spec 6.2.3: Multiple map patterns in sequence
#[test]
fn test_6_2_3_let_destructure_multiple_maps_in_seq() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[{:keys [a]} {:keys [b]}] [{:a 10} {:b 20}]] (+ a b))",
        30,
        &spec_ref("6.2.3", "let", "multiple map patterns in sequence"),
    );
}

/// Spec 6.2.3: Mixed map and symbol in sequence
#[test]
fn test_6_2_3_let_destructure_mixed_map_and_symbol() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[{:keys [x]} y z] [{:x 1} 2 3]] (+ x y z))",
        6,
        &spec_ref("6.2.3", "let", "mixed map and symbol bindings in sequence"),
    );
}

// ============================================================================
// Section 6.2.4: let with nested destructuring (map → sequential/map)
// Reference: docs/lonala/special-forms.md#nested-destructuring
// ============================================================================

/// Spec 6.2.4: Map → Sequential nesting
#[test]
fn test_6_2_4_let_destructure_map_to_seq() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{[a b] :point} {:point [1 2]}] (+ a b))",
        3,
        &spec_ref("6.2.4", "let", "map → sequential nesting {[a b] :key}"),
    );
}

/// Spec 6.2.4: Map → Map nesting
#[test]
fn test_6_2_4_let_destructure_map_to_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{{:keys [x]} :inner} {:inner {:x 99}}] x)",
        99,
        &spec_ref("6.2.4", "let", "map → map nesting {{:keys [x]} :key}"),
    );
}

/// Spec 6.2.4: Map → Sequential with multiple elements
#[test]
fn test_6_2_4_let_destructure_map_to_seq_multiple() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{[a b c] :coords} {:coords [10 20 30]}] (+ a b c))",
        60,
        &spec_ref("6.2.4", "let", "map → sequential with multiple elements"),
    );
}

/// Spec 6.2.4: Map → Sequential with rest binding
#[test]
fn test_6_2_4_let_destructure_map_to_seq_rest() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "(let [{[a & rest] :items} {:items [1 2 3 4]}] rest)",
        "'(2 3 4)",
        &spec_ref("6.2.4", "let", "map → sequential with rest binding"),
    );
}

/// Spec 6.2.4: Map → Map with :or default
#[test]
fn test_6_2_4_let_destructure_map_to_map_or() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{{:keys [x] :or {x 100}} :inner} {:inner {}}] x)",
        100,
        &spec_ref("6.2.4", "let", "map → map with :or default"),
    );
}

/// Spec 6.2.4: Missing key with nested pattern returns nil elements
#[test]
fn test_6_2_4_let_destructure_map_to_seq_missing() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(let [{[a] :missing} {}] a)",
        &spec_ref("6.2.4", "let", "missing key with nested pattern yields nil"),
    );
}

/// Spec 6.2.4: Deep nesting (3 levels) - seq → map → seq
#[test]
fn test_6_2_4_let_destructure_deep_nesting() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [[{[a b] :p}] [{:p [10 20]}]] (+ a b))",
        30,
        &spec_ref("6.2.4", "let", "3-level nesting: seq → map → seq"),
    );
}

/// Spec 6.2.4: Deep nesting - map → seq → map
#[test]
fn test_6_2_4_let_destructure_deep_nesting_map_seq_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{[{:keys [x]}] :items} {:items [{:x 42}]}] x)",
        42,
        &spec_ref("6.2.4", "let", "3-level nesting: map → seq → map"),
    );
}

/// Spec 6.2.4: Map → Sequential with ignore (_)
#[test]
fn test_6_2_4_let_destructure_map_to_seq_ignore() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(let [{[_ b _] :triple} {:triple [1 2 3]}] b)",
        2,
        &spec_ref("6.2.4", "let", "map → sequential with ignore"),
    );
}
