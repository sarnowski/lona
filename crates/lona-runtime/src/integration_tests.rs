// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Integration tests for the Lona runtime.
//!
//! These tests run in QEMU when the `integration-test` feature is enabled.
//! They validate the full stack from source code to execution results.

use crate::{print, println};
use lona_core::integer::Integer;
use lona_core::source;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_kernel::vm::{Globals, Vm};
use lona_test::{Status, Test, run_tests};
use lonala_compiler::compile;

use crate::repl;

/// Test source ID for integration tests.
const TEST_SOURCE_ID: source::Id = source::Id::new(0_u32);

/// Runs integration tests and outputs results via UART.
///
/// Tests are executed when the `integration-test` feature is enabled.
/// Results are output in a structured format for the test harness to parse.
pub fn run_integration_tests() {
    println!("Running integration tests...");

    let tests = [
        Test::new("boot", test_boot),
        Test::new("arithmetic", test_arithmetic),
        Test::new("subtraction", test_subtraction),
        Test::new("multiplication", test_multiplication),
        Test::new("comparison", test_comparison),
        Test::new("boolean_not", test_boolean_not),
        Test::new("nested_expr", test_nested_expression),
        Test::new("string_literal", test_string_literal),
        // Special form tests
        Test::new("do_empty", test_do_empty),
        Test::new("do_single", test_do_single),
        Test::new("do_multiple", test_do_multiple),
        Test::new("if_true", test_if_true),
        Test::new("if_false", test_if_false),
        Test::new("if_no_else", test_if_no_else),
        Test::new("def_simple", test_def_simple),
        // REPL-like persistent state tests
        Test::new("repl_def_persist", test_repl_def_persist),
        Test::new("repl_def_use_in_if", test_repl_def_use_in_if),
        // Test using actual Repl struct
        Test::new("actual_repl_test", test_actual_repl_def_use_in_if),
        // Error handling test
        Test::new("incomplete_input", test_incomplete_input_error),
        // Macro introspection tests
        Test::new("macro_predicate_true", test_macro_predicate_true),
        Test::new("macro_predicate_false", test_macro_predicate_false),
        Test::new("macroexpand_1", test_macroexpand_1),
        Test::new("macroexpand_1_non_macro", test_macroexpand_1_non_macro),
        Test::new("macroexpand", test_macroexpand),
        // Human-readable error formatting tests
        Test::new("vm_error_format", test_vm_error_human_readable_format),
        Test::new("vm_error_source_id", test_vm_error_shows_correct_source),
    ];

    let status = run_tests(&tests, |s| print!("{s}"));

    // Report final status
    println!(
        "Integration tests {}",
        if status == Status::Pass {
            "PASSED"
        } else {
            "FAILED"
        }
    );
}

