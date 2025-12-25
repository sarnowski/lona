// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for function call compilation.

extern crate alloc;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_a, decode_b, decode_c, decode_op};
use lona_core::symbol;

use super::{TEST_SOURCE_ID, compile_with_interner};
use crate::compiler::{CompileError, compile};
use crate::error::{Error, Kind as ErrorKind};

// =========================================================================
// Function Call Tests
// =========================================================================

#[test]
fn compile_function_call() {
    let (chunk, interner) = compile_with_interner("(print 42)");
    let code = chunk.code();

    // GetGlobal R0, K0 (print)
    // LoadK R1, K1 (42)
    // Call R0, 1, 1
    // Return R0, 1
    assert_eq!(code.len(), 4);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::GetGlobal));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::LoadK));

    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::Call));
    assert_eq!(decode_a(instr2), 0); // base register
    assert_eq!(decode_b(instr2), 1); // 1 argument
    assert_eq!(decode_c(instr2), 1); // 1 result

    // Verify symbol constant
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(0) {
        assert_eq!(interner.resolve(*sym_id), "print");
    } else {
        panic!("expected Symbol constant");
    }
}

#[test]
fn compile_print_addition() {
    let (chunk, interner) = compile_with_interner("(print (+ 1 2))");
    let code = chunk.code();

    // GetGlobal R0, K0 (print)
    // Add R1, K1, K2 (1 + 2)
    // Call R0, 1, 1
    // Return R0, 1
    assert_eq!(code.len(), 4);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::GetGlobal));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::Add));
    assert_eq!(decode_a(instr1), 1); // result in R1

    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::Call));

    // Verify print symbol
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(0) {
        assert_eq!(interner.resolve(*sym_id), "print");
    } else {
        panic!("expected Symbol constant");
    }
}

#[test]
fn compile_print_string() {
    let (chunk, interner) = compile_with_interner("(print \"hello\")");
    let code = chunk.code();

    // GetGlobal R0, K0 (print)
    // LoadK R1, K1 ("hello")
    // Call R0, 1, 1
    // Return R0, 1
    assert_eq!(code.len(), 4);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::GetGlobal));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::LoadK));
    assert_eq!(decode_a(instr1), 1); // string in R1

    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::Call));

    // Verify constants
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(0) {
        assert_eq!(interner.resolve(*sym_id), "print");
    } else {
        panic!("expected Symbol constant at K0");
    }
    assert_eq!(
        chunk.get_constant(1),
        Some(&Constant::String(alloc::string::String::from("hello")))
    );
}

// =========================================================================
// Error Tests
// =========================================================================

#[test]
fn compile_empty_call_error() {
    let interner = symbol::Interner::new();
    let result = compile("()", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::EmptyCall,
        ..
    })) = result
    {
        // Expected
    } else {
        panic!("expected EmptyCall error");
    }
}
