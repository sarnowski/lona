// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Parameter Destructuring
//!
//! Reference: docs/lonala/special-forms.md#65-fn

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 8.8: Parameter Destructuring
// Reference: docs/lonala/special-forms.md#fn
// ============================================================================

/// Spec 8.8: Destructuring parameter - basic fixed elements
#[test]
fn test_8_8_destructuring_basic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[a b]] (+ a b)) [1 2])",
        3,
        &spec_ref("8.8", "Destructuring", "basic [a b] extracts elements"),
    );
}

/// Spec 8.8: Destructuring parameter - three elements
#[test]
fn test_8_8_destructuring_three_elements() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[a b c]] (+ a b c)) [1 2 3])",
        6,
        &spec_ref("8.8", "Destructuring", "[a b c] extracts three elements"),
    );
}

/// Spec 8.8: Destructuring parameter - with rest binding
#[test]
fn test_8_8_destructuring_with_rest() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "((fn [[a & r]] r) [1 2 3])",
        "(list 2 3)",
        &spec_ref("8.8", "Destructuring", "rest binding collects remainder"),
    );
}

/// Spec 8.8: Destructuring parameter - rest binding empty
#[test]
fn test_8_8_destructuring_rest_empty() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_len(
        "((fn [[a & r]] r) [1])",
        0,
        &spec_ref(
            "8.8",
            "Destructuring",
            "rest binding empty when no remainder",
        ),
    );
}

/// Spec 8.8: Destructuring parameter - with ignore
#[test]
fn test_8_8_destructuring_with_ignore() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[a _ c]] c) [1 2 3])",
        3,
        &spec_ref("8.8", "Destructuring", "_ ignores middle element"),
    );
}

/// Spec 8.8: Destructuring parameter - with :as binding
#[test]
fn test_8_8_destructuring_with_as() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_vector_eq(
        "((fn [[a :as all]] all) [1 2 3])",
        "[1 2 3]",
        &spec_ref("8.8", "Destructuring", ":as binds whole collection"),
    );
}

/// Spec 8.8: Destructuring parameter - nested pattern
#[test]
fn test_8_8_destructuring_nested() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[[x y] z]] (+ x y z)) [[1 2] 3])",
        6,
        &spec_ref("8.8", "Destructuring", "nested pattern extracts deeply"),
    );
}

/// Spec 8.8: Multiple parameters with some destructured
#[test]
fn test_8_8_mixed_params() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[a b] c] (+ a b c)) [1 2] 3)",
        6,
        &spec_ref(
            "8.8",
            "Destructuring",
            "mixed destructured and simple params",
        ),
    );
}

/// Spec 8.8: Destructuring in multi-arity function
#[test]
fn test_8_8_multi_arity_destructuring() {
    let mut ctx = SpecTestContext::new();
    // 1-arity takes a vector, 2-arity takes a vector plus a number
    let _res = ctx
        .eval("(def f (fn ([[a b]] (+ a b)) ([[a b] c] (+ a b c))))")
        .unwrap();
    ctx.assert_int(
        "(f [10 20])",
        30,
        &spec_ref("8.8", "Destructuring", "1-arity destructuring"),
    );
    ctx.assert_int(
        "(f [10 20] 5)",
        35,
        &spec_ref(
            "8.8",
            "Destructuring",
            "2-arity destructuring with extra param",
        ),
    );
}

/// Spec 8.8: Destructuring with short collection binds nil
#[test]
fn test_8_8_short_collection() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "((fn [[a b]] b) [1])",
        &spec_ref("8.8", "Destructuring", "missing element binds to nil"),
    );
}

/// Spec 8.8: Destructuring nil behaves like empty collection
#[test]
fn test_8_8_nil_input() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "((fn [[a]] a) nil)",
        &spec_ref("8.8", "Destructuring", "nil input binds elements to nil"),
    );
}

/// Spec 8.8: Ignored parameter `_`
#[test]
fn test_8_8_ignored_param() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [_ x] x) 1 2)",
        2,
        &spec_ref("8.8", "Destructuring", "_ ignores first parameter"),
    );
}

