// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for literal and symbol compilation.

extern crate alloc;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_a, decode_bx, decode_op};
use lona_core::span::Span;
use lona_core::symbol;

use super::{compile_source, compile_with_interner};
use crate::compiler::{CompileError, compile};
use crate::error::Error;

// =========================================================================
// Literal Compilation Tests
// =========================================================================

#[test]
fn compile_integer() {
    let chunk = compile_source("42");
    let code = chunk.code();

    // Should have: LoadK R0, K0; Return R0, 1
    assert_eq!(code.len(), 2);

    // LoadK instruction
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    assert_eq!(decode_a(instr0), 0);
    let k_idx = decode_bx(instr0);
    assert_eq!(chunk.get_constant(k_idx), Some(&Constant::Integer(42)));

    // Return instruction
    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::Return));
    assert_eq!(decode_a(instr1), 0);
}

#[test]
fn compile_float() {
    let chunk = compile_source("3.14");
    let code = chunk.code();

    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let k_idx = decode_bx(instr0);
    assert_eq!(chunk.get_constant(k_idx), Some(&Constant::Float(3.14)));
}

#[test]
fn compile_true() {
    let chunk = compile_source("true");
    let code = chunk.code();

    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadTrue));
    assert_eq!(decode_a(instr0), 0);
}

#[test]
fn compile_false() {
    let chunk = compile_source("false");
    let code = chunk.code();

    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadFalse));
}

#[test]
fn compile_nil() {
    let chunk = compile_source("nil");
    let code = chunk.code();

    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadNil));
}

// =========================================================================
// Symbol Compilation Tests
// =========================================================================

#[test]
fn compile_symbol_global_lookup() {
    let (chunk, interner) = compile_with_interner("foo");
    let code = chunk.code();

    // GetGlobal R0, K0 (where K0 is sym#foo)
    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::GetGlobal));
    assert_eq!(decode_a(instr0), 0);

    let k_idx = decode_bx(instr0);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(k_idx) {
        assert_eq!(interner.resolve(*sym_id), "foo");
    } else {
        panic!("expected Symbol constant");
    }
}

// =========================================================================
// String Compilation Tests
// =========================================================================

#[test]
fn compile_string() {
    let chunk = compile_source("\"hello\"");
    let code = chunk.code();

    // LoadK R0, K0; Return R0, 1
    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    assert_eq!(decode_a(instr0), 0);
    let const_idx = decode_bx(instr0);
    assert_eq!(
        chunk.get_constant(const_idx),
        Some(&Constant::String(alloc::string::String::from("hello")))
    );
}

#[test]
fn compile_empty_string() {
    let chunk = compile_source("\"\"");
    let code = chunk.code();

    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr0);
    assert_eq!(
        chunk.get_constant(const_idx),
        Some(&Constant::String(alloc::string::String::from("")))
    );
}

#[test]
fn compile_string_with_escapes() {
    let chunk = compile_source("\"hello\\nworld\"");
    let code = chunk.code();

    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    let const_idx = decode_bx(instr0);
    assert_eq!(
        chunk.get_constant(const_idx),
        Some(&Constant::String(alloc::string::String::from(
            "hello\nworld"
        )))
    );
}

// =========================================================================
// Not Implemented Tests
// =========================================================================

#[test]
fn compile_keyword_not_implemented() {
    let mut interner = symbol::Interner::new();
    let result = compile(":keyword", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::NotImplemented { feature, .. })) = result {
        assert_eq!(feature, "keyword literals");
    } else {
        panic!("expected NotImplemented error");
    }
}

#[test]
fn compile_vector_not_implemented() {
    let mut interner = symbol::Interner::new();
    let result = compile("[1 2 3]", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::NotImplemented { feature, .. })) = result {
        assert_eq!(feature, "vector literals");
    } else {
        panic!("expected NotImplemented error");
    }
}

// =========================================================================
// Chunk Metadata Tests
// =========================================================================

#[test]
fn compile_tracks_max_registers() {
    let chunk = compile_source("(print (+ 1 2))");
    // Uses R0 for print, R1 for add result
    assert!(chunk.max_registers() >= 2);
}

// =========================================================================
// Disassembly Tests
// =========================================================================

#[test]
fn disassemble_print_add() {
    let chunk = compile_source("(print (+ 1 2))");
    let disasm = chunk.disassemble();

    // Verify key parts are present in disassembly
    assert!(disasm.contains("GetGlobal"));
    assert!(disasm.contains("Add"));
    assert!(disasm.contains("Call"));
    assert!(disasm.contains("Return"));
}

// =========================================================================
// Multiple Expression Tests
// =========================================================================

#[test]
fn compile_multiple_expressions() {
    let chunk = compile_source("1 2 3");
    let code = chunk.code();

    // Each expression resets registers, so:
    // LoadK R0, K0 (1)
    // LoadK R0, K1 (2)
    // LoadK R0, K2 (3)
    // Return R0, 1
    assert_eq!(code.len(), 4);

    // Last instruction is Return with R0
    let last = *code.get(3_usize).unwrap();
    assert_eq!(decode_op(last), Some(Opcode::Return));
    assert_eq!(decode_a(last), 0);
}

#[test]
fn compile_empty_program() {
    let chunk = compile_source("");
    let code = chunk.code();

    // Empty program: LoadNil R0; Return R0, 1
    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadNil));

    // Verify max_registers is correctly set for frame allocation
    assert_eq!(chunk.max_registers(), 1);
}

// =========================================================================
// CompileError Tests
// =========================================================================

#[test]
fn compile_error_display() {
    let err = CompileError::Compile(Error::EmptyCall {
        span: Span::new(0_usize, 2_usize),
    });
    let msg = alloc::format!("{}", err);
    assert!(msg.contains("empty list"));
}

#[test]
fn compile_error_from_parse() {
    let mut interner = symbol::Interner::new();
    let result = compile("(unclosed", &mut interner);
    assert!(matches!(result, Err(CompileError::Parse(_))));
}
