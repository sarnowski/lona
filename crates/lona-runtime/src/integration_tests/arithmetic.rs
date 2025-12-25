// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Arithmetic and primitive integration tests.
//!
//! Tests basic arithmetic operations, comparisons, and primitive types
//! in the full seL4 environment.

use lona_core::integer::Integer;
use lona_core::source;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_kernel::vm::Vm;
use lona_test::Status;
use lonala_compiler::compile;

/// Test source ID for integration tests.
const TEST_SOURCE_ID: source::Id = source::Id::new(0_u32);

/// Tests that the system booted successfully.
///
/// If we reach this code, boot has succeeded (implicit pass).
pub fn test_boot() -> Status {
    // If we're executing this code, boot succeeded
    Status::Pass
}

/// Tests basic arithmetic: (+ 1 2) should evaluate to 3.
pub fn test_arithmetic() -> Status {
    let mut interner = Interner::new();

    // Compile a simple arithmetic expression
    let chunk = match compile("(+ 1 2)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    // Execute it
    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(3) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests subtraction: (- 10 3) should evaluate to 7.
pub fn test_subtraction() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(- 10 3)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(7) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests multiplication: (* 6 7) should evaluate to 42.
pub fn test_multiplication() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(* 6 7)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(42) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests comparison: (< 1 2) should evaluate to true.
pub fn test_comparison() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(< 1 2)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Bool(result)) if result => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests boolean not: (not false) should evaluate to true.
pub fn test_boolean_not() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(not false)", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Bool(result)) if result => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests nested expression: (+ (* 2 3) (- 10 5)) should evaluate to 11.
pub fn test_nested_expression() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(+ (* 2 3) (- 10 5))", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(11) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests string literal: "hello" should evaluate to a string value.
pub fn test_string_literal() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("\"hello\"", TEST_SOURCE_ID, &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::String(ref string)) if string.as_str() == "hello" => Status::Pass,
        _ => Status::Fail,
    }
}
