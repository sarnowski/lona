// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Lonala bytecode compiler.

extern crate alloc;

use lona_core::chunk::{Chunk, Constant};
use lona_core::opcode::{Opcode, decode_a, decode_b, decode_bx, decode_c, decode_op};
use lona_core::span::Span;
use lona_core::symbol;

use crate::compiler::{CompileError, MacroRegistry, compile};
use crate::error::Error;

/// Helper to compile source and return the chunk.
fn compile_source(source: &str) -> Chunk {
    let mut interner = symbol::Interner::new();
    compile(source, &mut interner).expect("compilation should succeed")
}

/// Helper to compile and return chunk + interner for symbol checks.
fn compile_with_interner(source: &str) -> (Chunk, symbol::Interner) {
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
    assert_eq!(decode_a(instr0), 0);
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

    // GetGlobal R0, K0 (print)
    // LoadK R1, K1 (1)
    // Call R0, 1, 1
    // Add R0, K2, K3 (2 + 3)
    // Return R0, 1
    assert_eq!(code.len(), 5);

    // Verify print symbol
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(0) {
        assert_eq!(interner.resolve(*sym_id), "print");
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
    let mut interner = symbol::Interner::new();
    let result = compile("(if)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "if");
    } else {
        panic!("expected InvalidSpecialForm error for if");
    }
}

#[test]
fn compile_if_invalid_one_arg() {
    let mut interner = symbol::Interner::new();
    let result = compile("(if true)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "if");
    } else {
        panic!("expected InvalidSpecialForm error for if");
    }
}

#[test]
fn compile_if_invalid_four_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(if true 1 2 3)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "if");
    } else {
        panic!("expected InvalidSpecialForm error for if");
    }
}

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
    let result = compile("(def)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_one_arg() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def x)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_three_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def x 1 2)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_non_symbol_name() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def 42 1)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
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
    let result = compile("(let)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "let");
    } else {
        panic!("expected InvalidSpecialForm error for let");
    }
}

#[test]
fn compile_let_invalid_non_vector_bindings() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let (x 1) x)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "let");
        assert!(message.contains("vector"));
    } else {
        panic!("expected InvalidSpecialForm error for let with non-vector bindings");
    }
}

#[test]
fn compile_let_invalid_odd_bindings() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let [x 1 y] x)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "let");
        assert!(message.contains("pairs"));
    } else {
        panic!("expected InvalidSpecialForm error for let with odd bindings");
    }
}

#[test]
fn compile_let_invalid_non_symbol_binding_name() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let [42 1] 0)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "let");
        assert!(message.contains("symbol"));
    } else {
        panic!("expected InvalidSpecialForm error for let with non-symbol binding name");
    }
}

// =========================================================================
// Special Form: quote
// =========================================================================

#[test]
fn compile_quote_symbol() {
    let (chunk, interner) = compile_with_interner("(quote foo)");
    let code = chunk.code();

    // Should have LoadK instruction
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    // Verify symbol constant
    let const_idx = decode_bx(instr0);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) {
        assert_eq!(interner.resolve(*sym_id), "foo");
    } else {
        panic!("expected Symbol constant for quoted symbol");
    }
}

