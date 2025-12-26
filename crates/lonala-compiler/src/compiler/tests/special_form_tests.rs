// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for control flow special forms: do and if.
//!
//! Other special form tests are split into:
//! - `binding_form_tests` - def and let
//! - `quote_form_tests` - quote and syntax-quote

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_bx, decode_op};
use lona_core::symbol;

use super::{TEST_SOURCE_ID, compile_source, compile_with_interner};
use crate::compiler::CompileError;
use crate::compiler::compile;
use crate::error::{Error, Kind as ErrorKind};

// =========================================================================
// Special Form: do
// =========================================================================

#[test]
fn compile_do_empty() {
    let chunk = compile_source("(do)");
    let code = chunk.code();

    // Empty do: LoadNil R0; Return R0, 1
    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadNil));
}

#[test]
fn compile_do_single() {
    let chunk = compile_source("(do 42)");
    let code = chunk.code();

    // (do 42): LoadK R0, K0; Return R0, 1
    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr0);
    assert_eq!(chunk.get_constant(const_idx), Some(&Constant::Integer(42)));
}

#[test]
fn compile_do_multiple() {
    let chunk = compile_source("(do 1 2 3)");
    let code = chunk.code();

    // Should compile all three, but only last matters for return
    // LoadK R0, K0 (1) - discarded
    // LoadK R0, K1 (2) - discarded
    // LoadK R0, K2 (3) - returned
    // Return R0, 1
    assert_eq!(code.len(), 4);

    // Last LoadK should load 3
    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr2);
    assert_eq!(chunk.get_constant(const_idx), Some(&Constant::Integer(3)));
}

#[test]
fn compile_do_with_side_effects() {
    let (chunk, interner) = compile_with_interner("(do (print 1) (+ 2 3))");
    let code = chunk.code();

    // GetGlobal R0, K0 (user/print)
    // LoadK R1, K1 (1)
    // Call R0, 1, 1
    // Add R0, K2, K3 (2 + 3)
    // Return R0, 1
    assert_eq!(code.len(), 5);

    // Verify print symbol (namespace-qualified)
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(0) {
        assert_eq!(interner.resolve(*sym_id), "user/print");
    } else {
        panic!("expected Symbol constant");
    }

    // Last instruction before Return should be Add
    let add_instr = *code.get(3_usize).unwrap();
    assert_eq!(decode_op(add_instr), Some(Opcode::Add));
}

// =========================================================================
// Special Form: if
// =========================================================================

#[test]
fn compile_if_true_branch() {
    let chunk = compile_source("(if true 1 2)");
    let code = chunk.code();

    // LoadTrue R0 (test)
    // JumpIfNot R0, +offset_to_else
    // LoadK R1, K0 (1)
    // Move R0, R1
    // Jump +offset_to_end
    // LoadK R1, K1 (2)
    // Move R0, R1
    // Return R0, 1
    assert_eq!(code.len(), 8);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadTrue));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::JumpIfNot));

    // Then branch loads 1 and moves to dest
    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::LoadK));
    let then_const = decode_bx(instr2);
    assert_eq!(chunk.get_constant(then_const), Some(&Constant::Integer(1)));

    let instr3 = *code.get(3_usize).unwrap();
    assert_eq!(decode_op(instr3), Some(Opcode::Move));

    let instr4 = *code.get(4_usize).unwrap();
    assert_eq!(decode_op(instr4), Some(Opcode::Jump));

    // Else branch loads 2 and moves to dest
    let instr5 = *code.get(5_usize).unwrap();
    assert_eq!(decode_op(instr5), Some(Opcode::LoadK));
    let else_const = decode_bx(instr5);
    assert_eq!(chunk.get_constant(else_const), Some(&Constant::Integer(2)));

    let instr6 = *code.get(6_usize).unwrap();
    assert_eq!(decode_op(instr6), Some(Opcode::Move));
}

#[test]
fn compile_if_no_else() {
    let chunk = compile_source("(if false 1)");
    let code = chunk.code();

    // LoadFalse R0 (test)
    // JumpIfNot R0, +offset
    // LoadK R1, K0 (1)
    // Move R0, R1
    // Jump +offset
    // LoadNil R0 (implicit else)
    // Return R0, 1
    assert_eq!(code.len(), 7);

    // Else branch should be LoadNil
    let else_instr = *code.get(5_usize).unwrap();
    assert_eq!(decode_op(else_instr), Some(Opcode::LoadNil));
}

#[test]
fn compile_if_nested() {
    let chunk = compile_source("(if true (if false 1 2) 3)");
    let code = chunk.code();

    // This will have nested if structure
    // The outer then branch contains another if
    assert!(code.len() > 6);

    // First instruction is LoadTrue for outer test
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadTrue));
}

#[test]
fn compile_if_with_expressions() {
    let chunk = compile_source("(if (> 5 3) (+ 1 2) (- 10 5))");
    let code = chunk.code();

    // Test is (> 5 3)
    // Then is (+ 1 2)
    // Else is (- 10 5)

    // First instruction should be Gt for the test
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Gt));
}

#[test]
fn compile_if_invalid_no_args() {
    let interner = symbol::Interner::new();
    let result = compile("(if)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "if");
    } else {
        panic!("expected InvalidSpecialForm error for if");
    }
}

#[test]
fn compile_if_invalid_one_arg() {
    let interner = symbol::Interner::new();
    let result = compile("(if true)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "if");
    } else {
        panic!("expected InvalidSpecialForm error for if");
    }
}

#[test]
fn compile_if_invalid_four_args() {
    let interner = symbol::Interner::new();
    let result = compile("(if true 1 2 3)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "if");
    } else {
        panic!("expected InvalidSpecialForm error for if");
    }
}
