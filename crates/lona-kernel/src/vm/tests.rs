// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the VM interpreter.

use lona_core::integer::Integer;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lonala_compiler::opcode::{Opcode, encode_abc, encode_abx, encode_asbx, rk_constant};
use lonala_compiler::{Chunk, Constant};
use lonala_parser::Span;

use super::NativeError;
use super::error::Error;
use super::interpreter::Vm;

/// Creates a VM with a fresh interner for testing.
fn make_vm(interner: &Interner) -> Vm<'_> {
    Vm::new(interner)
}

/// Creates a test chunk.
fn make_chunk() -> Chunk {
    Chunk::new()
}

// =============================================================================
// Literal Execution Tests
// =============================================================================

#[test]
fn execute_load_true_and_return() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let _idx = chunk.emit(
        encode_abc(Opcode::LoadTrue, 0, 0, 0),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn execute_load_false_and_return() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let _idx = chunk.emit(
        encode_abc(Opcode::LoadFalse, 0, 0, 0),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn execute_load_nil_and_return() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let _idx = chunk.emit(
        encode_abc(Opcode::LoadNil, 0, 0, 0),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn execute_load_integer() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k_idx = chunk.add_constant(Constant::Integer(42)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k_idx),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(42)));
}

#[test]
fn execute_load_float() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k_idx = chunk.add_constant(Constant::Float(3.14)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k_idx),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Float(3.14));
}

// =============================================================================
// Arithmetic Tests
// =============================================================================

#[test]
fn execute_add_integers() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

    // LoadK R0, K0  ; R0 = 1
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k0),
        Span::new(0_usize, 1_usize),
    );
    // LoadK R1, K1  ; R1 = 2
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 1, k1),
        Span::new(1_usize, 2_usize),
    );
    // Add R0, R0, R1  ; R0 = R0 + R1 = 3
    let _idx = chunk.emit(
        encode_abc(Opcode::Add, 0, 0, 1),
        Span::new(2_usize, 3_usize),
    );
    // Return R0, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(3_usize, 4_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(3)));
}

#[test]
fn execute_add_with_constants() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(20)).unwrap();

    // Get RK encodings for constants
    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    // Add R0, K0, K1  ; R0 = 10 + 20 = 30
    let _idx = chunk.emit(
        encode_abc(Opcode::Add, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    // Return R0, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(30)));
}

#[test]
fn execute_add_floats() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Float(1.5)).unwrap();
    let k1 = chunk.add_constant(Constant::Float(2.5)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Add, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Float(4.0));
}

#[test]
fn execute_add_mixed_promotes_to_float() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let k1 = chunk.add_constant(Constant::Float(2.5)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Add, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Float(3.5));
}

#[test]
fn execute_sub() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(3)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Sub, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(7)));
}

#[test]
fn execute_mul() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(6)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(7)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Mul, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(42)));
}

#[test]
fn execute_div() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    // Use 6 / 2 = 3 (divides evenly, returns Integer not Ratio)
    let k0 = chunk.add_constant(Constant::Integer(6)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Div, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(3)));
}

#[test]
fn execute_mod() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(3)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Mod, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(1)));
}

#[test]
fn execute_neg_integer() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(42)).unwrap();

    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k0),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Neg, 0, 0, 0),
        Span::new(1_usize, 2_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(2_usize, 3_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(-42)));
}

#[test]
fn execute_neg_float() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Float(3.14)).unwrap();

    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k0),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Neg, 0, 0, 0),
        Span::new(1_usize, 2_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(2_usize, 3_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Float(-3.14));
}

// =============================================================================
// Division by Zero Tests
// =============================================================================

#[test]
fn execute_div_by_zero_integer() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(0)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Div, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );

    let result = vm.execute(&chunk);
    assert!(matches!(result, Err(Error::DivisionByZero { .. })));
}

#[test]
fn execute_mod_by_zero() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(0)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Mod, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );

    let result = vm.execute(&chunk);
    assert!(matches!(result, Err(Error::DivisionByZero { .. })));
}

// =============================================================================
// Comparison Tests
// =============================================================================