#[test]
fn compile_quote_integer() {
    let chunk = compile_source("(quote 42)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    let const_idx = decode_bx(instr0);
    assert_eq!(chunk.get_constant(const_idx), Some(&Constant::Integer(42)));
}

#[test]
fn compile_quote_list() {
    let chunk = compile_source("(quote (1 2 3))");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    let const_idx = decode_bx(instr0);
    if let Some(Constant::List(elements)) = chunk.get_constant(const_idx) {
        assert_eq!(elements.len(), 3);
        assert_eq!(elements.get(0), Some(&Constant::Integer(1)));
        assert_eq!(elements.get(1), Some(&Constant::Integer(2)));
        assert_eq!(elements.get(2), Some(&Constant::Integer(3)));
    } else {
        panic!("expected List constant for quoted list");
    }
}

#[test]
fn compile_quote_vector() {
    let chunk = compile_source("(quote [1 2 3])");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    let const_idx = decode_bx(instr0);
    if let Some(Constant::Vector(elements)) = chunk.get_constant(const_idx) {
        assert_eq!(elements.len(), 3);
        assert_eq!(elements.get(0), Some(&Constant::Integer(1)));
        assert_eq!(elements.get(1), Some(&Constant::Integer(2)));
        assert_eq!(elements.get(2), Some(&Constant::Integer(3)));
    } else {
        panic!("expected Vector constant for quoted vector");
    }
}

#[test]
fn compile_quote_nested_list() {
    let chunk = compile_source("(quote (a (b c) d))");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    let const_idx = decode_bx(instr0);
    if let Some(Constant::List(elements)) = chunk.get_constant(const_idx) {
        assert_eq!(elements.len(), 3);
        // First element is symbol 'a'
        assert!(matches!(elements.get(0), Some(Constant::Symbol(_))));
        // Second element is list (b c)
        if let Some(Constant::List(inner)) = elements.get(1) {
            assert_eq!(inner.len(), 2);
        } else {
            panic!("expected nested list");
        }
        // Third element is symbol 'd'
        assert!(matches!(elements.get(2), Some(Constant::Symbol(_))));
    } else {
        panic!("expected List constant for quoted nested list");
    }
}

#[test]
fn compile_quote_prevents_evaluation() {
    let (chunk, interner) = compile_with_interner("(quote (+ 1 2))");
    let code = chunk.code();

    // Should NOT have Add instruction - quote prevents evaluation
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
            break;
        }
    }
    assert!(!has_add, "quote should prevent evaluation");

    // Should have a List constant with symbol '+' and integers 1, 2
    let instr0 = *code.get(0_usize).unwrap();
    let const_idx = decode_bx(instr0);
    if let Some(Constant::List(elements)) = chunk.get_constant(const_idx) {
        assert_eq!(elements.len(), 3);
        // First element should be symbol '+'
        if let Some(Constant::Symbol(sym_id)) = elements.get(0) {
            assert_eq!(interner.resolve(*sym_id), "+");
        } else {
            panic!("expected symbol '+' in quoted list");
        }
    } else {
        panic!("expected List constant for quoted expression");
    }
}

#[test]
fn compile_quote_invalid_no_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(quote)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "quote");
    } else {
        panic!("expected InvalidSpecialForm error for quote");
    }
}

#[test]
fn compile_quote_invalid_two_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(quote a b)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "quote");
    } else {
        panic!("expected InvalidSpecialForm error for quote with two args");
    }
}

// =========================================================================
// Special Form: syntax-quote (quasiquote)
// =========================================================================

#[test]
fn compile_syntax_quote_literal() {
    // Literals are quoted
    let chunk = compile_source("`42");
    let code = chunk.code();

    // Should generate (quote 42)
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr0);
    assert_eq!(chunk.get_constant(const_idx), Some(&Constant::Integer(42)));
}

#[test]
fn compile_syntax_quote_symbol() {
    // Symbols are quoted
    let (chunk, interner) = compile_with_interner("`foo");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr0);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) {
        assert_eq!(interner.resolve(*sym_id), "foo");
    } else {
        panic!("expected Symbol constant for syntax-quoted symbol");
    }
}

#[test]
fn compile_syntax_quote_empty_list() {
    // Empty list stays empty
    let chunk = compile_source("`()");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr0);
    if let Some(Constant::List(elements)) = chunk.get_constant(const_idx) {
        assert!(elements.is_empty());
    } else {
        panic!("expected empty List constant");
    }
}

#[test]
fn compile_syntax_quote_simple_list() {
    // `(a b) expands to (list 'a 'b)
    let chunk = compile_source("`(a b)");
    let code = chunk.code();

    // Should have Call instruction for (list ...)
    let mut has_call = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Call) {
            has_call = true;
            break;
        }
    }
    assert!(
        has_call,
        "syntax-quote should generate function calls to list"
    );
}

#[test]
fn compile_unquote_in_syntax_quote() {
    // `(a ~x) with unquote should evaluate x
    let chunk = compile_source("(let [x 1] `(a ~x))");
    let code = chunk.code();

    // Should have Call instruction for list
    let mut has_call = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Call) {
            has_call = true;
            break;
        }
    }
    assert!(
        has_call,
        "unquote should be compiled in syntax-quote context"
    );
}

