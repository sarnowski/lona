// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for tail call optimization in the compiler.
//!
//! These tests verify that the compiler correctly emits `TailCall` opcode
//! for calls in tail position and `Call` opcode for calls not in tail position.

extern crate alloc;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_op};

use super::compile_with_interner;

/// Helper to extract the function body's chunk from a compiled function definition.
///
/// Compiles the source, finds the Function constant, and returns its first body's code.
fn get_fn_body_code(source: &str) -> alloc::vec::Vec<u32> {
    let (chunk, _interner) = compile_with_interner(source);

    // The function constant should be the first one
    for constant in chunk.constants() {
        if let Constant::Function { ref bodies, .. } = *constant {
            if let Some(body) = bodies.first() {
                return body.chunk.code().to_vec();
            }
        }
    }
    panic!("expected to find a Function constant in: {source}");
}

/// Helper to check if any instruction in the code is a TailCall.
fn has_tail_call(code: &[u32]) -> bool {
    code.iter()
        .any(|&instr| decode_op(instr) == Some(Opcode::TailCall))
}

/// Helper to check if any instruction in the code is a regular Call.
fn has_regular_call(code: &[u32]) -> bool {
    code.iter()
        .any(|&instr| decode_op(instr) == Some(Opcode::Call))
}

// =============================================================================
// Direct tail position tests
// =============================================================================

#[test]
fn tail_call_in_fn_body() {
    // (fn [x] (f x)) - call is in tail position
    let code = get_fn_body_code("(fn [x] (f x))");

    assert!(
        has_tail_call(&code),
        "call in tail position should emit TailCall"
    );
    assert!(
        !has_regular_call(&code),
        "should not have regular Call when in tail position"
    );
}

#[test]
fn regular_call_not_in_tail_position() {
    // (fn [x] (+ 1 (f x))) - call is NOT in tail position (result is used by +)
    let code = get_fn_body_code("(fn [x] (+ 1 (f x)))");

    assert!(
        has_regular_call(&code),
        "call not in tail position should emit regular Call"
    );
    assert!(
        !has_tail_call(&code),
        "should not have TailCall when not in tail position"
    );
}

// =============================================================================
// Binary/comparison operators - operands never in tail position
// =============================================================================

#[test]
fn no_tail_call_in_arithmetic_operands() {
    // (fn [x] (+ (f x) (g x))) - both operands are NOT in tail position
    let code = get_fn_body_code("(fn [x] (+ (f x) (g x)))");

    // Should have exactly 2 regular Calls (for f and g)
    let call_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::Call))
        .count();
    assert_eq!(call_count, 2, "both operands should use regular Call");
    assert!(
        !has_tail_call(&code),
        "arithmetic operands should never have TailCall"
    );
}

#[test]
fn no_tail_call_in_comparison_operands() {
    // (fn [x] (> (f x) (g x))) - both operands are NOT in tail position
    let code = get_fn_body_code("(fn [x] (> (f x) (g x)))");

    assert!(
        has_regular_call(&code),
        "comparison operands should use regular Call"
    );
    assert!(
        !has_tail_call(&code),
        "comparison operands should never have TailCall"
    );
}

#[test]
fn no_tail_call_in_unary_negation_operand() {
    // (fn [x] (- (f x))) - unary negation operand is NOT in tail position
    let code = get_fn_body_code("(fn [x] (- (f x)))");

    assert!(
        has_regular_call(&code),
        "unary negation operand should use regular Call"
    );
    assert!(
        !has_tail_call(&code),
        "unary negation operand should never have TailCall"
    );
}

#[test]
fn no_tail_call_in_not_operand() {
    // (fn [x] (not (f x))) - not operand is NOT in tail position
    let code = get_fn_body_code("(fn [x] (not (f x)))");

    assert!(
        has_regular_call(&code),
        "not operand should use regular Call"
    );
    assert!(
        !has_tail_call(&code),
        "not operand should never have TailCall"
    );
}

// =============================================================================
// Tail position in `if` branches
// =============================================================================