/// Spec 8.8: Ignored rest parameter `& _`
#[test]
fn test_8_8_ignored_rest_param() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [x & _] x) 1 2 3 4)",
        1,
        &spec_ref("8.8", "Destructuring", "& _ ignores all rest arguments"),
    );
}

/// Spec 8.8: Destructuring with closure capture in multi-arity
#[test]
fn test_8_8_closure_destructuring_multi_arity() {
    let mut ctx = SpecTestContext::new();
    // Multi-arity function: 1-arity destructures, 2-arity destructures + extra param
    // Both capture 'x' from outer scope
    let _res = ctx
        .eval("(def make-fn (fn [x] (fn ([[a b]] (+ x a b)) ([[a b] c] (+ x a b c)))))")
        .unwrap();
    let _res = ctx.eval("(def f (make-fn 100))").unwrap();
    ctx.assert_int(
        "(f [1 2])",
        103,
        &spec_ref("8.8", "Destructuring", "1-arity with capture"),
    );
    ctx.assert_int(
        "(f [1 2] 10)",
        113,
        &spec_ref("8.8", "Destructuring", "2-arity with same capture"),
    );
}

/// Spec 8.8: Destructuring pattern as rest parameter
#[test]
fn test_8_8_rest_destructuring() {
    let mut ctx = SpecTestContext::new();
    // Note: The rest parameter collects remaining args as a list.
    // Destructuring [a b] extracts from that list.
    ctx.assert_int(
        "((fn [x & [a b]] (+ x a b)) 1 2 3)",
        6,
        &spec_ref("8.8", "Destructuring", "rest param with destructuring"),
    );
}

/// Spec 8.8: Destructuring rest parameter with :as
#[test]
fn test_8_8_rest_destructuring_with_as() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "((fn [x & [a :as all]] all) 1 2 3 4)",
        "(list 2 3 4)",
        &spec_ref("8.8", "Destructuring", "rest destructuring with :as"),
    );
}

// ============================================================================
// Section 8.9: Map Destructuring in Function Parameters
// Reference: docs/lonala/special-forms.md#associative-destructuring
// ============================================================================

/// Spec 8.9: Basic :keys destructuring in fn parameter
#[test]
fn test_8_9_fn_map_destructure_keys() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [{:keys [a b]}] (+ a b)) {:a 1 :b 2})",
        3,
        &spec_ref("8.9", "Map Destructuring", ":keys extracts keyword keys"),
    );
}

/// Spec 8.9: :strs destructuring (string keys) in fn parameter
#[test]
fn test_8_9_fn_map_destructure_strs() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "((fn [{:strs [name]}] name) {\"name\" \"Bob\"})",
        "Bob",
        &spec_ref("8.9", "Map Destructuring", ":strs extracts string keys"),
    );
}

/// Spec 8.9: :syms destructuring in fn parameter
#[test]
fn test_8_9_fn_map_destructure_syms() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [{:syms [x]}] x) {'x 42})",
        42,
        &spec_ref("8.9", "Map Destructuring", ":syms extracts symbol keys"),
    );
}

/// Spec 8.9: :or provides default values in fn parameter
#[test]
fn test_8_9_fn_map_destructure_or() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [{:keys [a b] :or {b 100}}] (+ a b)) {:a 1})",
        101,
        &spec_ref("8.9", "Map Destructuring", ":or provides default"),
    );
}

/// Spec 8.9: :as binds whole map in fn parameter
#[test]
fn test_8_9_fn_map_destructure_as() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "((fn [{:keys [a] :as m}] m) {:a 1 :b 2})",
        "{:a 1 :b 2}",
        &spec_ref("8.9", "Map Destructuring", ":as binds entire map"),
    );
}

/// Spec 8.9: Explicit key binding in fn parameter
#[test]
fn test_8_9_fn_map_destructure_explicit() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [{x :foo}] x) {:foo 99})",
        99,
        &spec_ref("8.9", "Map Destructuring", "explicit binding {x :key}"),
    );
}

/// Spec 8.9: Multiple parameters with map destructuring
#[test]
fn test_8_9_fn_mixed_params_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [x {:keys [a b]}] (+ x a b)) 10 {:a 1 :b 2})",
        13,
        &spec_ref("8.9", "Map Destructuring", "mixed simple and map params"),
    );
}

