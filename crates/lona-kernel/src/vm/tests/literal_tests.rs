// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for literal value loading.

use lona_core::chunk::Constant;
use lona_core::integer::Integer;
use lona_core::opcode::{Opcode, encode_abc, encode_abx};
use lona_core::span::Span;
use lona_core::symbol::Interner;
use lona_core::value::Value;

use super::{make_chunk, make_vm};

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

#[test]
fn execute_empty_chunk_returns_nil() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);
    let chunk = make_chunk();

    let result = vm.execute(&chunk).unwrap();
    assert_eq!(result, Value::Nil);
}

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
