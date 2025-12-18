// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for arithmetic operations.

use lona_core::chunk::Constant;
use lona_core::integer::Integer;
use lona_core::opcode::{Opcode, encode_abc, encode_abx, rk_constant};
use lona_core::span::Span;
use lona_core::symbol::Interner;
use lona_core::value::Value;

use super::{make_chunk, make_vm};
use crate::vm::error::{Error, Kind as ErrorKind};

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
    assert!(matches!(
        result,
        Err(Error {
            kind: ErrorKind::DivisionByZero,
            ..
        })
    ));
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
    assert!(matches!(
        result,
        Err(Error {
            kind: ErrorKind::DivisionByZero,
            ..
        })
    ));
}

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
    assert!(matches!(
        result,
        Err(Error {
            kind: ErrorKind::TypeError { .. },
            ..
        })
    ));
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
    assert!(matches!(
        result,
        Err(Error {
            kind: ErrorKind::TypeError { .. },
            ..
        })
    ));
}
