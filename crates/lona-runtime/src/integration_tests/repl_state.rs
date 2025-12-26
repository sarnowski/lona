// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! REPL persistent state integration tests.
//!
//! Tests that state persists correctly across REPL evaluations,
//! including global definitions and error handling.

use crate::{println, repl};
use lona_core::integer::Integer;
use lona_core::source;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_kernel::vm::{Globals, Vm};
use lona_test::Status;
use lonala_compiler::compile;

/// Test source ID for integration tests.
const TEST_SOURCE_ID: source::Id = source::Id::new(0_u32);

/// Helper to evaluate source with persistent state (like the REPL does).
fn eval_with_state(source: &str, interner: &Interner, globals: &mut Globals) -> Result<Value, ()> {
    let chunk = compile(source, TEST_SOURCE_ID, interner).map_err(|err| {
        println!("Compile error: {err:?}");
    })?;

    let mut vm = Vm::new(interner);
    *vm.globals_mut() = globals.clone();

    let result = vm.execute(&chunk).map_err(|err| {
        println!("Runtime error: {err:?}");
    })?;

    *globals = vm.globals().clone();

    Ok(result)
}

/// Tests that def persists across evaluations.
pub fn test_repl_def_persist() -> Status {
    let interner = Interner::new();
    let mut globals = Globals::new();

    // First evaluation: define x
    if eval_with_state("(def x 42)", &interner, &mut globals).is_err() {
        return Status::Fail;
    }

    // Second evaluation: use x
    match eval_with_state("x", &interner, &mut globals) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(42) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests the exact failing scenario: def x, then use x in if condition.
pub fn test_repl_def_use_in_if() -> Status {
    let interner = Interner::new();
    let mut globals = Globals::new();

    // First evaluation: define x
    if eval_with_state("(def x 42)", &interner, &mut globals).is_err() {
        return Status::Fail;
    }

    // Second evaluation: use x in if condition
    match eval_with_state("(if (> x 10) \"big\" \"small\")", &interner, &mut globals) {
        Ok(Value::String(ref s)) if s.as_str() == "big" => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests using the core Repl struct - this is the real end-to-end test.
///
/// Uses `Repl::eval()` which is the same core function used by the interactive console.
pub fn test_actual_repl_def_use_in_if() -> Status {
    let mut repl_instance = repl::Repl::new();

    // First evaluation: define x
    if repl_instance.eval("(def x 42)").is_err() {
        return Status::Fail;
    }

    // Second evaluation: use x in if condition
    match repl_instance.eval("(if (> x 10) \"big\" \"small\")") {
        Ok(Value::String(ref s)) if s.as_str() == "big" => Status::Pass,
        other => {
            // Print debug info
            println!("test_actual_repl_def_use_in_if failed: {other:?}");
            Status::Fail
        }
    }
}

/// Tests that incomplete input returns an appropriate error.
///
/// Incomplete input (unbalanced parentheses) should result in a parse error,
/// not a crash or hang.
pub fn test_incomplete_input_error() -> Status {
    let mut repl_instance = repl::Repl::new();

    // This input has unbalanced parentheses - it's incomplete
    match repl_instance.eval("(def x") {
        // lonala-human formats parse errors as "error[VariantName]: ..."
        Err(ref msg) if msg.contains("error[") => Status::Pass,
        Ok(value) => {
            println!("test_incomplete_input_error: expected error, got value: {value:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_incomplete_input_error: unexpected error format: {msg}");
            Status::Fail
        }
    }
}