/// Halts the system in a low-power loop.
///
/// Used after integration tests complete. The loop never exits.
pub fn halt_loop() -> ! {
    loop {
        // SAFETY: These instructions are safe to execute - they simply
        // put the CPU into a low-power state until an interrupt occurs.
        // WFI = Wait For Interrupt (ARM64), HLT = Halt (x86_64)
        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Tests that the system booted successfully.
///
/// If we reach this code, boot has succeeded (implicit pass).
fn test_boot() -> Status {
    // If we're executing this code, boot succeeded
    Status::Pass
}

/// Tests basic arithmetic: (+ 1 2) should evaluate to 3.
fn test_arithmetic() -> Status {
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
fn test_subtraction() -> Status {
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
fn test_multiplication() -> Status {
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
fn test_comparison() -> Status {
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
fn test_boolean_not() -> Status {
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
fn test_nested_expression() -> Status {
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
fn test_string_literal() -> Status {
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

// =============================================================================
// Special Form Tests
// =============================================================================

/// Tests empty do: (do) should return nil.
fn test_do_empty() -> Status {
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
fn test_do_single() -> Status {
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
fn test_do_multiple() -> Status {
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
fn test_if_true() -> Status {
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
fn test_if_false() -> Status {
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
fn test_if_no_else() -> Status {
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
fn test_def_simple() -> Status {
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

// =============================================================================
// REPL-like Persistent State Tests
// =============================================================================

/// Helper to evaluate source with persistent state (like the REPL does).
fn eval_with_state(
    source: &str,
    interner: &mut Interner,
    globals: &mut Globals,
) -> Result<Value, ()> {
    let chunk = compile(source, TEST_SOURCE_ID, interner).map_err(|err| {
        println!("Compile error: {err}");
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
fn test_repl_def_persist() -> Status {
    let mut interner = Interner::new();
    let mut globals = Globals::new();

    // First evaluation: define x
    if eval_with_state("(def x 42)", &mut interner, &mut globals).is_err() {
        return Status::Fail;
    }

    // Second evaluation: use x
    match eval_with_state("x", &mut interner, &mut globals) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(42) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests the exact failing scenario: def x, then use x in if condition.
fn test_repl_def_use_in_if() -> Status {
    let mut interner = Interner::new();
    let mut globals = Globals::new();

    // First evaluation: define x
    if eval_with_state("(def x 42)", &mut interner, &mut globals).is_err() {
        return Status::Fail;
    }

    // Second evaluation: use x in if condition
    match eval_with_state(
        "(if (> x 10) \"big\" \"small\")",
        &mut interner,
        &mut globals,
    ) {
        Ok(Value::String(ref s)) if s.as_str() == "big" => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests using the core Repl struct - this is the real end-to-end test.
///
/// Uses `Repl::eval()` which is the same core function used by the interactive console.
fn test_actual_repl_def_use_in_if() -> Status {
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
fn test_incomplete_input_error() -> Status {
    let mut repl_instance = repl::Repl::new();

    // This input has unbalanced parentheses - it's incomplete
    match repl_instance.eval("(def x") {
        Err(ref msg) if msg.contains("Compile error") => Status::Pass,
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

/// Tests macro? predicate returns true for defined macros.
fn test_macro_predicate_true() -> Status {
    let mut repl_instance = repl::Repl::new();

    // Define a simple macro
    match repl_instance.eval("(defmacro when [test body] (list 'if test body nil))") {
        Ok(_) => {}
        Err(ref msg) => {
            println!("test_macro_predicate_true: failed to define macro: {msg}");
            return Status::Fail;
        }
    }

    // Check that macro? returns true
    match repl_instance.eval("(macro? 'when)") {
        Ok(Value::Bool(true)) => Status::Pass,
        Ok(ref other) => {
            println!("test_macro_predicate_true: expected true, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macro_predicate_true: error: {msg}");
            Status::Fail
        }
    }
}

/// Tests macro? predicate returns false for non-macros.
fn test_macro_predicate_false() -> Status {
    let mut repl_instance = repl::Repl::new();

    // Check that macro? returns false for undefined symbol
    match repl_instance.eval("(macro? 'not-a-macro)") {
        Ok(Value::Bool(false)) => Status::Pass,
        Ok(ref other) => {
            println!("test_macro_predicate_false: expected false, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macro_predicate_false: error: {msg}");
            Status::Fail
        }
    }
}

/// Tests macroexpand-1 expands a macro call once.
fn test_macroexpand_1() -> Status {
    let mut repl_instance = repl::Repl::new();

    // Define an identity macro that returns its argument
    match repl_instance.eval("(defmacro identity [x] x)") {
        Ok(_) => {}
        Err(ref msg) => {
            println!("test_macroexpand_1: failed to define macro: {msg}");
            return Status::Fail;
        }
    }

    // macroexpand-1 should expand once
    match repl_instance.eval("(macroexpand-1 '(identity 42))") {
        Ok(Value::Integer(ref n)) if n.to_i64() == Some(42_i64) => Status::Pass,
        Ok(ref other) => {
            println!("test_macroexpand_1: expected 42, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macroexpand_1: error: {msg}");
            Status::Fail
        }
    }
}

/// Tests macroexpand-1 returns non-macro form unchanged.
fn test_macroexpand_1_non_macro() -> Status {
    let mut repl_instance = repl::Repl::new();

    // macroexpand-1 on a non-macro form should return it unchanged
    match repl_instance.eval("(macroexpand-1 '(+ 1 2))") {
        Ok(Value::List(ref list)) if list.len() == 3 => Status::Pass,
        Ok(ref other) => {
            println!("test_macroexpand_1_non_macro: expected list, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macroexpand_1_non_macro: error: {msg}");
            Status::Fail
        }
    }
}

/// Tests macroexpand fully expands nested macro calls.
fn test_macroexpand() -> Status {
    let mut repl_instance = repl::Repl::new();

    // Define an identity macro
    match repl_instance.eval("(defmacro pass-through [x] x)") {
        Ok(_) => {}
        Err(ref msg) => {
            println!("test_macroexpand: failed to define macro: {msg}");
            return Status::Fail;
        }
    }

    // macroexpand should fully expand
    match repl_instance.eval("(macroexpand '(pass-through 99))") {
        Ok(Value::Integer(ref n)) if n.to_i64() == Some(99_i64) => Status::Pass,
        Ok(ref other) => {
            println!("test_macroexpand: expected 99, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macroexpand: error: {msg}");
            Status::Fail
        }
    }
}

// =============================================================================
// Human-Readable Error Formatting Tests
// =============================================================================

/// Tests that VM errors are formatted using `lonala_human`, not Debug format.
///
/// This test verifies the fix for the bug where REPL displayed raw Debug output
/// like `Error { kind: UndefinedGlobal { ... } }` instead of human-readable
/// error messages like `error[UndefinedGlobal]: undefined symbol 'foo'`.
fn test_vm_error_human_readable_format() -> Status {
    let mut repl_instance = repl::Repl::new();

    // Try to evaluate an undefined symbol - this should produce a VM error
    match repl_instance.eval("undefined_symbol") {
        Ok(value) => {
            println!("test_vm_error_format: expected error, got value: {value:?}");
            Status::Fail
        }
        Err(ref error_msg) => {
            // The error message should NOT contain Debug format artifacts
            let has_debug_format = error_msg.contains("Error {")
                || error_msg.contains("kind:")
                || error_msg.contains("Id(")
                || error_msg.contains("Span {");

            if has_debug_format {
                println!(
                    "test_vm_error_format: error uses Debug format instead of human-readable:"
                );
                println!("  {error_msg}");
                return Status::Fail;
            }

            // The error message SHOULD contain human-readable format markers
            let has_human_format = error_msg.contains("error[")
                || error_msg.contains("undefined symbol")
                || error_msg.contains("-->");

            if !has_human_format {
                println!("test_vm_error_format: error does not use expected format:");
                println!("  {error_msg}");
                return Status::Fail;
            }

            Status::Pass
        }
    }
}

/// Tests that VM errors show the correct source content for each evaluation.
///
/// This test verifies that when multiple expressions are evaluated, each error
/// shows the source content from its own evaluation, not from a previous one.
fn test_vm_error_shows_correct_source() -> Status {
    let mut repl_instance = repl::Repl::new();

    // First evaluation - a valid expression
    if repl_instance.eval("(+ 1 2)").is_err() {
        println!("test_vm_error_source_id: first eval should succeed");
        return Status::Fail;
    }

    // Second evaluation - an error with different source content
    match repl_instance.eval("(/ 0 0)") {
        Ok(value) => {
            println!("test_vm_error_source_id: expected error, got value: {value:?}");
            Status::Fail
        }
        Err(ref error_msg) => {
            // The error message MUST contain the actual source content "(/ 0 0)"
            // NOT content from a previous evaluation
            if !error_msg.contains("(/ 0 0)") {
                println!("test_vm_error_source_id: error does not show correct source content:");
                println!("  Expected to find: (/ 0 0)");
                println!("  Actual error message:");
                println!("  {error_msg}");
                return Status::Fail;
            }

            // The source name should be <repl:2> since this is the second evaluation
            if !error_msg.contains("<repl:2>") {
                println!("test_vm_error_source_id: error shows wrong source name:");
                println!("  Expected: <repl:2>");
                println!("  Actual error message:");
                println!("  {error_msg}");
                return Status::Fail;
            }

            Status::Pass
        }
    }
}
