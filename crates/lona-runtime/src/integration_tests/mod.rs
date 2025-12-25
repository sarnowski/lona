// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Integration tests for the Lona runtime.
//!
//! These tests run in QEMU when the `integration-test` feature is enabled.
//! They validate the full stack from source code to execution results.
//!
//! ## Submodules
//!
//! - [`arithmetic`] - Basic arithmetic and primitive tests
//! - [`special_forms`] - Special form tests (do, if, def)
//! - [`repl_state`] - REPL persistent state tests
//! - [`macros`] - Macro introspection tests
//! - [`errors`] - Human-readable error formatting tests

mod arithmetic;
mod errors;
mod macros;
mod repl_state;
mod special_forms;

use crate::{print, println};
use lona_test::{Status, Test, run_tests};

/// Runs integration tests and outputs results via UART.
///
/// Tests are executed when the `integration-test` feature is enabled.
/// Results are output in a structured format for the test harness to parse.
pub fn run_integration_tests() {
    println!("Running integration tests...");

    let tests = [
        // Arithmetic tests
        Test::new("boot", arithmetic::test_boot),
        Test::new("arithmetic", arithmetic::test_arithmetic),
        Test::new("subtraction", arithmetic::test_subtraction),
        Test::new("multiplication", arithmetic::test_multiplication),
        Test::new("comparison", arithmetic::test_comparison),
        Test::new("boolean_not", arithmetic::test_boolean_not),
        Test::new("nested_expr", arithmetic::test_nested_expression),
        Test::new("string_literal", arithmetic::test_string_literal),
        // Special form tests
        Test::new("do_empty", special_forms::test_do_empty),
        Test::new("do_single", special_forms::test_do_single),
        Test::new("do_multiple", special_forms::test_do_multiple),
        Test::new("if_true", special_forms::test_if_true),
        Test::new("if_false", special_forms::test_if_false),
        Test::new("if_no_else", special_forms::test_if_no_else),
        Test::new("def_simple", special_forms::test_def_simple),
        // REPL-like persistent state tests
        Test::new("repl_def_persist", repl_state::test_repl_def_persist),
        Test::new("repl_def_use_in_if", repl_state::test_repl_def_use_in_if),
        // Test using actual Repl struct
        Test::new(
            "actual_repl_test",
            repl_state::test_actual_repl_def_use_in_if,
        ),
        // Error handling test
        Test::new("incomplete_input", repl_state::test_incomplete_input_error),
        // Macro introspection tests
        Test::new("macro_predicate_true", macros::test_macro_predicate_true),
        Test::new("macro_predicate_false", macros::test_macro_predicate_false),
        Test::new("macroexpand_1", macros::test_macroexpand_1),
        Test::new(
            "macroexpand_1_non_macro",
            macros::test_macroexpand_1_non_macro,
        ),
        Test::new("macroexpand", macros::test_macroexpand),
        // Human-readable error formatting tests
        Test::new(
            "vm_error_format",
            errors::test_vm_error_human_readable_format,
        ),
        Test::new(
            "vm_error_source_id",
            errors::test_vm_error_shows_correct_source,
        ),
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
