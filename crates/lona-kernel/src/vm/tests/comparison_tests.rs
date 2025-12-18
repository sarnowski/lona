// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for comparison operations.

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abc, rk_constant};
use lona_core::span::Span;
use lona_core::symbol::Interner;
use lona_core::value::Value;

use super::{make_chunk, make_vm};

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
