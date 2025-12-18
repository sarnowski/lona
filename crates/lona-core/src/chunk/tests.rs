// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the bytecode chunk implementation.

use alloc::string::String;

use crate::opcode::{Opcode, decode_sbx, encode_abc, encode_abx, encode_asbx, rk_constant};
use crate::span::Span;

use super::{Chunk, Constant};

// =============================================================================
// Chunk Construction Tests
// =============================================================================

#[test]
fn new_chunk_is_empty() {
    let chunk = Chunk::new();
    assert!(chunk.is_empty());
    assert_eq!(chunk.len(), 0);
    assert!(chunk.code().is_empty());
    assert!(chunk.constants().is_empty());
    assert_eq!(chunk.arity(), 0);
    assert_eq!(chunk.max_registers(), 0);
    assert!(chunk.name().is_empty());
}

#[test]
fn chunk_with_name() {
    let chunk = Chunk::with_name(String::from("test_func"));
    assert_eq!(chunk.name(), "test_func");
}

#[test]
fn set_chunk_properties() {
    let mut chunk = Chunk::new();
    chunk.set_name(String::from("my_func"));
    chunk.set_arity(3);
    chunk.set_max_registers(10);

    assert_eq!(chunk.name(), "my_func");
    assert_eq!(chunk.arity(), 3);
    assert_eq!(chunk.max_registers(), 10);
}

// =============================================================================
// Instruction Emission Tests
// =============================================================================

#[test]
fn emit_instructions() {
    let mut chunk = Chunk::new();

    let idx0 = chunk.emit(encode_abc(Opcode::LoadTrue, 0, 0, 0), Span::new(0, 4));
    let idx1 = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(4, 10));

    assert_eq!(idx0, 0);
    assert_eq!(idx1, 1);
    assert_eq!(chunk.len(), 2);
    assert!(!chunk.is_empty());
}

#[test]
fn patch_instruction() {
    let mut chunk = Chunk::new();

    // Emit a placeholder jump
    let jump_idx = chunk.emit(encode_asbx(Opcode::Jump, 0, 0), Span::new(0, 4));

    // Emit some instructions
    let _idx1 = chunk.emit(encode_abc(Opcode::LoadTrue, 0, 0, 0), Span::new(4, 8));
    let _idx2 = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(8, 12));

    // Patch the jump to skip the LoadTrue
    chunk.patch(jump_idx, encode_asbx(Opcode::Jump, 0, 1));

    assert_eq!(decode_sbx(*chunk.code().get(0).unwrap()), 1);
}

// =============================================================================
// Constant Pool Tests
// =============================================================================

#[test]
fn add_and_get_constants() {
    let mut chunk = Chunk::new();

    let idx0 = chunk.add_constant(Constant::Integer(42)).unwrap();
    let idx1 = chunk
        .add_constant(Constant::String(String::from("hello")))
        .unwrap();
    let idx2 = chunk.add_constant(Constant::Nil).unwrap();
    let idx3 = chunk.add_constant(Constant::Bool(true)).unwrap();
    let idx4 = chunk.add_constant(Constant::Float(3.14)).unwrap();

    assert_eq!(idx0, 0);
    assert_eq!(idx1, 1);
    assert_eq!(idx2, 2);
    assert_eq!(idx3, 3);
    assert_eq!(idx4, 4);

    assert_eq!(chunk.get_constant(0), Some(&Constant::Integer(42)));
    assert_eq!(
        chunk.get_constant(1),
        Some(&Constant::String(String::from("hello")))
    );
    assert_eq!(chunk.get_constant(2), Some(&Constant::Nil));
    assert_eq!(chunk.get_constant(3), Some(&Constant::Bool(true)));
    assert_eq!(chunk.get_constant(4), Some(&Constant::Float(3.14)));
    assert_eq!(chunk.get_constant(5), None);
}

#[test]
fn constant_display() {
    extern crate alloc;
    use alloc::format;

    assert_eq!(format!("{}", Constant::Nil), "nil");
    assert_eq!(format!("{}", Constant::Bool(true)), "true");
    assert_eq!(format!("{}", Constant::Bool(false)), "false");
    assert_eq!(format!("{}", Constant::Integer(42)), "42");
    assert_eq!(format!("{}", Constant::Float(3.14)), "3.14");
    assert_eq!(
        format!("{}", Constant::String(String::from("hello"))),
        "\"hello\""
    );
}

// =============================================================================
// Span Tracking Tests
// =============================================================================

