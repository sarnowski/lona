// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! `Diagnostic` implementation for VM runtime errors.
//!
//! This module provides the bridge between VM error types and the
//! diagnostic formatting system, enabling Rust-style error messages for
//! runtime errors during bytecode execution.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use lona_core::error_context::ArityExpectation;
use lona_core::source::Location as SourceLocation;
use lona_core::symbol::Interner;
use lona_kernel::vm::{Error, ErrorKind as Kind, NativeError};

use crate::diagnostic::{Diagnostic, Note, Severity};

impl Diagnostic for Error {
    #[inline]
    fn location(&self) -> SourceLocation {
        self.location
    }

    #[inline]
    fn severity(&self) -> Severity {
        Severity::Error
    }

    #[inline]
    fn variant_name(&self) -> &'static str {
        self.kind.variant_name()
    }

    #[inline]
    fn message(&self, interner: &Interner) -> String {
        match self.kind {
            Kind::InvalidOpcode { byte, pc } => {
                format!("invalid opcode 0x{byte:02X} at position {pc}")
            }
            Kind::UndefinedGlobal { symbol, .. } => {
                let name = interner.resolve(symbol);
                format!("undefined symbol '{name}'")
            }
            Kind::UndefinedFunction { symbol, .. } => {
                let name = interner.resolve(symbol);
                format!("undefined function '{name}'")
            }
            Kind::TypeError {
                operation,
                ref expected,
                got,
                ..
            } => {
                format!(
                    "type error in '{}': expected {}, got {}",
                    operation,
                    expected.description(),
                    got.name()
                )
            }
            Kind::DivisionByZero => String::from("division by zero"),
            Kind::StackOverflow { max_depth } => {
                format!("stack overflow (maximum depth: {max_depth})")
            }
            Kind::NotCallable { got } => {
                format!("cannot call value of type {}", got.name())
            }
            Kind::InvalidConstant { index } => {
                format!("invalid constant index {index}")
            }
            Kind::InvalidRegister { index } => {
                format!("invalid register index {index}")
            }
            Kind::Native { ref error } => format_native_error(error),
            Kind::ArityMismatch {
                callable,
                expected,
                got,
            } => {
                let expected_str = format_arity(expected);
                callable.map_or_else(
                    || {
                        format!(
                            "function called with wrong number of arguments: expected {expected_str}, got {got}"
                        )
                    },
                    |sym| {
                        let name = interner.resolve(sym);
                        format!(
                            "function '{name}' called with wrong number of arguments: expected {expected_str}, got {got}"
                        )
                    },
                )
            }
            Kind::InvalidUpvalue { index, available } => {
                format!(
                    "closure captured variable out of bounds (index {index}, closure has {available} captured values)"
                )
            }
            Kind::NotImplemented { feature } => {
                format!("feature not yet implemented: {feature}")
            }
            Kind::NoMatchingCase { value_type } => {
                format!(
                    "no matching clause in case expression for value of type {}",
                    value_type.name()
                )
            }
            // Non-exhaustive pattern: future variants
            _ => String::from("runtime error"),
        }
    }

    #[inline]
    fn notes(&self, interner: &Interner) -> Vec<Note> {
        let mut notes = Vec::new();

        match self.kind {
            Kind::InvalidOpcode { .. }
            | Kind::InvalidConstant { .. }
            | Kind::InvalidRegister { .. }
            | Kind::InvalidUpvalue { .. } => {
                notes.push(Note::text_static(
                    "this may indicate a corrupted bytecode chunk",
                ));
            }
            Kind::UndefinedGlobal {
                suggestion: Some(sugg_id),
                ..
            }
            | Kind::UndefinedFunction {
                suggestion: Some(sugg_id),
                ..
            } => {
                let name = interner.resolve(sugg_id);
                notes.push(Note::help(format!("did you mean '{name}'?")));
            }
            Kind::TypeError {
                operation,
                operand: Some(idx),
                ..
            } => {
                let ordinal = match idx {
                    0_u8 => "first",
                    1_u8 => "second",
                    2_u8 => "third",
                    _ => "an",
                };
                notes.push(Note::text(format!(
                    "the {ordinal} argument to '{operation}' has the wrong type"
                )));
            }
            Kind::DivisionByZero => {
                notes.push(Note::text_static("division or modulo by zero is undefined"));
            }
            Kind::StackOverflow { .. } => {
                notes.push(Note::text_static(
                    "this typically indicates infinite recursion",
                ));
                notes.push(Note::help_static(
                    "check for functions that call themselves without a base case",
                ));
            }
            Kind::NotCallable { got } => {
                notes.push(Note::text(format!(
                    "only functions can be called, but found {}",
                    got.name()
                )));
                notes.push(Note::help_static(
                    "ensure the first element of the list is a function",
                ));
            }
            Kind::Native { ref error } => {
                add_native_notes(&mut notes, error);
            }
            Kind::ArityMismatch {
                expected: ArityExpectation::AtLeast(min),
                ..
            } => {
                notes.push(Note::text(format!(
                    "this function requires at least {min} argument(s)"
                )));
            }
            Kind::ArityMismatch {
                expected: ArityExpectation::Range { min, max },
                ..
            } => {
                notes.push(Note::text(format!(
                    "this function accepts {min} to {max} arguments"
                )));
            }
            Kind::NoMatchingCase { .. } => {
                notes.push(Note::help_static(
                    "add an :else clause to handle unmatched values",
                ));
            }
            // No extra notes for: suggestion-less undefined symbols, operand-less type errors,
            // exact arity (message says the count), and future variants
            Kind::UndefinedGlobal {
                suggestion: None, ..
            }
            | Kind::UndefinedFunction {
                suggestion: None, ..
            }
            | Kind::TypeError { operand: None, .. }
            | Kind::ArityMismatch {
                expected: ArityExpectation::Exact(_),
                ..
            }
            | _ => {}
        }

        notes
    }
}