#[test]
fn compile_unquote_splicing_in_syntax_quote() {
    // `(a ~@xs b) should use concat
    let chunk = compile_source("(let [xs (vector 1 2)] `(a ~@xs b))");
    let code = chunk.code();

    // Should have multiple Call instructions (for concat, list)
    let mut call_count = 0_usize;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Call) {
            call_count = call_count.saturating_add(1);
        }
    }
    // At least 2 calls: vector constructor and the concat/list calls
    assert!(
        call_count >= 2,
        "unquote-splicing should generate multiple function calls"
    );
}

#[test]
fn compile_syntax_quote_vector() {
    // `[a b] should produce a vector
    let chunk = compile_source("`[a b]");
    let code = chunk.code();

    // Should have Call instruction for vec
    let mut has_call = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Call) {
            has_call = true;
            break;
        }
    }
    assert!(has_call, "syntax-quote vector should generate calls to vec");
}

#[test]
fn compile_unquote_outside_syntax_quote_error() {
    let mut interner = symbol::Interner::new();
    let result = compile("~x", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "unquote");
        assert!(message.contains("not inside syntax-quote"));
    } else {
        panic!("expected InvalidSpecialForm error for unquote outside syntax-quote");
    }
}

#[test]
fn compile_unquote_splicing_outside_syntax_quote_error() {
    let mut interner = symbol::Interner::new();
    let result = compile("~@x", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "unquote-splicing");
        assert!(message.contains("not inside syntax-quote"));
    } else {
        panic!("expected InvalidSpecialForm error for unquote-splicing outside syntax-quote");
    }
}

#[test]
fn compile_syntax_quote_invalid_no_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(syntax-quote)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "syntax-quote");
    } else {
        panic!("expected InvalidSpecialForm error for syntax-quote");
    }
}

#[test]
fn compile_syntax_quote_invalid_two_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(syntax-quote a b)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "syntax-quote");
    } else {
        panic!("expected InvalidSpecialForm error for syntax-quote with two args");
    }
}

// =========================================================================
// Special Form: defmacro
// =========================================================================

#[test]
fn defmacro_basic_definition() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro identity [x] x)").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_ok());
}

#[test]
fn defmacro_stores_in_registry() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro my-macro [x] x)").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    assert!(compiler.is_macro_by_name("my-macro"));

    let macro_def = compiler.get_macro_by_name("my-macro").unwrap();
    assert_eq!(macro_def.arity(), 1);
    assert_eq!(macro_def.name(), "my-macro");
}

#[test]
fn defmacro_requires_name() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    // (defmacro [x] x) - first arg is vector, not symbol
    let exprs = lonala_parser::parse("(defmacro [x] x)").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_err());

    if let Err(Error::InvalidSpecialForm { form, message, .. }) = result {
        assert_eq!(form, "defmacro");
        assert!(message.contains("symbol"));
    } else {
        panic!("expected InvalidSpecialForm error for defmacro with non-symbol name");
    }
}

#[test]
fn defmacro_requires_params_vector() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    // (defmacro foo x x) - params is symbol, not vector
    let exprs = lonala_parser::parse("(defmacro foo x x)").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_err());

    if let Err(Error::InvalidSpecialForm { form, message, .. }) = result {
        assert_eq!(form, "defmacro");
        assert!(message.contains("vector"));
    } else {
        panic!("expected InvalidSpecialForm error for defmacro with non-vector params");
    }
}

#[test]
fn defmacro_requires_body() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro foo [x])").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_err());

    if let Err(Error::InvalidSpecialForm { form, message, .. }) = result {
        assert_eq!(form, "defmacro");
        assert!(message.contains("empty"));
    } else {
        panic!("expected InvalidSpecialForm error for defmacro without body");
    }
}

#[test]
fn defmacro_multiple_params() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro swap [a b] b)").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    let macro_def = compiler.get_macro_by_name("swap").unwrap();
    assert_eq!(macro_def.arity(), 2);
}