/// Spec 8.9: Map destructuring in multi-arity function
#[test]
fn test_8_9_fn_multi_arity_map() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def f (fn ([{:keys [a]}] a) ([{:keys [a]} b] (+ a b))))")
        .unwrap();
    ctx.assert_int(
        "(f {:a 10})",
        10,
        &spec_ref("8.9", "Map Destructuring", "1-arity with map"),
    );
    ctx.assert_int(
        "(f {:a 10} 5)",
        15,
        &spec_ref("8.9", "Map Destructuring", "2-arity with map and simple"),
    );
}

/// Spec 8.9: Map destructuring with closure capture
#[test]
fn test_8_9_fn_closure_map_destructure() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def make-fn (fn [x] (fn [{:keys [a b]}] (+ x a b))))")
        .unwrap();
    let _res = ctx.eval("(def f (make-fn 100))").unwrap();
    ctx.assert_int(
        "(f {:a 1 :b 2})",
        103,
        &spec_ref("8.9", "Map Destructuring", "closure captures x"),
    );
}

/// [IGNORED] Spec 8.9: Map destructuring as rest parameter
/// Tracking: Task 1.2.4 - Rest parameter map destructuring requires special handling
/// where flattened key-value pairs are converted to a map (Clojure behavior).
/// See docs/roadmap/milestone-01-rust-foundation/02-language-features.md#task-124
#[test]
#[ignore]
fn test_8_9_fn_rest_map_destructure() {
    let mut ctx = SpecTestContext::new();
    // Note: In Clojure, `& {:keys [y]}` collects remaining args as flattened key-value
    // pairs that are then reassembled into a map. This requires special handling in
    // the compiler to convert the list (1 :y 2) into a map {:y 2} for destructuring.
    ctx.assert_int(
        "((fn [x & {:keys [y]}] (+ x y)) 1 :y 2)",
        3,
        &spec_ref("8.9", "Map Destructuring", "rest param with map options"),
    );
}

// ============================================================================
// Section 8.10: Nested Destructuring in Function Parameters
// Reference: docs/lonala/special-forms.md#nested-destructuring
// ============================================================================

/// Spec 8.10: Sequential → Map nesting in fn parameter with :keys
#[test]
fn test_8_10_fn_nested_seq_to_map_keys() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[{:keys [x]}]] x) [{:x 42}])",
        42,
        &spec_ref(
            "8.10",
            "Nested Destructuring",
            "sequential → map with :keys in fn",
        ),
    );
}

/// Spec 8.10: Sequential → Map nesting with explicit binding
#[test]
fn test_8_10_fn_nested_seq_to_map_explicit() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[{y :key}]] y) [{:key 99}])",
        99,
        &spec_ref(
            "8.10",
            "Nested Destructuring",
            "sequential → map explicit in fn",
        ),
    );
}

/// Spec 8.10: Sequential → Map with :or default in fn
#[test]
fn test_8_10_fn_nested_seq_to_map_or() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[{:keys [a] :or {a 50}}]] a) [{}])",
        50,
        &spec_ref(
            "8.10",
            "Nested Destructuring",
            "sequential → map :or default in fn",
        ),
    );
}

/// Spec 8.10: Mixed sequential and map nesting with multiple elements
#[test]
fn test_8_10_fn_nested_mixed() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[{:keys [a]} b]] (+ a b)) [{:a 10} 20])",
        30,
        &spec_ref(
            "8.10",
            "Nested Destructuring",
            "mixed map and symbol in fn param",
        ),
    );
}

/// Spec 8.10: Multi-arity function with nested destructuring
#[test]
fn test_8_10_fn_multi_arity_nested() {
    let mut ctx = SpecTestContext::new();
    // Uses different arities: 1-arg vs 2-arg (the nested pattern counts as one param)
    let _res = ctx
        .eval("(def f (fn ([[{:keys [a]}]] a) ([[{:keys [a]}] b] (+ a b))))")
        .unwrap();
    ctx.assert_int(
        "(f [{:a 5}])",
        5,
        &spec_ref("8.10", "Nested Destructuring", "multi-arity 1-arg nested"),
    );
    ctx.assert_int(
        "(f [{:a 5}] 10)",
        15,
        &spec_ref("8.10", "Nested Destructuring", "multi-arity 2-arg nested"),
    );
}

