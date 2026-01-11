// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the compiler.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::Vaddr;
use crate::bytecode::{decode_a, decode_b, decode_bx, decode_opcode, decode_sbx, op};
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::reader::read;
use crate::realm::{Realm, bootstrap};

/// Create a test environment with bootstrapped realm and process.
fn setup() -> Option<(Process, Realm, MockVSpace)> {
    // Memory layout:
    // 0x10000 - 0x20000: Process young heap (64KB)
    // 0x20000 - 0x24000: Process old heap (16KB)
    // 0x30000 - 0x40000: Realm code region (64KB)
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let mut proc = Process::new(1, young_base, young_size, old_base, old_size);

    // Create realm at a higher address (after process heaps)
    let realm_base = base.add(128 * 1024);
    let mut realm = Realm::new(realm_base, 64 * 1024);

    // Bootstrap realm and process
    let result = bootstrap(&mut realm, &mut mem)?;
    proc.bootstrap(result.ns_var, result.core_ns);

    Some((proc, realm, mem))
}

/// Parse and compile an expression.
fn compile_expr(
    src: &str,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut MockVSpace,
) -> Result<Chunk, CompileError> {
    let expr = read(src, proc, mem)
        .expect("parse error")
        .expect("empty input");
    compile(expr, proc, mem, realm)
}

// --- Literal tests ---

#[test]
fn compile_nil() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("nil", &mut proc, &mut realm, &mut mem).unwrap();

    assert_eq!(chunk.code.len(), 2); // LOADNIL + HALT
    assert_eq!(decode_opcode(chunk.code[0]), op::LOADNIL);
    assert_eq!(decode_a(chunk.code[0]), 0); // target = X0
    assert_eq!(decode_opcode(chunk.code[1]), op::HALT);
}

#[test]
fn compile_true() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("true", &mut proc, &mut realm, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADBOOL);
    assert_eq!(decode_a(chunk.code[0]), 0);
    assert_ne!(decode_bx(chunk.code[0]), 0); // true
}

#[test]
fn compile_false() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("false", &mut proc, &mut realm, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADBOOL);
    assert_eq!(decode_a(chunk.code[0]), 0);
    assert_eq!(decode_bx(chunk.code[0]), 0); // false
}

#[test]
fn compile_small_int() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("42", &mut proc, &mut realm, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_a(chunk.code[0]), 0);
    assert_eq!(decode_sbx(chunk.code[0]), 42);
}

#[test]
fn compile_negative_int() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("-100", &mut proc, &mut realm, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_sbx(chunk.code[0]), -100);
}

#[test]
fn compile_max_inline_int() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("131071", &mut proc, &mut realm, &mut mem).unwrap(); // MAX_SIGNED_BX

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_sbx(chunk.code[0]), 131_071);
}

#[test]
fn compile_min_inline_int() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("-131072", &mut proc, &mut realm, &mut mem).unwrap(); // MIN_SIGNED_BX

    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_sbx(chunk.code[0]), -131_072);
}

#[test]
fn compile_large_int_uses_constant() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("1000000", &mut proc, &mut realm, &mut mem).unwrap();

    // Too large for LOADINT, should use LOADK
    assert_eq!(decode_opcode(chunk.code[0]), op::LOADK);
    assert_eq!(chunk.constants.len(), 1);
    assert_eq!(chunk.constants[0], Value::int(1_000_000));
}

#[test]
fn compile_string() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("\"hello\"", &mut proc, &mut realm, &mut mem).unwrap();

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
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(+ 1 2)", &mut proc, &mut realm, &mut mem).unwrap();

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
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(+ 1 (+ 2 3))", &mut proc, &mut realm, &mut mem).unwrap();

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
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(* 6 7)", &mut proc, &mut realm, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[4]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[4]), 2); // * intrinsic ID
}

#[test]
fn compile_comparison() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(< 1 2)", &mut proc, &mut realm, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[4]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[4]), 6); // < intrinsic ID
}