#[test]
fn execute_eq_true() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(42)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(42)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Eq, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn execute_eq_false() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Eq, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn execute_lt() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Lt, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn execute_not_truthy() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let _idx = chunk.emit(
        encode_abc(Opcode::LoadTrue, 0, 0, 0),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Not, 0, 0, 0),
        Span::new(1_usize, 2_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(2_usize, 3_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn execute_not_falsy() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let _idx = chunk.emit(
        encode_abc(Opcode::LoadNil, 0, 0, 0),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Not, 0, 0, 0),
        Span::new(1_usize, 2_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(2_usize, 3_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Bool(true));
}

// =============================================================================
// Global Variable Tests
// =============================================================================

#[test]
fn execute_set_and_get_global() {
    let mut interner = Interner::new();
    let x_sym = interner.intern("x");

    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k_val = chunk.add_constant(Constant::Integer(42)).unwrap();
    let k_sym = chunk.add_constant(Constant::Symbol(x_sym)).unwrap();

    // LoadK R0, K0  ; R0 = 42
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k_val),
        Span::new(0_usize, 1_usize),
    );
    // SetGlobal R0, K1  ; globals[x] = 42
    let _idx = chunk.emit(
        encode_abx(Opcode::SetGlobal, 0, k_sym),
        Span::new(1_usize, 2_usize),
    );
    // LoadNil R0  ; R0 = nil (clear it)
    let _idx = chunk.emit(
        encode_abc(Opcode::LoadNil, 0, 0, 0),
        Span::new(2_usize, 3_usize),
    );
    // GetGlobal R0, K1  ; R0 = globals[x]
    let _idx = chunk.emit(
        encode_abx(Opcode::GetGlobal, 0, k_sym),
        Span::new(3_usize, 4_usize),
    );
    // Return R0, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(4_usize, 5_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(42)));
}

#[test]
fn execute_undefined_global_error() {
    let mut interner = Interner::new();
    let x_sym = interner.intern("undefined");

    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k_sym = chunk.add_constant(Constant::Symbol(x_sym)).unwrap();

    // GetGlobal R0, K0  ; should fail
    let _idx = chunk.emit(
        encode_abx(Opcode::GetGlobal, 0, k_sym),
        Span::new(0_usize, 1_usize),
    );

    let result = vm.execute(&chunk);
    assert!(matches!(result, Err(Error::UndefinedGlobal { .. })));
}

// =============================================================================
// Control Flow Tests
// =============================================================================

#[test]
fn execute_unconditional_jump() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

    // 0: LoadK R0, K0  ; R0 = 1
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k0),
        Span::new(0_usize, 1_usize),
    );
    // 1: Jump +1  ; skip next instruction
    let _idx = chunk.emit(encode_asbx(Opcode::Jump, 0, 1), Span::new(1_usize, 2_usize));
    // 2: LoadK R0, K1  ; R0 = 2 (should be skipped)
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k1),
        Span::new(2_usize, 3_usize),
    );
    // 3: Return R0, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(3_usize, 4_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(1)));
}

#[test]
fn execute_jump_if_true() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

    // 0: LoadTrue R0
    let _idx = chunk.emit(
        encode_abc(Opcode::LoadTrue, 0, 0, 0),
        Span::new(0_usize, 1_usize),
    );
    // 1: LoadK R1, K0  ; R1 = 1
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 1, k0),
        Span::new(1_usize, 2_usize),
    );
    // 2: JumpIf R0, +1  ; if true, skip next
    let _idx = chunk.emit(
        encode_asbx(Opcode::JumpIf, 0, 1),
        Span::new(2_usize, 3_usize),
    );
    // 3: LoadK R1, K1  ; R1 = 2 (should be skipped)
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 1, k1),
        Span::new(3_usize, 4_usize),
    );
    // 4: Return R1, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 1, 1, 0),
        Span::new(4_usize, 5_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(1)));
}

