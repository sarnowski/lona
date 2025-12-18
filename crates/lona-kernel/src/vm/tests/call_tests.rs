// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for function calls.

use lona_core::chunk::Constant;
use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::integer::Integer;
use lona_core::opcode::{Opcode, encode_abc, encode_abx};
use lona_core::span::Span;
use lona_core::symbol::Interner;
use lona_core::value::{self, Value};

use super::{make_chunk, make_vm};
use crate::vm::NativeError;
use crate::vm::error::{Error, Kind as ErrorKind};
use crate::vm::interpreter::Vm;
use crate::vm::natives::NativeContext;

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
    fn native_double(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
        if args.len() != 1_usize {
            return Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(1_u8),
                got: u8::try_from(args.len()).unwrap_or(u8::MAX),
            });
        }
        let num = args
            .first()
            .and_then(Value::as_integer)
            .ok_or(NativeError::TypeError {
                expected: TypeExpectation::Single(value::Kind::Integer),
                got: args.first().map_or(value::Kind::Nil, Value::kind),
                arg_index: 0_u8,
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
    assert!(matches!(
        result,
        Err(Error {
            kind: ErrorKind::UndefinedFunction { .. },
            ..
        })
    ));
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
    assert!(matches!(
        result,
        Err(Error {
            kind: ErrorKind::NotCallable { .. },
            ..
        })
    ));
}