#[test]
fn compile_not() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(not true)", &mut proc, &mut realm, &mut mem).unwrap();

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
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(nil? nil)", &mut proc, &mut realm, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[2]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[2]), 11); // nil? ID
}

#[test]
fn compile_str_single() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(str \"hello\")", &mut proc, &mut realm, &mut mem).unwrap();

    assert_eq!(decode_opcode(chunk.code[2]), op::INTRINSIC);
    assert_eq!(decode_a(chunk.code[2]), 14); // str ID
    assert_eq!(decode_b(chunk.code[2]), 1); // 1 arg
}

#[test]
fn compile_str_multiple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(str \"a\" \"b\" \"c\")", &mut proc, &mut realm, &mut mem).unwrap();

    let intrinsic_idx = chunk.code.len() - 2;
    assert_eq!(decode_opcode(chunk.code[intrinsic_idx]), op::INTRINSIC);
    assert_eq!(decode_b(chunk.code[intrinsic_idx]), 3); // 3 args
}

// --- Error tests ---

#[test]
fn compile_unbound_symbol() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = compile_expr("foo", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Err(CompileError::UnboundSymbol));
}

#[test]
fn compile_unknown_intrinsic() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = compile_expr("(unknown 1 2)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Err(CompileError::UnboundSymbol));
}

#[test]
fn compile_callable_expression_head() {
    // With callable data structure support, non-symbol heads compile
    // and are evaluated at runtime. The CALL instruction will dispatch
    // based on the callee type.
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(42 1 2)", &mut proc, &mut realm, &mut mem).unwrap();

    // Should compile: loads 42 into a temp, loads args, emits CALL
    // At runtime, this will fail with NotCallable
    assert!(chunk.code.len() > 1);
    assert_eq!(decode_opcode(*chunk.code.last().unwrap()), op::HALT);
}

// --- Disassembly tests ---

// --- def syntax error tests ---

#[test]
fn compile_def_no_args_error() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // (def) with no arguments should fail
    let result = compile_expr("(def)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Err(CompileError::InvalidSyntax));
}

#[test]
fn compile_def_non_symbol_name_error() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // (def 123 value) - name must be a symbol
    let result = compile_expr("(def 123 42)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Err(CompileError::InvalidSyntax));
}

#[test]
fn compile_def_too_many_args_error() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // (def x 1 2) - too many arguments
    let result = compile_expr("(def x 1 2)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Err(CompileError::InvalidSyntax));
}

// --- Qualified symbol resolution tests ---

#[test]
fn compile_qualified_symbol_resolution() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // lona.core/+ should resolve to the + intrinsic
    let chunk = compile_expr("(lona.core/+ 1 2)", &mut proc, &mut realm, &mut mem).unwrap();

    // Should compile successfully and produce INTRINSIC instruction
    assert!(chunk.code.len() > 1);
    // Find the INTRINSIC instruction
    let has_intrinsic = chunk
        .code
        .iter()
        .any(|&instr| decode_opcode(instr) == op::INTRINSIC);
    assert!(has_intrinsic, "should emit INTRINSIC for lona.core/+");
}

#[test]
fn compile_unknown_namespace_error() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // unknown.ns/x should fail - namespace doesn't exist
    let result = compile_expr("unknown.ns/x", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Err(CompileError::UnboundSymbol));
}

#[test]
fn compile_qualified_unknown_var_error() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // lona.core/nonexistent - var doesn't exist in namespace
    let result = compile_expr("lona.core/nonexistent", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Err(CompileError::UnboundSymbol));
}

// --- Disassembly tests ---

#[test]
fn disassemble_simple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let chunk = compile_expr("(+ 1 2)", &mut proc, &mut realm, &mut mem).unwrap();
    let dis = disassemble(&chunk);

    assert!(dis.contains("LOADINT"));
    assert!(dis.contains("INTRINSIC"));
    assert!(dis.contains("HALT"));
}