#[test]
fn execute_jump_if_not_false() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

    // 0: LoadFalse R0
    let _idx = chunk.emit(
        encode_abc(Opcode::LoadFalse, 0, 0, 0),
        Span::new(0_usize, 1_usize),
    );
    // 1: LoadK R1, K0  ; R1 = 1
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 1, k0),
        Span::new(1_usize, 2_usize),
    );
    // 2: JumpIfNot R0, +1  ; if false, skip next
    let _idx = chunk.emit(
        encode_asbx(Opcode::JumpIfNot, 0, 1),
        Span::new(2_usize, 3_usize),
    );
    // 3: LoadK R1, K1  ; R1 = 2 (should be skipped)
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 1, k1),
        Span::new(3_usize, 4_usize),
    );
    // 4: Return R1, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 1, 1, 0),
        Span::new(4_usize, 5_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(1)));
}

// =============================================================================
// Type Error Tests
// =============================================================================

#[test]
fn execute_add_type_error() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let k1 = chunk.add_constant(Constant::Bool(true)).unwrap();

    let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
    let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

    let _idx = chunk.emit(
        encode_abc(Opcode::Add, 0, rk0, rk1),
        Span::new(0_usize, 1_usize),
    );

    let result = vm.execute(&chunk);
    assert!(matches!(result, Err(Error::TypeError { .. })));
}

#[test]
fn execute_neg_type_error() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let _idx = chunk.emit(
        encode_abc(Opcode::LoadTrue, 0, 0, 0),
        Span::new(0_usize, 1_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Neg, 0, 0, 0),
        Span::new(1_usize, 2_usize),
    );

    let result = vm.execute(&chunk);
    assert!(matches!(result, Err(Error::TypeError { .. })));
}

// =============================================================================
// Move Operation Test
// =============================================================================

#[test]
fn execute_move() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    let k0 = chunk.add_constant(Constant::Integer(42)).unwrap();

    // LoadK R0, K0  ; R0 = 42
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k0),
        Span::new(0_usize, 1_usize),
    );
    // Move R1, R0  ; R1 = R0
    let _idx = chunk.emit(
        encode_abc(Opcode::Move, 1, 0, 0),
        Span::new(1_usize, 2_usize),
    );
    // Return R1, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 1, 1, 0),
        Span::new(2_usize, 3_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(42)));
}

// =============================================================================
// Empty Chunk Test
// =============================================================================

#[test]
fn execute_empty_chunk_returns_nil() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);
    let chunk = make_chunk();

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Nil);
}

// =============================================================================
// Native Function Call Tests
// =============================================================================

#[test]
fn execute_print_returns_nil() {
    // Test that print call completes without error and returns nil
    let mut interner = Interner::new();
    let print_sym = interner.intern("print");

    let mut vm = Vm::new(&interner);
    vm.update_print_symbol(print_sym);
    // Register print as a global (the value is the symbol itself)
    vm.set_global(print_sym, Value::Symbol(print_sym));
    // Note: No print callback set - output is discarded

    let mut chunk = make_chunk();
    // GetGlobal R0, K0 (print symbol)
    let k_print = chunk.add_constant(Constant::Symbol(print_sym)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::GetGlobal, 0, k_print),
        Span::new(0_usize, 5_usize),
    );
    // LoadK R1, K1 (42)
    let k_42 = chunk.add_constant(Constant::Integer(42)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 1, k_42),
        Span::new(6_usize, 8_usize),
    );
    // Call R0, 1, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Call, 0, 1, 1),
        Span::new(0_usize, 10_usize),
    );
    // Return R0, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(0_usize, 10_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    // Print returns nil
    assert_eq!(result, Value::Nil);
}