/// Formats a native function error message.
#[inline]
fn format_native_error(error: &NativeError) -> String {
    match *error {
        NativeError::ArityMismatch { expected, got, .. } => {
            let expected_str = format_arity(expected);
            format!(
                "native function called with wrong number of arguments: expected {expected_str}, got {got}"
            )
        }
        NativeError::TypeError {
            ref expected,
            got,
            arg_index,
        } => {
            let ordinal = match arg_index {
                0_u8 => "first",
                1_u8 => "second",
                2_u8 => "third",
                _ => "an",
            };
            format!(
                "type error in native function: {ordinal} argument expected {}, got {}",
                expected.description(),
                got.name()
            )
        }
        NativeError::DivisionByZero => String::from("division by zero"),
        NativeError::Error(msg) => {
            format!("native function error: {msg}")
        }
        // Non-exhaustive pattern
        _ => String::from("native function error"),
    }
}

/// Formats arity expectation for error messages.
#[inline]
fn format_arity(expected: ArityExpectation) -> String {
    match expected {
        ArityExpectation::Exact(num) => format!("{num}"),
        ArityExpectation::AtLeast(min) => format!("at least {min}"),
        ArityExpectation::Range { min, max } => format!("{min} to {max}"),
        // Non-exhaustive pattern
        _ => String::from("unknown"),
    }
}

