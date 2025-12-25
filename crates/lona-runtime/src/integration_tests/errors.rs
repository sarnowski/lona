// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Human-readable error formatting integration tests.
//!
//! Tests that VM errors are formatted using `lonala_human` for user-friendly
//! error messages instead of raw Debug output.

use crate::{println, repl};
use lona_test::Status;

/// Tests that VM errors are formatted using `lonala_human`, not Debug format.
///
/// This test verifies the fix for the bug where REPL displayed raw Debug output
/// like `Error { kind: UndefinedGlobal { ... } }` instead of human-readable
/// error messages like `error[UndefinedGlobal]: undefined symbol 'foo'`.
pub fn test_vm_error_human_readable_format() -> Status {
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
pub fn test_vm_error_shows_correct_source() -> Status {
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