#[test]
fn execute_print_with_multiple_args() {
    let mut interner = Interner::new();
    let print_sym = interner.intern("print");

    let mut vm = Vm::new(&interner);
    vm.update_print_symbol(print_sym);
    vm.set_global(print_sym, Value::Symbol(print_sym));

    let mut chunk = make_chunk();
    // GetGlobal R0, K0 (print symbol)
    let k_print = chunk.add_constant(Constant::Symbol(print_sym)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::GetGlobal, 0, k_print),
        Span::new(0_usize, 5_usize),
    );
    // LoadK R1, K1 (1)
    let k_1 = chunk.add_constant(Constant::Integer(1)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 1, k_1),
        Span::new(0_usize, 1_usize),
    );
    // LoadK R2, K2 (2)
    let k_2 = chunk.add_constant(Constant::Integer(2)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 2, k_2),
        Span::new(0_usize, 1_usize),
    );
    // LoadK R3, K3 (3)
    let k_3 = chunk.add_constant(Constant::Integer(3)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 3, k_3),
        Span::new(0_usize, 1_usize),
    );
    // Call R0, 3, 1 (3 arguments)
    let _idx = chunk.emit(
        encode_abc(Opcode::Call, 0, 3, 1),
        Span::new(0_usize, 10_usize),
    );
    // Return R0, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(0_usize, 10_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn execute_native_function() {
    fn native_double(args: &[Value], _interner: &Interner) -> Result<Value, NativeError> {
        if args.len() != 1_usize {
            return Err(NativeError::ArityMismatch {
                expected: 1,
                got: args.len(),
            });
        }
        let num = args
            .first()
            .and_then(Value::as_integer)
            .ok_or(NativeError::TypeError {
                expected: "integer",
                got: "non-integer",
                arg_index: 0,
            })?;
        Ok(Value::Integer(num * &Integer::from_i64(2)))
    }

    let mut interner = Interner::new();
    let double_sym = interner.intern("double");

    let mut vm = Vm::new(&interner);
    vm.register_native(double_sym, native_double);
    // Register the function as a global
    vm.set_global(double_sym, Value::Symbol(double_sym));

    let mut chunk = make_chunk();
    // GetGlobal R0, K0 (double symbol)
    let k_double = chunk.add_constant(Constant::Symbol(double_sym)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::GetGlobal, 0, k_double),
        Span::new(0_usize, 6_usize),
    );
    // LoadK R1, K1 (21)
    let k_21 = chunk.add_constant(Constant::Integer(21)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 1, k_21),
        Span::new(0_usize, 2_usize),
    );
    // Call R0, 1, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Call, 0, 1, 1),
        Span::new(0_usize, 10_usize),
    );
    // Return R0, 1
    let _idx = chunk.emit(
        encode_abc(Opcode::Return, 0, 1, 0),
        Span::new(0_usize, 10_usize),
    );

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(42)));
}

#[test]
fn execute_undefined_function_error() {
    let mut interner = Interner::new();
    let unknown_sym = interner.intern("unknown");

    let mut vm = Vm::new(&interner);
    // Register the symbol as a global (so GetGlobal works)
    // but don't register it as a native function
    vm.set_global(unknown_sym, Value::Symbol(unknown_sym));

    let mut chunk = make_chunk();
    let k_unknown = chunk.add_constant(Constant::Symbol(unknown_sym)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::GetGlobal, 0, k_unknown),
        Span::new(0_usize, 7_usize),
    );
    let _idx = chunk.emit(
        encode_abc(Opcode::Call, 0, 0, 1),
        Span::new(0_usize, 10_usize),
    );

    let result = vm.execute(&chunk);
    assert!(matches!(result, Err(Error::UndefinedFunction { .. })));
}

#[test]
fn execute_call_non_symbol_error() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let mut chunk = make_chunk();
    // Load an integer (not a symbol) into R0
    let k_42 = chunk.add_constant(Constant::Integer(42)).unwrap();
    let _idx = chunk.emit(
        encode_abx(Opcode::LoadK, 0, k_42),
        Span::new(0_usize, 2_usize),
    );
    // Try to call it
    let _idx = chunk.emit(
        encode_abc(Opcode::Call, 0, 0, 1),
        Span::new(0_usize, 10_usize),
    );

    let result = vm.execute(&chunk);
    assert!(matches!(result, Err(Error::NotCallable { .. })));
}

// =============================================================================
// Integration Tests: End-to-End Compile + Execute with Persistent State
// =============================================================================

