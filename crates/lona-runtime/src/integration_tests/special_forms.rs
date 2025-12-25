// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Special form integration tests.
//!
//! Tests for do, if, and def special forms in the full seL4 environment.

use lona_core::integer::Integer;
use lona_core::source;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_kernel::vm::Vm;
use lona_test::Status;
use lonala_compiler::compile;

/// Test source ID for integration tests.
const TEST_SOURCE_ID: source::Id = source::Id::new(0_u32);

/// Tests empty do: (do) should return nil.
pub fn test_do_empty() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(do)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Nil) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests single do: (do 42) should return 42.
pub fn test_do_single() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(do 42)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(42) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests multiple do: (do 1 2 3) should return 3.
pub fn test_do_multiple() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(do 1 2 3)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(3) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests if true branch: (if true 1 2) should return 1.
pub fn test_if_true() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(if true 1 2)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(1) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests if false branch: (if false 1 2) should return 2.
pub fn test_if_false() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(if false 1 2)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(2) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests if without else: (if false 1) should return nil.
pub fn test_if_no_else() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(if false 1)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Nil) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests simple def: (def x 42) should define x and return symbol.
pub fn test_def_simple() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(do (def x 42) x)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(42) => Status::Pass,
        _ => Status::Fail,
    }
}
