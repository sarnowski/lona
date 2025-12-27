// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Formatting helpers for namespace-related errors.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use lona_kernel::vm::ErrorKind as Kind;

use crate::diagnostic::Note;

/// Formats a compile error that occurred during namespace loading.
///
/// Uses the error type's `variant_name()` method to produce human-readable
/// error codes, avoiding Debug formatting for user-facing messages.
pub fn format_compile_error(error: &lonala_compiler::CompileError) -> String {
    use lonala_compiler::CompileError;
    match *error {
        CompileError::Parse(ref parse_err) => {
            format!(
                "parse error in loaded namespace: {}",
                parse_err.kind.variant_name()
            )
        }
        CompileError::Compile(ref compile_err) => {
            format!(
                "compile error in loaded namespace: {}",
                compile_err.kind.variant_name()
            )
        }
        // Non-exhaustive pattern
        _ => String::from("compilation error during namespace loading"),
    }
}

/// Adds notes for namespace-related and case matching errors.
pub fn add_namespace_and_case_notes(notes: &mut Vec<Note>, kind: &Kind) {
    match *kind {
        Kind::NoMatchingCase { .. } => {
            notes.push(Note::help_static(
                "add an :else clause to handle unmatched values",
            ));
        }
        Kind::CircularDependency { .. } => {
            notes.push(Note::text_static(
                "namespaces cannot require each other in a cycle",
            ));
            notes.push(Note::help_static(
                "reorganize dependencies to break the cycle",
            ));
        }
        Kind::NamespaceNotFound { .. } => {
            notes.push(Note::help_static(
                "check the namespace name and source file exist",
            ));
        }
        Kind::NoSourceLoader => {
            notes.push(Note::text_static("the VM has no source loader configured"));
            notes.push(Note::help_static(
                "configure SourceLoader with Vm::set_loader",
            ));
        }
        // Other variants handled by main notes() match
        Kind::InvalidOpcode { .. }
        | Kind::UndefinedGlobal { .. }
        | Kind::UndefinedFunction { .. }
        | Kind::TypeError { .. }
        | Kind::DivisionByZero
        | Kind::StackOverflow { .. }
        | Kind::NotCallable { .. }
        | Kind::InvalidConstant { .. }
        | Kind::InvalidRegister { .. }
        | Kind::Native { .. }
        | Kind::ArityMismatch { .. }
        | Kind::InvalidUpvalue { .. }
        | Kind::NotImplemented { .. }
        | Kind::CompileError { .. }
        | _ => {}
    }
}
