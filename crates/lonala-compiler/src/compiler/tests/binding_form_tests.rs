// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for binding special forms: def and let.

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_bx, decode_op};
use lona_core::symbol;

use super::{TEST_SOURCE_ID, compile_source, compile_with_interner};
use crate::compiler::CompileError;
use crate::compiler::compile;
use crate::error::{Error, Kind as ErrorKind};

// =========================================================================
// Special Form: def
// =========================================================================

#[test]
fn compile_def_simple() {
    let (chunk, interner) = compile_with_interner("(def x 42)");
    let code = chunk.code();

    // LoadK R0, K0 (42)
    // SetGlobal R0, K1 (x)
    // LoadK R0, K1 (x) - return the symbol
    // Return R0, 1
    assert_eq!(code.len(), 4);

    // First instruction loads the value
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let value_const = decode_bx(instr0);
    assert_eq!(
        chunk.get_constant(value_const),
        Some(&Constant::Integer(42))
    );

    // Second instruction is SetGlobal
    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::SetGlobal));
    let sym_const = decode_bx(instr1);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(sym_const) {
        assert_eq!(interner.resolve(*sym_id), "x");
    } else {
        panic!("expected Symbol constant for def name");
    }

    // Third instruction loads the symbol to return it
    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::LoadK));
}

#[test]
fn compile_def_with_expression() {
    let (chunk, interner) = compile_with_interner("(def y (+ 1 2))");
    let code = chunk.code();

    // Add R0, K0, K1 (1 + 2)
    // SetGlobal R0, K2 (y)
    // LoadK R0, K2 (y) - return the symbol
    // Return R0, 1
    assert_eq!(code.len(), 4);

    // First instruction should be Add
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Add));

    // Check symbol name
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(2) {
        assert_eq!(interner.resolve(*sym_id), "y");
    } else {
        panic!("expected Symbol constant at K2");
    }
}

#[test]
fn compile_def_then_use() {
    let (chunk, interner) = compile_with_interner("(do (def x 10) x)");
    let code = chunk.code();

    // LoadK R0, K0 (10)
    // SetGlobal R0, K1 (x)
    // LoadK R0, K1 (x) - return from def (discarded by do)
    // GetGlobal R0, K1 (x) - lookup x
    // Return R0, 1
    assert_eq!(code.len(), 5);

    // Last instruction before Return should be GetGlobal
    let get_global = *code.get(3_usize).unwrap();
    assert_eq!(decode_op(get_global), Some(Opcode::GetGlobal));

    let sym_const = decode_bx(get_global);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(sym_const) {
        assert_eq!(interner.resolve(*sym_id), "x");
    } else {
        panic!("expected Symbol constant");
    }
}

#[test]
fn compile_def_multiple() {
    let chunk = compile_source("(do (def a 1) (def b 2) (+ a b))");
    let code = chunk.code();

    // Should have SetGlobal for both a and b, then Add
    let mut set_global_count = 0_usize;
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::SetGlobal) {
            set_global_count = set_global_count.saturating_add(1);
        }
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
        }
    }
    assert_eq!(set_global_count, 2);
    assert!(has_add);
}

#[test]
fn compile_def_invalid_no_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def)", TEST_SOURCE_ID, &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_one_arg() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def x)", TEST_SOURCE_ID, &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_three_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def x 1 2)", TEST_SOURCE_ID, &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_non_symbol_name() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def 42 1)", TEST_SOURCE_ID, &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
        assert_eq!(form, "def");
        assert!(message.contains("symbol"));
    } else {
        panic!("expected InvalidSpecialForm error for def with non-symbol name");
    }
}

// =========================================================================
// Special Form: let
// =========================================================================

#[test]
fn compile_let_single_binding() {
    let chunk = compile_source("(let [x 42] x)");
    let code = chunk.code();

    // Should have instructions for: load 42, move to binding, move from binding, return
    assert!(code.len() >= 2);

    // Should use Move opcode for local variable access
    let mut has_move = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Move) {
            has_move = true;
            break;
        }
    }
    assert!(has_move);
}

#[test]
fn compile_let_multiple_bindings() {
    let chunk = compile_source("(let [x 1 y 2] (+ x y))");
    let code = chunk.code();

    // Should have Add instruction
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
            break;
        }
    }
    assert!(has_add);
}

#[test]
fn compile_let_forward_reference() {
    let chunk = compile_source("(let [x 1 y (+ x 1)] y)");
    let code = chunk.code();

    // Should have Add instruction for (+ x 1)
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
            break;
        }
    }
    assert!(has_add);
}

#[test]
fn compile_let_nested() {
    let chunk = compile_source("(let [x 1] (let [y 2] (+ x y)))");
    let code = chunk.code();

    // Should have Add instruction
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
            break;
        }
    }
    assert!(has_add);
}

#[test]
fn compile_let_empty_body() {
    let chunk = compile_source("(let [x 1])");
    let code = chunk.code();

    // Empty body should produce LoadNil
    let mut has_load_nil = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::LoadNil) {
            has_load_nil = true;
            break;
        }
    }
    assert!(has_load_nil);
}

#[test]
fn compile_let_invalid_no_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let)", TEST_SOURCE_ID, &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "let");
    } else {
        panic!("expected InvalidSpecialForm error for let");
    }
}

#[test]
fn compile_let_invalid_non_vector_bindings() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let (x 1) x)", TEST_SOURCE_ID, &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
        assert_eq!(form, "let");
        assert!(message.contains("vector"));
    } else {
        panic!("expected InvalidSpecialForm error for let with non-vector bindings");
    }
}

#[test]
fn compile_let_invalid_odd_bindings() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let [x 1 y] x)", TEST_SOURCE_ID, &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
        assert_eq!(form, "let");
        assert!(message.contains("pairs"));
    } else {
        panic!("expected InvalidSpecialForm error for let with odd bindings");
    }
}

#[test]
fn compile_let_invalid_non_symbol_binding_name() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let [42 1] 0)", TEST_SOURCE_ID, &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
        assert_eq!(form, "let");
        assert!(message.contains("symbol"));
    } else {
        panic!("expected InvalidSpecialForm error for let with non-symbol binding name");
    }
}