/// Simulates REPL-like evaluation: compiles source, executes with persistent
/// globals, and returns the result. This tests the full pipeline that the
/// REPL uses, ensuring globals persist between evaluations.
fn eval_with_state(
    source: &str,
    interner: &mut Interner,
    globals: &mut super::Globals,
) -> Result<Value, super::Error> {
    // Compile the source
    let chunk = lonala_compiler::compile(source, interner)
        .expect("compilation should succeed in eval_with_state");

    // Create VM and restore persistent globals
    let mut vm = Vm::new(interner);
    *vm.globals_mut() = globals.clone();

    // Register print function
    if let Some(print_sym) = interner.get("print") {
        vm.update_print_symbol(print_sym);
        vm.set_global(print_sym, Value::Symbol(print_sym));
    }

    // Execute
    let result = vm.execute(&chunk)?;

    // Save globals back (including any newly defined)
    *globals = vm.globals().clone();

    Ok(result)
}

#[test]
fn integration_def_persists_across_evaluations() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // First evaluation: define x
    let result1 = eval_with_state("(def x 42)", &mut interner, &mut globals).unwrap();
    // def returns the symbol
    assert!(matches!(result1, Value::Symbol(_)));

    // Second evaluation: use x
    let result2 = eval_with_state("x", &mut interner, &mut globals).unwrap();
    assert_eq!(result2, Value::Integer(Integer::from_i64(42)));
}

#[test]
fn integration_def_multiple_variables() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // Define two variables in separate evaluations
    let _r1 = eval_with_state("(def a 10)", &mut interner, &mut globals).unwrap();
    let _r2 = eval_with_state("(def b 20)", &mut interner, &mut globals).unwrap();

    // Use both in arithmetic
    let result = eval_with_state("(+ a b)", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(30)));
}

#[test]
fn integration_def_overwrite() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // Define x
    let _r1 = eval_with_state("(def x 1)", &mut interner, &mut globals).unwrap();
    let result1 = eval_with_state("x", &mut interner, &mut globals).unwrap();
    assert_eq!(result1, Value::Integer(Integer::from_i64(1)));

    // Redefine x
    let _r2 = eval_with_state("(def x 2)", &mut interner, &mut globals).unwrap();
    let result2 = eval_with_state("x", &mut interner, &mut globals).unwrap();
    assert_eq!(result2, Value::Integer(Integer::from_i64(2)));
}

#[test]
fn integration_if_with_defined_variable() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // Define x
    let _r1 = eval_with_state("(def x 42)", &mut interner, &mut globals).unwrap();

    // Use x in if condition
    let result = eval_with_state(
        "(if (> x 10) \"big\" \"small\")",
        &mut interner,
        &mut globals,
    )
    .unwrap();
    assert_eq!(
        result,
        Value::String(lona_core::string::HeapStr::new("big"))
    );
}

#[test]
fn integration_do_with_def() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // Use do to define and immediately use a variable
    let result = eval_with_state("(do (def y 10) (+ y 5))", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(15)));

    // y should persist after the do block
    let result2 = eval_with_state("y", &mut interner, &mut globals).unwrap();
    assert_eq!(result2, Value::Integer(Integer::from_i64(10)));
}

#[test]
fn integration_complex_expression_sequence() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // Session 1: Define initial values
    let _r1 = eval_with_state("(def x 10)", &mut interner, &mut globals).unwrap();
    let _r2 = eval_with_state("(def y 20)", &mut interner, &mut globals).unwrap();

    // Session 2: Compute and store result
    let _r3 = eval_with_state("(def sum (+ x y))", &mut interner, &mut globals).unwrap();

    // Session 3: Verify and use computed value
    let result = eval_with_state("(* sum 2)", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(60)));
}

#[test]
fn integration_undefined_global_error() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // Try to use an undefined variable
    let result = eval_with_state("undefined_var", &mut interner, &mut globals);
    assert!(matches!(result, Err(Error::UndefinedGlobal { .. })));
}

#[test]
fn integration_if_branches() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // Test true branch
    let result1 = eval_with_state("(if true 1 2)", &mut interner, &mut globals).unwrap();
    assert_eq!(result1, Value::Integer(Integer::from_i64(1)));

    // Test false branch
    let result2 = eval_with_state("(if false 1 2)", &mut interner, &mut globals).unwrap();
    assert_eq!(result2, Value::Integer(Integer::from_i64(2)));

    // Test nil is falsy
    let result3 = eval_with_state("(if nil 1 2)", &mut interner, &mut globals).unwrap();
    assert_eq!(result3, Value::Integer(Integer::from_i64(2)));

    // Test 0 is truthy
    let result4 = eval_with_state("(if 0 1 2)", &mut interner, &mut globals).unwrap();
    assert_eq!(result4, Value::Integer(Integer::from_i64(1)));
}

