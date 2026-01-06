// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the compiler.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::Vaddr;
use crate::bytecode::{decode_a, decode_b, decode_bx, decode_opcode, decode_sbx, op};
use crate::heap::Heap;
use crate::platform::MockVSpace;
use crate::reader::read;

/// Create a test environment.
fn setup() -> (Heap, MockVSpace) {
    let mem = MockVSpace::new(64 * 1024, Vaddr::new(0x1_0000));
    let heap = Heap::new(Vaddr::new(0x1_0000 + 64 * 1024), 64 * 1024);
    (heap, mem)
}

/// Parse and compile an expression.
fn compile_expr(src: &str, heap: &mut Heap, mem: &mut MockVSpace) -> Result<Chunk, CompileError> {
    let expr = read(src, heap, mem)
        .expect("parse error")
        .expect("empty input");
    compile(expr, heap, mem)
}

// --- Literal tests ---

#[test]
fn compile_nil() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("nil", &mut heap, &mut mem).unwrap();

    assert_eq!(chunk.code.len(), 2); // LOADNIL + HALT
    assert_eq!(decode_opcode(chunk.code[0]), op::LOADNIL);
    assert_eq!(decode_a(chunk.code[0]), 0); // target = X0
    assert_eq!(decode_opcode(chunk.code[1]), op::HALT);
}

#[test]
fn compile_true() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("true", &mut heap, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADBOOL);
    assert_eq!(decode_a(chunk.code[0]), 0);
    assert_ne!(decode_bx(chunk.code[0]), 0); // true
}

#[test]
fn compile_false() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("false", &mut heap, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADBOOL);
    assert_eq!(decode_a(chunk.code[0]), 0);
    assert_eq!(decode_bx(chunk.code[0]), 0); // false
}

#[test]
fn compile_small_int() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("42", &mut heap, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_a(chunk.code[0]), 0);
    assert_eq!(decode_sbx(chunk.code[0]), 42);
}

#[test]
fn compile_negative_int() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("-100", &mut heap, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_sbx(chunk.code[0]), -100);
}

#[test]
fn compile_max_inline_int() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("131071", &mut heap, &mut mem).unwrap(); // MAX_SIGNED_BX

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_sbx(chunk.code[0]), 131_071);
}

#[test]
fn compile_min_inline_int() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("-131072", &mut heap, &mut mem).unwrap(); // MIN_SIGNED_BX

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_sbx(chunk.code[0]), -131_072);
}

#[test]
fn compile_large_int_uses_constant() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("1000000", &mut heap, &mut mem).unwrap();

    // Too large for LOADINT, should use LOADK
    assert_eq!(decode_opcode(chunk.code[0]), op::LOADK);
    assert_eq!(chunk.constants.len(), 1);
    assert_eq!(chunk.constants[0], Value::int(1_000_000));
}

#[test]
fn compile_string() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("\"hello\"", &mut heap, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADK);
    assert_eq!(chunk.constants.len(), 1);
    // The string is stored in the constant pool
    if let Value::String(_) = chunk.constants[0] {
        // OK
    } else {
        panic!("Expected string constant");
    }
}

// --- Intrinsic call tests ---

#[test]
fn compile_add() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("(+ 1 2)", &mut heap, &mut mem).unwrap();

    // Arguments compiled to temp registers, then moved to X1..Xn:
    // 0: LOADINT X128, 1   (arg0 to temp)
    // 1: LOADINT X129, 2   (arg1 to temp)
    // 2: MOVE X1, X128     (move temps to arg positions)
    // 3: MOVE X2, X129
    // 4: INTRINSIC +, 2
    // 5: HALT

    assert_eq!(chunk.code.len(), 6);

    // X128 = 1
    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_a(chunk.code[0]), 128);
    assert_eq!(decode_sbx(chunk.code[0]), 1);

    // X129 = 2
    assert_eq!(decode_opcode(chunk.code[1]), op::LOADINT);
    assert_eq!(decode_a(chunk.code[1]), 129);
    assert_eq!(decode_sbx(chunk.code[1]), 2);

    // MOVE X1, X128
    assert_eq!(decode_opcode(chunk.code[2]), op::MOVE);
    assert_eq!(decode_a(chunk.code[2]), 1);
    assert_eq!(decode_b(chunk.code[2]), 128);

    // MOVE X2, X129
    assert_eq!(decode_opcode(chunk.code[3]), op::MOVE);
    assert_eq!(decode_a(chunk.code[3]), 2);
    assert_eq!(decode_b(chunk.code[3]), 129);

    // INTRINSIC
    assert_eq!(decode_opcode(chunk.code[4]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[4]), 0); // + intrinsic ID
    assert_eq!(decode_b(chunk.code[4]), 2); // 2 args

    assert_eq!(decode_opcode(chunk.code[5]), op::HALT);
}

