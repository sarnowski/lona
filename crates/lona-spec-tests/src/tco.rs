// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tail Call Optimization (TCO) Tests
//!
//! These tests verify that proper tail call optimization works correctly,
//! enabling unbounded recursion in tail position without stack overflow.
//!
//! The tests use iteration counts that would normally cause stack overflow
//! (stack limit is 256 frames) but should complete successfully with TCO.
//!
//! ## Spec Reference
//!
//! TCO is a semantic guarantee (not just optimization) in Lonala. Any call
//! in tail position must not consume stack space, enabling:
//! - Self-recursion (countdown, factorial accumulator)
//! - Mutual recursion (even?/odd?)
//! - State machines (state-a → state-b → state-c → state-a)

extern crate alloc;

use crate::context::SpecTestContext;

// =============================================================================
// Self-Recursion Tests
// =============================================================================

/// Test that simple self-recursion works beyond the normal stack limit.
///
/// Without TCO, this would overflow at depth 256.
/// With TCO, 1000 iterations should complete instantly.
#[test]
fn test_tco_self_recursion_countdown() {
    let mut ctx = SpecTestContext::new();

    // Define countdown function with tail-recursive call using def/fn
    ctx.eval("(def countdown (fn countdown [n] (if (= n 0) :done (countdown (- n 1)))))")
        .unwrap();

    // Test with count well beyond stack limit (256)
    ctx.assert_keyword_eq(
        "(countdown 1000)",
        "done",
        "[TCO self-recursion] countdown(1000) should complete with TCO",
    );
}

/// Test self-recursion with accumulator pattern.
///
/// This is the classic tail-recursive factorial pattern.
#[test]
fn test_tco_self_recursion_with_accumulator() {
    let mut ctx = SpecTestContext::new();

    // Tail-recursive sum: sum-acc(n, acc) = acc if n=0, else sum-acc(n-1, acc+n)
    ctx.eval("(def sum-acc (fn sum-acc [n acc] (if (= n 0) acc (sum-acc (- n 1) (+ acc n)))))")
        .unwrap();

    // Sum 1..500 = 500*501/2 = 125250
    ctx.assert_int(
        "(sum-acc 500 0)",
        125250,
        "[TCO accumulator] sum(1..500) should equal 125250",
    );
}

// =============================================================================
// Mutual Recursion Tests
// =============================================================================

/// Test mutual recursion between two functions.
///
/// The classic even?/odd? mutual recursion pattern.
/// Without TCO, this would overflow very quickly.
#[test]
fn test_tco_mutual_recursion_even_odd() {
    let mut ctx = SpecTestContext::new();

    // Define mutually recursive even?/odd? functions
    ctx.eval("(def my-even? (fn my-even? [n] (if (= n 0) true (my-odd? (- n 1)))))")
        .unwrap();
    ctx.eval("(def my-odd? (fn my-odd? [n] (if (= n 0) false (my-even? (- n 1)))))")
        .unwrap();

    // Test with values beyond stack limit
    ctx.assert_bool("(my-even? 500)", true, "[TCO mutual recursion] 500 is even");

    ctx.assert_bool("(my-odd? 501)", true, "[TCO mutual recursion] 501 is odd");

    ctx.assert_bool(
        "(my-even? 501)",
        false,
        "[TCO mutual recursion] 501 is not even",
    );
}

// =============================================================================
// Tail Position in Control Flow Tests
// =============================================================================

/// Test that both branches of `if` in tail position use TCO.
#[test]
fn test_tco_if_both_branches() {
    let mut ctx = SpecTestContext::new();

    // Function where both if branches are tail calls
    ctx.eval(
        "(def branch-test (fn branch-test [n which] \
           (if (= n 0) \
             which \
             (if which \
               (branch-test (- n 1) true) \
               (branch-test (- n 1) false)))))",
    )
    .unwrap();

    ctx.assert_bool(
        "(branch-test 500 true)",
        true,
        "[TCO if branches] true branch uses TCO",
    );

    ctx.assert_bool(
        "(branch-test 500 false)",
        false,
        "[TCO if branches] false branch uses TCO",
    );
}

/// Test that the last expression in `do` uses TCO.
#[test]
fn test_tco_do_last_expression() {
    let mut ctx = SpecTestContext::new();

    // First define identity function
    ctx.eval("(def identity (fn identity [x] x))").unwrap();

    // Use do with side effect (identity returns its argument)
    ctx.eval(
        "(def do-test (fn do-test [n] \
           (do \
             (identity n) \
             (if (= n 0) \
               :done \
               (do-test (- n 1))))))",
    )
    .unwrap();

    ctx.assert_keyword_eq(
        "(do-test 500)",
        "done",
        "[TCO do] last expression in do uses TCO",
    );
}

/// Test that the body of `let` uses TCO.
#[test]
fn test_tco_let_body() {
    let mut ctx = SpecTestContext::new();

    // Let binding followed by tail call
    ctx.eval(
        "(def let-test (fn let-test [n] \
           (let [m (- n 1)] \
             (if (= m 0) \
               :done \
               (let-test m)))))",
    )
    .unwrap();

    ctx.assert_keyword_eq("(let-test 500)", "done", "[TCO let] body of let uses TCO");
}

// =============================================================================
// State Machine Tests
// =============================================================================