#[test]
fn tail_call_in_if_then_branch() {
    // (fn [x] (if test (f x) nil)) - call in then branch is in tail position
    let code = get_fn_body_code("(fn [x] (if test (f x) nil))");

    assert!(
        has_tail_call(&code),
        "call in if-then branch should be TailCall when if is in tail position"
    );
}

#[test]
fn tail_call_in_if_else_branch() {
    // (fn [x] (if test nil (g x))) - call in else branch is in tail position
    let code = get_fn_body_code("(fn [x] (if test nil (g x)))");

    assert!(
        has_tail_call(&code),
        "call in if-else branch should be TailCall when if is in tail position"
    );
}

#[test]
fn tail_call_in_both_if_branches() {
    // (fn [x] (if test (f x) (g x))) - both branches have tail calls
    let code = get_fn_body_code("(fn [x] (if test (f x) (g x)))");

    // Count TailCall instructions - should be 2 (one for each branch)
    let tail_call_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::TailCall))
        .count();
    assert_eq!(tail_call_count, 2, "both if branches should have TailCall");
}

#[test]
fn no_tail_call_for_if_test() {
    // (fn [x] (if (pred x) 1 2)) - call in test position is NOT in tail position
    let code = get_fn_body_code("(fn [x] (if (pred x) 1 2))");

    assert!(
        has_regular_call(&code),
        "call in if test should be regular Call"
    );
    assert!(!has_tail_call(&code), "if test call should not be TailCall");
}

// =============================================================================
// Tail position in `do`
// =============================================================================

#[test]
fn tail_call_in_do_last() {
    // (fn [x] (do (side-effect) (f x))) - last in do is tail position
    let code = get_fn_body_code("(fn [x] (do (side-effect) (f x)))");

    assert!(has_tail_call(&code), "last call in do should be TailCall");
}

#[test]
fn no_tail_call_in_do_non_last() {
    // (fn [x] (do (f x) 42)) - call is not last, not in tail position
    let code = get_fn_body_code("(fn [x] (do (f x) 42))");

    assert!(
        has_regular_call(&code),
        "non-last call in do should be regular Call"
    );
    assert!(
        !has_tail_call(&code),
        "non-last call in do should not be TailCall"
    );
}

// =============================================================================
// Tail position in `let`
// =============================================================================

#[test]
fn tail_call_in_let_body() {
    // (fn [x] (let [y 1] (f y))) - last in let body is tail position
    let code = get_fn_body_code("(fn [x] (let [y 1] (f y)))");

    assert!(has_tail_call(&code), "call in let body should be TailCall");
}

#[test]
fn no_tail_call_in_let_binding() {
    // (fn [x] (let [y (f x)] y)) - call in binding is NOT in tail position
    let code = get_fn_body_code("(fn [x] (let [y (f x)] y))");

    assert!(
        has_regular_call(&code),
        "call in let binding should be regular Call"
    );
    assert!(
        !has_tail_call(&code),
        "call in let binding should not be TailCall"
    );
}

// =============================================================================
// Nested control flow
// =============================================================================

#[test]
fn tail_call_nested_if_do_let() {
    // Complex nesting: (fn [x] (if test (do 1 (let [y 2] (f y))) nil))
    // The call to f is in tail position through: fn body -> if then -> do last -> let body
    let code = get_fn_body_code("(fn [x] (if test (do 1 (let [y 2] (f y))) nil))");

    assert!(
        has_tail_call(&code),
        "call should be TailCall through nested control flow"
    );
}

#[test]
fn no_tail_call_when_result_used() {
    // (fn [x] (let [result (if test (f x) (g x))] (+ result 1)))
    // The if is in let binding position, so neither branch is in tail position
    let code = get_fn_body_code("(fn [x] (let [result (if test (f x) (g x))] (+ result 1)))");

    assert!(
        has_regular_call(&code),
        "calls in let binding should be regular Call"
    );
    // Note: There might be 2 or more regular calls here
}