#[test]
fn defmacro_zero_params() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro always-nil [] nil)").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    let macro_def = compiler.get_macro_by_name("always-nil").unwrap();
    assert_eq!(macro_def.arity(), 0);
}

#[test]
fn defmacro_with_quasiquote() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    // unless macro that uses quasiquote
    let exprs =
        lonala_parser::parse("(defmacro unless [test body] `(if (not ~test) ~body nil))").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_ok());

    assert!(compiler.is_macro_by_name("unless"));
}

#[test]
fn defmacro_multiple_body_expressions() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    // Macro with multiple body expressions (last is return value)
    let exprs =
        lonala_parser::parse("(defmacro with-logging [expr] (print \"expanding\") expr)").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_ok());
}

#[test]
fn defmacro_returns_symbol() {
    let mut interner = symbol::Interner::new();
    let chunk = compile("(defmacro foo [x] x)", &mut interner).unwrap();
    let code = chunk.code();

    // The defmacro expression should return the symbol 'foo.
    // Verify the bytecode loads the symbol constant before returning.
    // (VM execution test is in lona-kernel integration tests)
    let last_loadk = code
        .iter()
        .rev()
        .skip(1) // Skip Return
        .find(|&&instr| decode_op(instr) == Some(Opcode::LoadK))
        .expect("expected LoadK instruction");

    let const_idx = decode_bx(*last_loadk);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) {
        assert_eq!(interner.resolve(*sym_id), "foo");
    } else {
        panic!("expected Symbol constant for defmacro return value");
    }
}

#[test]
fn defmacro_multiple_definitions() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(do (defmacro foo [x] x) (defmacro bar [y] y))").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    assert!(compiler.is_macro_by_name("foo"));
    assert!(compiler.is_macro_by_name("bar"));
}

#[test]
fn defmacro_redefine() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(do (defmacro foo [x] x) (defmacro foo [x y] y))").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    let macro_def = compiler.get_macro_by_name("foo").unwrap();
    // Should have the latest definition (2 params)
    assert_eq!(macro_def.arity(), 2);
}

// =========================================================================
// Macro Expansion Behavior Tests
// =========================================================================

#[test]
fn macro_call_without_expander_compiles_as_function_call() {
    // Without an expander, macro calls are treated as regular function calls.
    // The macro is stored in the registry but not expanded at compile time.
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();

    // First, define a macro
    let def_exprs = lonala_parser::parse("(defmacro my-macro [x] x)").unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry);
    let _chunk = compiler.compile_program(&def_exprs).unwrap();

    // Verify macro is stored
    assert!(registry.contains(interner.intern("my-macro")));

    // Now compile code that calls the macro (without expander)
    // This should compile as a regular function call to "my-macro"
    let call_exprs = lonala_parser::parse("(my-macro 42)").unwrap();
    let mut compiler2 = crate::Compiler::new(&mut interner, &mut registry);
    let result = compiler2.compile_program(&call_exprs);

    // Should compile successfully (as a function call)
    assert!(result.is_ok());

    // The resulting bytecode should have GetGlobal for "my-macro"
    // (not macro expansion)
    let chunk = result.unwrap();
    let code = chunk.code();

    // Should have GetGlobal instruction (the macro is treated as a function)
    let has_get_global = code
        .iter()
        .any(|&instr| decode_op(instr) == Some(Opcode::GetGlobal));
    assert!(
        has_get_global,
        "macro call without expander should compile as GetGlobal"
    );
}

#[test]
fn macro_registry_persists_across_compilations() {
    // Verify that macros defined in one compilation are available in subsequent
    // compilations with the same registry.
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();

    // First compilation: define macros
    let exprs1 = lonala_parser::parse("(do (defmacro m1 [x] x) (defmacro m2 [x y] y))").unwrap();
    let mut compiler1 = crate::Compiler::new(&mut interner, &mut registry);
    let _chunk1 = compiler1.compile_program(&exprs1).unwrap();

    // Verify macros are in registry
    assert_eq!(registry.len(), 2);
    assert!(registry.contains(interner.intern("m1")));
    assert!(registry.contains(interner.intern("m2")));

    // Second compilation: registry should still have the macros
    let exprs2 = lonala_parser::parse("(+ 1 2)").unwrap();
    let mut compiler2 = crate::Compiler::new(&mut interner, &mut registry);
    let _chunk2 = compiler2.compile_program(&exprs2).unwrap();

    // Macros should persist
    assert_eq!(registry.len(), 2);
    assert!(registry.contains(interner.intern("m1")));
    assert!(registry.contains(interner.intern("m2")));
}

