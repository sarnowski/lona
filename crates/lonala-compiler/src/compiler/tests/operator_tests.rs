// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for arithmetic, comparison, and unary operators.

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_a, decode_op};

use super::compile_source;

// =========================================================================
// Arithmetic Compilation Tests
// =========================================================================

#[test]
fn compile_addition() {
    let chunk = compile_source("(+ 1 2)");
    let code = chunk.code();

    // Add R0, K0, K1; Return R0, 1
    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Add));
    assert_eq!(decode_a(instr0), 0);

    // Verify constants
    assert_eq!(chunk.get_constant(0), Some(&Constant::Integer(1)));
    assert_eq!(chunk.get_constant(1), Some(&Constant::Integer(2)));
}

#[test]
fn compile_subtraction() {
    let chunk = compile_source("(- 10 3)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Sub));
}

#[test]
fn compile_multiplication() {
    let chunk = compile_source("(* 4 5)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Mul));
}

#[test]
fn compile_division() {
    let chunk = compile_source("(/ 20 4)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Div));
}

#[test]
fn compile_modulo() {
    let chunk = compile_source("(mod 10 3)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Mod));
}

#[test]
fn compile_nested_arithmetic() {
    let chunk = compile_source("(+ (* 2 3) 4)");
    let code = chunk.code();

    // Mul R0, K0, K1 (2 * 3)
    // Add R0, R0, K2 (result + 4)
    // Return R0, 1
    assert_eq!(code.len(), 3);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Mul));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::Add));

    // Verify constants
    assert_eq!(chunk.get_constant(0), Some(&Constant::Integer(2)));
    assert_eq!(chunk.get_constant(1), Some(&Constant::Integer(3)));
    assert_eq!(chunk.get_constant(2), Some(&Constant::Integer(4)));
}

// =========================================================================
// Comparison Operators Tests
// =========================================================================

#[test]
fn compile_equality() {
    let chunk = compile_source("(= 1 1)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Eq));
}

#[test]
fn compile_less_than() {
    let chunk = compile_source("(< 1 2)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Lt));
}

#[test]
fn compile_greater_than() {
    let chunk = compile_source("(> 2 1)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Gt));
}

#[test]
fn compile_less_than_or_equal() {
    let chunk = compile_source("(<= 2 2)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Le));
}

#[test]
fn compile_greater_than_or_equal() {
    let chunk = compile_source("(>= 3 2)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Ge));
}

// =========================================================================
// Unary Operators Tests
// =========================================================================

#[test]
fn compile_unary_negation() {
    let chunk = compile_source("(- 5)");
    let code = chunk.code();

    // LoadK R0, K0 (5)
    // Neg R0, R0
    // Return R0, 1
    assert_eq!(code.len(), 3);

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::Neg));
}

#[test]
fn compile_unary_negation_expression() {
    let chunk = compile_source("(- (+ 1 2))");
    let code = chunk.code();

    // Add R0, K0, K1 (1 + 2)
    // Neg R0, R0
    // Return R0, 1
    assert_eq!(code.len(), 3);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Add));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::Neg));
}

#[test]
fn compile_not_operator() {
    let chunk = compile_source("(not true)");
    let code = chunk.code();

    // LoadTrue R0
    // Not R0, R0
    // Return R0, 1
    assert_eq!(code.len(), 3);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadTrue));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::Not));
}

#[test]
fn compile_not_false() {
    let chunk = compile_source("(not false)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadFalse));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::Not));
}

#[test]
fn compile_not_with_comparison() {
    let chunk = compile_source("(not (= 1 2))");
    let code = chunk.code();

    // Eq R0, K0, K1 (1 = 2)
    // Not R0, R0
    // Return R0, 1
    assert_eq!(code.len(), 3);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Eq));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::Not));
}