#[test]
fn compile_nested_add() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("(+ 1 (+ 2 3))", &mut heap, &mut mem).unwrap();

    // With temp register allocation for nested calls:
    // Outer + uses temps X128, X129 (temp_base=128, 2 args)
    // Inner + uses temps X130, X131 (temp_base=130, 2 args)
    //
    // 0:  LOADINT X128, 1      (outer arg0 to temp)
    // 1:  LOADINT X130, 2      (inner arg0 to temp)
    // 2:  LOADINT X131, 3      (inner arg1 to temp)
    // 3:  MOVE X1, X130        (inner: move temps to args)
    // 4:  MOVE X2, X131
    // 5:  INTRINSIC +, 2       (inner call)
    // 6:  MOVE X129, X0        (save inner result to outer temp)
    // 7:  MOVE X1, X128        (outer: move temps to args)
    // 8:  MOVE X2, X129
    // 9:  INTRINSIC +, 2       (outer call)
    // 10: HALT

    assert_eq!(chunk.code.len(), 11);
    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT); // X128 = 1
    assert_eq!(decode_opcode(chunk.code[1]), op::LOADINT); // X130 = 2
    assert_eq!(decode_opcode(chunk.code[2]), op::LOADINT); // X131 = 3
    assert_eq!(decode_opcode(chunk.code[3]), op::MOVE); // X1 = X130
    assert_eq!(decode_opcode(chunk.code[4]), op::MOVE); // X2 = X131
    assert_eq!(decode_opcode(chunk.code[5]), op::INTRINSIC); // inner +
    assert_eq!(decode_opcode(chunk.code[6]), op::MOVE); // X129 = X0
    assert_eq!(decode_opcode(chunk.code[7]), op::MOVE); // X1 = X128
    assert_eq!(decode_opcode(chunk.code[8]), op::MOVE); // X2 = X129
    assert_eq!(decode_opcode(chunk.code[9]), op::INTRINSIC); // outer +
    assert_eq!(decode_opcode(chunk.code[10]), op::HALT);
}

#[test]
fn compile_mul() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("(* 6 7)", &mut heap, &mut mem).unwrap();

    // With temp registers: LOADINT X128, LOADINT X129, MOVE X1, MOVE X2, INTRINSIC, HALT
    assert_eq!(decode_opcode(chunk.code[4]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[4]), 2); // * intrinsic ID
}

#[test]
fn compile_comparison() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("(< 1 2)", &mut heap, &mut mem).unwrap();

    // With temp registers: LOADINT X128, LOADINT X129, MOVE X1, MOVE X2, INTRINSIC, HALT
    assert_eq!(decode_opcode(chunk.code[4]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[4]), 6); // < intrinsic ID
}

#[test]
fn compile_not() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("(not true)", &mut heap, &mut mem).unwrap();

    // With temp registers:
    // 0: LOADBOOL X128, true
    // 1: MOVE X1, X128
    // 2: INTRINSIC not, 1
    // 3: HALT

    assert_eq!(chunk.code.len(), 4);
    assert_eq!(decode_opcode(chunk.code[0]), op::LOADBOOL);
    assert_eq!(decode_a(chunk.code[0]), 128); // X128
    assert_eq!(decode_opcode(chunk.code[1]), op::MOVE);
    assert_eq!(decode_a(chunk.code[1]), 1); // X1
    assert_eq!(decode_b(chunk.code[1]), 128); // from X128
    assert_eq!(decode_opcode(chunk.code[2]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[2]), 10); // not ID
    assert_eq!(decode_b(chunk.code[2]), 1); // 1 arg
}

#[test]
fn compile_nil_predicate() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("(nil? nil)", &mut heap, &mut mem).unwrap();

    // With temp registers: LOADNIL X128, MOVE X1, INTRINSIC, HALT
    assert_eq!(decode_opcode(chunk.code[2]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[2]), 11); // nil? ID
}

#[test]
fn compile_str_single() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("(str \"hello\")", &mut heap, &mut mem).unwrap();

    // With temp registers: LOADK X128, MOVE X1, INTRINSIC, HALT
    assert_eq!(decode_opcode(chunk.code[2]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[2]), 14); // str ID
    assert_eq!(decode_b(chunk.code[2]), 1); // 1 arg
}

#[test]
fn compile_str_multiple() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("(str \"a\" \"b\" \"c\")", &mut heap, &mut mem).unwrap();

    // Should have 3 LOADK + INTRINSIC + HALT
    let intrinsic_idx = chunk.code.len() - 2;
    assert_eq!(decode_opcode(chunk.code[intrinsic_idx]), op::INTRINSIC);
    assert_eq!(decode_b(chunk.code[intrinsic_idx]), 3); // 3 args
}

// --- Error tests ---

#[test]
fn compile_unbound_symbol() {
    let (mut heap, mut mem) = setup();
    let result = compile_expr("foo", &mut heap, &mut mem);
    assert_eq!(result, Err(CompileError::UnboundSymbol));
}

#[test]
fn compile_unknown_intrinsic() {
    let (mut heap, mut mem) = setup();
    let result = compile_expr("(unknown 1 2)", &mut heap, &mut mem);
    assert_eq!(result, Err(CompileError::UnboundSymbol));
}

#[test]
fn compile_invalid_call_head() {
    let (mut heap, mut mem) = setup();
    // (42 1 2) - number as operator
    let result = compile_expr("(42 1 2)", &mut heap, &mut mem);
    assert_eq!(result, Err(CompileError::InvalidSyntax));
}

// --- Disassembly tests ---

#[test]
fn disassemble_simple() {
    let (mut heap, mut mem) = setup();
    let chunk = compile_expr("(+ 1 2)", &mut heap, &mut mem).unwrap();
    let dis = disassemble(&chunk);

    assert!(dis.contains("LOADINT"));
    assert!(dis.contains("INTRINSIC"));
    assert!(dis.contains("HALT"));
}