#[test]
fn compile_with_registry_stores_macros() {
    // Test the compile_with_registry public API
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();

    // Compile code that defines a macro
    let _chunk =
        crate::compile_with_registry("(defmacro test-macro [x] x)", &mut interner, &mut registry)
            .unwrap();

    // Macro should be in registry
    assert!(registry.contains(interner.intern("test-macro")));
}

// =========================================================================
// Macro Expansion Tests (with Mock Expanders)
// =========================================================================
// These tests verify the macro expansion infrastructure using mock expanders.
// Full integration tests with the VM-based expander are in lona-kernel.

/// Helper struct that implements MacroExpander for testing.
/// Returns the first argument unchanged (identity macro).
struct IdentityExpander;

impl crate::MacroExpander for IdentityExpander {
    fn expand(
        &mut self,
        _definition: &crate::MacroDefinition,
        args: alloc::vec::Vec<lona_core::value::Value>,
        _interner: &mut symbol::Interner,
    ) -> Result<lona_core::value::Value, crate::MacroExpansionError> {
        // Return first argument, or nil if no args
        Ok(args
            .into_iter()
            .next()
            .unwrap_or(lona_core::value::Value::Nil))
    }
}

#[test]
fn macro_expansion_with_mock_expander() {
    // Test that macro expansion works with a mock expander
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let mut expander = IdentityExpander;

    // First define the macro
    let _def_chunk = crate::compile_with_expansion(
        "(defmacro id [x] x)",
        &mut interner,
        &mut registry,
        &mut expander,
    )
    .unwrap();

    // Verify macro is registered
    assert!(registry.contains(interner.intern("id")));

    // Now compile code that uses the macro
    // (id 42) should expand to 42 (via mock expander)
    let chunk =
        crate::compile_with_expansion("(id 42)", &mut interner, &mut registry, &mut expander)
            .unwrap();

    // The expanded code should just load 42 and return it
    let code = chunk.code();
    assert_eq!(code.len(), 2); // LoadK + Return
    assert_eq!(decode_op(*code.get(0_usize).unwrap()), Some(Opcode::LoadK));
}

/// Helper expander that always returns a fixed value to test expansion tracking.
struct FixedValueExpander {
    expansion_count: core::cell::Cell<usize>,
}

impl crate::MacroExpander for FixedValueExpander {
    fn expand(
        &mut self,
        _definition: &crate::MacroDefinition,
        _args: alloc::vec::Vec<lona_core::value::Value>,
        _interner: &mut symbol::Interner,
    ) -> Result<lona_core::value::Value, crate::MacroExpansionError> {
        let count = self.expansion_count.get();
        self.expansion_count.set(count.saturating_add(1));
        // Always return 42 to stop recursion after first expansion
        Ok(lona_core::value::Value::Integer(
            lona_core::integer::Integer::from_i64(42),
        ))
    }
}

#[test]
fn macro_expansion_tracks_expansion_count() {
    // Test that expansion happens and is tracked
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let mut expander = FixedValueExpander {
        expansion_count: core::cell::Cell::new(0),
    };

    // Define a macro
    let _def_chunk = crate::compile_with_expansion(
        "(defmacro test-macro [x] x)",
        &mut interner,
        &mut registry,
        &mut expander,
    )
    .unwrap();

    // Call the macro - expander returns 42, which terminates expansion
    let chunk = crate::compile_with_expansion(
        "(test-macro 1)",
        &mut interner,
        &mut registry,
        &mut expander,
    )
    .unwrap();

    // Verify expansion happened
    assert!(expander.expansion_count.get() >= 1);

    // Verify result is 42 (from the expander)
    let code = chunk.code();
    let k_idx = decode_bx(*code.get(0_usize).unwrap());
    assert_eq!(chunk.get_constant(k_idx), Some(&Constant::Integer(42)));
}
