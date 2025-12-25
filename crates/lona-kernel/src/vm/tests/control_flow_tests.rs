// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for control flow and global variables.

use lona_core::chunk::Constant;
use lona_core::integer::Integer;
use lona_core::opcode::{Opcode, encode_abc, encode_abx, encode_asbx};
use lona_core::span::Span;
use lona_core::symbol::Interner;
use lona_core::value::Value;

use super::{make_chunk, make_vm};
use crate::vm::error::{Error, Kind as ErrorKind};

#[test]
fn execute_set_and_get_global() {
    let interner = Interner::new();
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
    let interner = Interner::new();
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
    assert!(matches!(
        result,
        Err(Error {
            kind: ErrorKind::UndefinedGlobal { .. },
            ..
        })
    ));
}

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