#[test]
fn integration_if_no_else_returns_nil() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(if false 1)", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn integration_do_empty() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(do)", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn integration_do_returns_last() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(do 1 2 3)", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(3)));
}

// =============================================================================
// Integration Tests for `let`
// =============================================================================

#[test]
fn integration_let_single_binding() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(let [x 42] x)", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(42)));
}

#[test]
fn integration_let_multiple_bindings() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(let [x 1 y 2] (+ x y))", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(3)));
}

#[test]
fn integration_let_forward_reference() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(let [x 1 y (+ x 1)] y)", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(2)));
}

#[test]
fn integration_let_nested() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state(
        "(let [x 1] (let [y 2] (+ x y)))",
        &mut interner,
        &mut globals,
    )
    .unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(3)));
}

#[test]
fn integration_let_shadows_global() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // Define a global x = 100
    let _result = eval_with_state("(def x 100)", &mut interner, &mut globals).unwrap();

    // let should shadow the global
    let result = eval_with_state("(let [x 1] x)", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(1)));

    // After let, global should still be accessible
    let result2 = eval_with_state("x", &mut interner, &mut globals).unwrap();
    assert_eq!(result2, Value::Integer(Integer::from_i64(100)));
}

#[test]
fn integration_let_empty_body() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(let [x 1])", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn integration_let_multiple_body_exprs() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(let [x 1] x (+ x 1))", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(2)));
}

// =============================================================================
// Integration Tests for `quote`
// =============================================================================

#[test]
fn integration_quote_symbol() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(quote foo)", &mut interner, &mut globals).unwrap();
    // Should return a symbol value
    assert!(matches!(result, Value::Symbol(_)));
}

#[test]
fn integration_quote_integer() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(quote 42)", &mut interner, &mut globals).unwrap();
    assert_eq!(result, Value::Integer(Integer::from_i64(42)));
}

#[test]
fn integration_quote_list() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(quote (1 2 3))", &mut interner, &mut globals).unwrap();
    // Should return a list value
    if let Value::List(list) = result {
        assert_eq!(list.len(), 3);
        assert_eq!(list.first(), Some(&Value::Integer(Integer::from_i64(1))));
    } else {
        panic!("expected List value, got {:?}", result);
    }
}

#[test]
fn integration_quote_vector() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(quote [1 2 3])", &mut interner, &mut globals).unwrap();
    // Should return a vector value
    if let Value::Vector(vector) = result {
        assert_eq!(vector.len(), 3);
        assert_eq!(vector.get(0), Some(&Value::Integer(Integer::from_i64(1))));
    } else {
        panic!("expected Vector value, got {:?}", result);
    }
}

#[test]
fn integration_quote_prevents_evaluation() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    // (+ 1 2) would normally evaluate to 3, but quoted it should be a list
    let result = eval_with_state("(quote (+ 1 2))", &mut interner, &mut globals).unwrap();
    if let Value::List(list) = result {
        assert_eq!(list.len(), 3);
        // First element should be symbol '+'
        assert!(matches!(list.first(), Some(Value::Symbol(_))));
    } else {
        panic!("expected List value, got {:?}", result);
    }
}

#[test]
fn integration_quote_nested_list() {
    let mut interner = Interner::new();
    let mut globals = super::Globals::new();

    let result = eval_with_state("(quote (a (b c) d))", &mut interner, &mut globals).unwrap();
    if let Value::List(list) = result {
        assert_eq!(list.len(), 3);
        // Second element should be a list
        let rest = list.rest();
        if let Some(Value::List(inner)) = rest.first() {
            assert_eq!(inner.len(), 2);
        } else {
            panic!("expected nested List, got {:?}", rest.first());
        }
    } else {
        panic!("expected List value, got {:?}", result);
    }
}