/// Adds notes for native function errors.
#[inline]
fn add_native_notes(notes: &mut Vec<Note>, error: &NativeError) {
    match *error {
        NativeError::ArityMismatch {
            expected: ArityExpectation::AtLeast(min),
            ..
        } => {
            notes.push(Note::text(format!(
                "this native function requires at least {min} argument(s)"
            )));
        }
        NativeError::ArityMismatch {
            expected: ArityExpectation::Range { min, max },
            ..
        } => {
            notes.push(Note::text(format!(
                "this native function accepts {min} to {max} arguments"
            )));
        }
        NativeError::TypeError { arg_index, .. } => {
            let position = arg_index.saturating_add(1_u8);
            notes.push(Note::text(format!(
                "argument {position} has the wrong type"
            )));
        }
        NativeError::DivisionByZero => {
            notes.push(Note::text_static("division or modulo by zero is undefined"));
        }
        // No extra note needed for Exact arity, generic errors, and future variants
        NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(_),
            ..
        }
        | NativeError::Error(_)
        | _ => {}
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use lona_core::error_context::TypeExpectation;
    use lona_core::source::Id as SourceId;
    use lona_core::span::Span;
    use lona_core::symbol;
    use lona_core::value;

    use super::*;

    /// Helper to create a test location.
    fn test_location() -> SourceLocation {
        SourceLocation::new(SourceId::new(0_u32), Span::new(0_usize, 1_usize))
    }

    #[test]
    fn invalid_opcode_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::InvalidOpcode {
                byte: 0xFF_u8,
                pc: 42_usize,
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "invalid opcode 0xFF at position 42"
        );
    }

    #[test]
    fn undefined_global_message() {
        let interner = Interner::new();
        let sym = interner.intern("foo");
        let error = Error::new(
            Kind::UndefinedGlobal {
                symbol: sym,
                suggestion: None,
            },
            test_location(),
        );
        assert_eq!(error.message(&interner), "undefined symbol 'foo'");
    }

    #[test]
    fn undefined_global_with_suggestion() {
        let interner = Interner::new();
        let sym = interner.intern("fooo");
        let suggestion = interner.intern("foo");
        let error = Error::new(
            Kind::UndefinedGlobal {
                symbol: sym,
                suggestion: Some(suggestion),
            },
            test_location(),
        );
        let notes = error.notes(&interner);
        assert!(
            notes
                .iter()
                .any(|note| matches!(note, Note::Help(msg) if msg.contains("did you mean 'foo'")))
        );
    }

    #[test]
    fn type_error_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::TypeError {
                operation: "+",
                expected: TypeExpectation::Numeric,
                got: value::Kind::String,
                operand: Some(0_u8),
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "type error in '+': expected numeric type, got string"
        );
    }

    #[test]
    fn type_error_operand_note() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::TypeError {
                operation: "-",
                expected: TypeExpectation::Numeric,
                got: value::Kind::Bool,
                operand: Some(1_u8),
            },
            test_location(),
        );
        let notes = error.notes(&interner);
        assert!(
            notes
                .iter()
                .any(|note| matches!(note, Note::Text(msg) if msg.contains("second argument")))
        );
    }

    #[test]
    fn division_by_zero_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::DivisionByZero, test_location());
        assert_eq!(error.message(&interner), "division by zero");
    }

    #[test]
    fn stack_overflow_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::StackOverflow {
                max_depth: 256_usize,
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "stack overflow (maximum depth: 256)"
        );
    }

    #[test]
    fn stack_overflow_notes() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::StackOverflow {
                max_depth: 256_usize,
            },
            test_location(),
        );
        let notes = error.notes(&interner);
        assert!(
            notes
                .iter()
                .any(|note| matches!(note, Note::Text(msg) if msg.contains("recursion")))
        );
        assert!(
            notes
                .iter()
                .any(|note| matches!(note, Note::Help(msg) if msg.contains("base case")))
        );
    }

    #[test]
    fn not_callable_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::NotCallable {
                got: value::Kind::Integer,
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "cannot call value of type integer"
        );
    }

    #[test]
    fn arity_mismatch_message_with_name() {
        let interner = Interner::new();
        let sym = interner.intern("add");
        let error = Error::new(
            Kind::ArityMismatch {
                callable: Some(sym),
                expected: ArityExpectation::Exact(2_u8),
                got: 3_u8,
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "function 'add' called with wrong number of arguments: expected 2, got 3"
        );
    }

    #[test]
    fn arity_mismatch_message_without_name() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::ArityMismatch {
                callable: None,
                expected: ArityExpectation::AtLeast(1_u8),
                got: 0_u8,
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "function called with wrong number of arguments: expected at least 1, got 0"
        );
    }

    #[test]
    fn native_arity_error_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::Native {
                error: NativeError::ArityMismatch {
                    expected: ArityExpectation::Exact(2_u8),
                    got: 1_u8,
                },
            },
            test_location(),
        );
        assert!(error.message(&interner).contains("expected 2, got 1"));
    }

    #[test]
    fn native_type_error_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::Native {
                error: NativeError::TypeError {
                    expected: TypeExpectation::Numeric,
                    got: value::Kind::Bool,
                    arg_index: 0_u8,
                },
            },
            test_location(),
        );
        assert!(error.message(&interner).contains("first argument"));
        assert!(error.message(&interner).contains("numeric type"));
    }

    #[test]
    fn native_generic_error_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::Native {
                error: NativeError::Error("test error"),
            },
            test_location(),
        );
        assert!(error.message(&interner).contains("test error"));
    }

    #[test]
    fn severity_is_error() {
        let error = Error::new(Kind::DivisionByZero, test_location());
        assert_eq!(error.severity(), Severity::Error);
    }

    #[test]
    fn location_accessor() {
        let location = SourceLocation::new(SourceId::new(42_u32), Span::new(10_usize, 20_usize));
        let error = Error::new(Kind::DivisionByZero, location);
        assert_eq!(error.location(), location);
    }

    #[test]
    fn variant_name_matches_kind() {
        let error = Error::new(Kind::DivisionByZero, test_location());
        assert_eq!(error.variant_name(), "DivisionByZero");

        let error2 = Error::new(
            Kind::UndefinedGlobal {
                symbol: symbol::Id::new(0_u32),
                suggestion: None,
            },
            test_location(),
        );
        assert_eq!(error2.variant_name(), "UndefinedGlobal");
    }
}