/// Test state machine pattern with multiple states.
///
/// This is a key use case for TCO: expressing state machines
/// as mutually recursive functions.
#[test]
fn test_tco_state_machine() {
    let mut ctx = SpecTestContext::new();

    // Three-state machine that cycles through states
    ctx.eval("(def state-a (fn state-a [n] (if (= n 0) :done-a (state-b (- n 1)))))")
        .unwrap();
    ctx.eval("(def state-b (fn state-b [n] (if (= n 0) :done-b (state-c (- n 1)))))")
        .unwrap();
    ctx.eval("(def state-c (fn state-c [n] (if (= n 0) :done-c (state-a (- n 1)))))")
        .unwrap();

    // 900 transitions = 300 full cycles (900 % 3 = 0 → ends at state-a)
    ctx.assert_keyword_eq(
        "(state-a 900)",
        "done-a",
        "[TCO state machine] 900 transitions (ends at state-a)",
    );

    // 901 % 3 = 1 → ends at state-b
    ctx.assert_keyword_eq(
        "(state-a 901)",
        "done-b",
        "[TCO state machine] 901 transitions (ends at state-b)",
    );

    // 902 % 3 = 2 → ends at state-c
    ctx.assert_keyword_eq(
        "(state-a 902)",
        "done-c",
        "[TCO state machine] 902 transitions (ends at state-c)",
    );
}

// =============================================================================
// Closure Tests
// =============================================================================

/// Test that closures also support TCO.
///
/// Note: Named inner functions need explicit def to be callable by name.
/// The closure captures the target but needs a reference to itself for recursion.
#[test]
fn test_tco_closure() {
    let mut ctx = SpecTestContext::new();

    // Define the closure as a global so it can call itself
    ctx.eval("(def closure-target 42)").unwrap();
    ctx.eval(
        "(def my-countdown (fn my-countdown [n] \
           (if (= n 0) \
             closure-target \
             (my-countdown (- n 1)))))",
    )
    .unwrap();

    ctx.assert_int(
        "(my-countdown 500)",
        42,
        "[TCO closure] closure with captured value uses TCO",
    );
}

// =============================================================================
// Non-Tail Position Verification Tests
// =============================================================================

/// Verify that calls NOT in tail position are correctly identified.
///
/// This test ensures we haven't accidentally made non-tail calls into tail calls.
/// A call wrapped in arithmetic is not in tail position.
#[test]
fn test_non_tail_position_arithmetic() {
    let mut ctx = SpecTestContext::new();

    // This function does NOT use TCO because recursive call is not in tail position
    // The result is used by (+ 1 ...)
    // This should still work for small values
    ctx.eval(
        "(def add-one-recurse (fn add-one-recurse [n] \
           (if (= n 0) 0 (+ 1 (add-one-recurse (- n 1))))))",
    )
    .unwrap();

    // Small value should work (within stack limit)
    ctx.assert_int(
        "(add-one-recurse 50)",
        50,
        "[non-tail position] call in arithmetic operand position works for small values",
    );
}

/// Verify that test expression in `if` is not in tail position.
#[test]
fn test_non_tail_position_if_test() {
    let mut ctx = SpecTestContext::new();

    // The call in if test position is not tail position
    // This tests that the predicate call works correctly
    ctx.eval("(def is-positive (fn is-positive [n] (> n 0)))")
        .unwrap();
    ctx.eval(
        "(def check-positive (fn check-positive [n] \
           (if (is-positive n) :positive :not-positive)))",
    )
    .unwrap();

    ctx.assert_keyword_eq(
        "(check-positive 5)",
        "positive",
        "[non-tail position] call in if test works",
    );

    ctx.assert_keyword_eq(
        "(check-positive 0)",
        "not-positive",
        "[non-tail position] call in if test works for false case",
    );
}

// =============================================================================
// Nested Control Flow Tests
// =============================================================================

/// Test deeply nested control flow maintains TCO.
#[test]
fn test_tco_nested_control_flow() {
    let mut ctx = SpecTestContext::new();

    // First define identity function
    ctx.eval("(def identity (fn identity [x] x))").unwrap();

    // Nested if -> do -> let -> if -> tail call
    ctx.eval(
        "(def nested-test (fn nested-test [n] \
           (if (> n 0) \
             (do \
               (identity nil) \
               (let [m (- n 1)] \
                 (if (= m 0) \
                   :done \
                   (nested-test m)))) \
             :zero)))",
    )
    .unwrap();

    ctx.assert_keyword_eq(
        "(nested-test 500)",
        "done",
        "[TCO nested] deeply nested control flow uses TCO",
    );

    ctx.assert_keyword_eq("(nested-test 0)", "zero", "[TCO nested] zero case works");
}

// =============================================================================
// Multi-Arity Function Tests
// =============================================================================

/// Test that multi-arity functions support TCO.
#[test]
fn test_tco_multi_arity() {
    let mut ctx = SpecTestContext::new();

    // Multi-arity function with default starting value
    ctx.eval(
        "(def multi-count (fn multi-count \
           ([n] (multi-count n :result)) \
           ([n result] \
             (if (= n 0) \
               result \
               (multi-count (- n 1) result)))))",
    )
    .unwrap();

    ctx.assert_keyword_eq(
        "(multi-count 500)",
        "result",
        "[TCO multi-arity] single-arg call dispatches correctly with TCO",
    );

    ctx.assert_int(
        "(multi-count 500 42)",
        42,
        "[TCO multi-arity] two-arg call uses TCO",
    );
}

// =============================================================================
// Rest Parameter Tests
// =============================================================================

/// Test that functions with rest parameters support TCO.
#[test]
fn test_tco_rest_params() {
    let mut ctx = SpecTestContext::new();

    // Function with rest parameter that recurses
    ctx.eval(
        "(def count-with-rest (fn count-with-rest [n & rest] \
           (if (= n 0) \
             (first rest) \
             (count-with-rest (- n 1) (first rest)))))",
    )
    .unwrap();

    ctx.assert_int(
        "(count-with-rest 500 42)",
        42,
        "[TCO rest params] function with rest parameter uses TCO",
    );
}