#[test]
fn span_tracking() {
    let mut chunk = Chunk::new();

    let _idx0 = chunk.emit(encode_abc(Opcode::LoadTrue, 0, 0, 0), Span::new(0, 4));
    let _idx1 = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(10, 20));

    assert_eq!(chunk.span_at(0), Some(Span::new(0, 4)));
    assert_eq!(chunk.span_at(1), Some(Span::new(10, 20)));
    assert_eq!(chunk.span_at(2), None);

    let spans = chunk.spans();
    assert_eq!(spans.len(), 2);
}

// =============================================================================
// Disassembler Tests
// =============================================================================

#[test]
fn disassemble_empty_chunk() {
    let chunk = Chunk::new();
    let output = chunk.disassemble();

    assert!(output.contains("<anonymous>"));
    assert!(output.contains("arity: 0"));
    assert!(output.contains("max_registers: 0"));
}

#[test]
fn disassemble_named_chunk() {
    let chunk = Chunk::with_name(String::from("main"));
    let output = chunk.disassemble();

    assert!(output.contains("== main =="));
}

#[test]
fn disassemble_load_k() {
    let mut chunk = Chunk::new();
    let k_idx = chunk.add_constant(Constant::Integer(42)).unwrap();
    let _idx = chunk.emit(encode_abx(Opcode::LoadK, 0, k_idx), Span::new(0, 10));

    let output = chunk.disassemble();

    assert!(output.contains("LoadK"));
    assert!(output.contains("R0"));
    assert!(output.contains("K0"));
    assert!(output.contains("; 42"));
}

#[test]
fn disassemble_arithmetic() {
    let mut chunk = Chunk::new();

    // Add R0, R1, K0 (where K0 = 10)
    let k_idx = chunk.add_constant(Constant::Integer(10)).unwrap();
    let rk_const = rk_constant(u8::try_from(k_idx).unwrap()).unwrap();
    let _idx = chunk.emit(encode_abc(Opcode::Add, 0, 1, rk_const), Span::new(0, 10));

    let output = chunk.disassemble();

    assert!(output.contains("Add"));
    assert!(output.contains("R0"));
    assert!(output.contains("R1"));
    assert!(output.contains("K0"));
}

#[test]
fn disassemble_jump() {
    let mut chunk = Chunk::new();

    // Jump +5
    let _idx = chunk.emit(encode_asbx(Opcode::Jump, 0, 5), Span::new(0, 4));

    let output = chunk.disassemble();

    assert!(output.contains("Jump"));
    assert!(output.contains("5"));
    assert!(output.contains("; -> 6")); // offset 0 + 1 + 5 = 6
}

#[test]
fn disassemble_call() {
    let mut chunk = Chunk::new();

    // Call R0, 2, 1 (call function in R0 with 2 args, expect 1 result)
    let _idx = chunk.emit(encode_abc(Opcode::Call, 0, 2, 1), Span::new(0, 10));

    let output = chunk.disassemble();

    assert!(output.contains("Call"));
    assert!(output.contains("R0"));
    assert!(output.contains("2 args"));
    assert!(output.contains("1 results"));
}

#[test]
fn disassemble_return() {
    let mut chunk = Chunk::new();

    // Return R0, 1 (return 1 value starting at R0)
    let _idx = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(0, 6));

    let output = chunk.disassemble();

    assert!(output.contains("Return"));
    assert!(output.contains("return 1 values"));
}

#[test]
fn disassemble_full_program() {
    // Compile (+ 1 2) conceptually
    let mut chunk = Chunk::with_name(String::from("main"));
    chunk.set_max_registers(3);

    let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

    // LoadK R0, K0  ; load 1
    let _idx = chunk.emit(encode_abx(Opcode::LoadK, 0, k0), Span::new(4, 5));
    // LoadK R1, K1  ; load 2
    let _idx = chunk.emit(encode_abx(Opcode::LoadK, 1, k1), Span::new(6, 7));
    // Add R0, R0, R1  ; R0 = 1 + 2
    let _idx = chunk.emit(encode_abc(Opcode::Add, 0, 0, 1), Span::new(1, 8));
    // Return R0, 1
    let _idx = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(0, 9));

    let output = chunk.disassemble();

    // Verify structure
    assert!(output.contains("== main =="));
    assert!(output.contains("max_registers: 3"));
    assert!(output.contains("LoadK"));
    assert!(output.contains("Add"));
    assert!(output.contains("Return"));
    assert!(output.contains("Constants:"));
    assert!(output.contains("K0: 1"));
    assert!(output.contains("K1: 2"));
}

#[test]
fn disassemble_single_instruction() {
    let chunk = Chunk::new();
    let instr = encode_abc(Opcode::Move, 5, 10, 0);

    let output = chunk.disassemble_instruction(0, instr);

    assert!(output.contains("0000"));
    assert!(output.contains("Move"));
    assert!(output.contains("R5"));
    assert!(output.contains("R10"));
}