/// Spec 8.10: Closure with nested destructuring
#[test]
fn test_8_10_fn_closure_nested() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def make-fn (fn [x] (fn [[{:keys [a]}]] (+ x a))))")
        .unwrap();
    let _res = ctx.eval("(def f (make-fn 100))").unwrap();
    ctx.assert_int(
        "(f [{:a 5}])",
        105,
        &spec_ref(
            "8.10",
            "Nested Destructuring",
            "closure captures with nested param",
        ),
    );
}

// ============================================================================
// Section 8.11: Map → Any Nesting in Function Parameters
// Reference: docs/lonala/special-forms.md#nested-destructuring
// ============================================================================

/// Spec 8.11: Map → Sequential nesting in fn parameter
#[test]
fn test_8_11_fn_map_to_seq() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [{[a b] :point}] (+ a b)) {:point [1 2]})",
        3,
        &spec_ref("8.11", "Map→Any Nesting", "map → sequential in fn param"),
    );
}

/// Spec 8.11: Map → Map nesting in fn parameter
#[test]
fn test_8_11_fn_map_to_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [{{:keys [x]} :inner}] x) {:inner {:x 99}})",
        99,
        &spec_ref("8.11", "Map→Any Nesting", "map → map in fn param"),
    );
}

/// Spec 8.11: Map → Sequential with rest binding in fn
#[test]
fn test_8_11_fn_map_to_seq_rest() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_list_eq(
        "((fn [{[a & rest] :items}] rest) {:items [1 2 3 4]})",
        "'(2 3 4)",
        &spec_ref(
            "8.11",
            "Map→Any Nesting",
            "map → sequential with rest in fn",
        ),
    );
}

/// Spec 8.11: Map → Map with :or default in fn
#[test]
fn test_8_11_fn_map_to_map_or() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [{{:keys [x] :or {x 100}} :inner}] x) {:inner {}})",
        100,
        &spec_ref("8.11", "Map→Any Nesting", "map → map with :or in fn"),
    );
}

/// Spec 8.11: Deep nesting in fn - seq → map → seq
#[test]
fn test_8_11_fn_deep_nesting() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [[{[a b] :p}]] (+ a b)) [{:p [10 20]}])",
        30,
        &spec_ref("8.11", "Map→Any Nesting", "3-level nesting in fn param"),
    );
}

/// Spec 8.11: Map → Sequential with ignore in fn
#[test]
fn test_8_11_fn_map_to_seq_ignore() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "((fn [{[_ b] :pair}] b) {:pair [1 2]})",
        2,
        &spec_ref(
            "8.11",
            "Map→Any Nesting",
            "map → sequential with ignore in fn",
        ),
    );
}

/// Spec 8.11: Multi-arity with map → seq nesting
#[test]
fn test_8_11_fn_multi_arity_map_to_seq() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def f (fn ([{[a b] :p}] (+ a b)) ([{[a b] :p} c] (+ a b c))))")
        .unwrap();
    ctx.assert_int(
        "(f {:p [1 2]})",
        3,
        &spec_ref("8.11", "Map→Any Nesting", "multi-arity 1-arg"),
    );
    ctx.assert_int(
        "(f {:p [1 2]} 10)",
        13,
        &spec_ref("8.11", "Map→Any Nesting", "multi-arity 2-arg"),
    );
}

/// Spec 8.11: Closure with map → seq nesting
#[test]
fn test_8_11_fn_closure_map_to_seq() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def make-fn (fn [x] (fn [{[a b] :p}] (+ x a b))))")
        .unwrap();
    let _res = ctx.eval("(def f (make-fn 100))").unwrap();
    ctx.assert_int(
        "(f {:p [1 2]})",
        103,
        &spec_ref("8.11", "Map→Any Nesting", "closure with map → seq nesting"),
    );
}
