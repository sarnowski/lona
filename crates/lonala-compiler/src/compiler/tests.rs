// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Lonala bytecode compiler.

extern crate alloc;

use lona_core::symbol;

use crate::chunk::Constant;
use crate::compiler::{CompileError, compile};
use crate::error::Error;
use crate::opcode::{Opcode, decode_a, decode_b, decode_bx, decode_c, decode_op};

use lonala_parser::Span;

/// Helper to compile source and return the chunk.
fn compile_source(source: &str) -> crate::chunk::Chunk {
    let mut interner = symbol::Interner::new();
    compile(source, &mut interner).expect("compilation should succeed")
}

/// Helper to compile and return chunk + interner for symbol checks.
fn compile_with_interner(source: &str) -> (crate::chunk::Chunk, symbol::Interner) {
    let mut interner = symbol::Interner::new();
    let chunk = compile(source, &mut interner).expect("compilation should succeed");
    (chunk, interner)
}

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
    assert_eq!(decode_b(instr1), 1);
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
    let mut interner = symbol::Interner::new();
    let result = compile("()", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::EmptyCall { .. })) = result {
        // Expected
    } else {
        panic!("expected EmptyCall error");
    }
}

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
// Disassembly Tests (for debugging)
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
