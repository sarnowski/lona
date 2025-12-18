// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! `Diagnostic` implementation for compiler errors.
//!
//! This module provides the bridge between compiler error types and the
//! diagnostic formatting system, enabling Rust-style error messages for
//! compilation errors.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use lona_core::source::Location as SourceLocation;
use lona_core::symbol::Interner;
use lonala_compiler::error::{Error, Kind};

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
    fn message(&self, _interner: &Interner) -> String {
        match self.kind {
            Kind::TooManyConstants => String::from("too many constants in chunk (maximum 65535)"),
            Kind::TooManyRegisters => String::from("too many registers needed (maximum 255)"),
            Kind::JumpTooLarge => String::from("jump offset too large"),
            Kind::EmptyCall => String::from("empty list cannot be called as function"),
            Kind::NotImplemented { feature } => {
                format!("not implemented: {feature}")
            }
            Kind::InvalidSpecialForm { form, message } => {
                format!("invalid '{form}' form: {message}")
            }
            Kind::InvalidMacroResult { ref message } => {
                format!("invalid macro result: {message}")
            }
            Kind::MacroExpansionFailed { ref message } => {
                format!("macro expansion failed: {message}")
            }
            Kind::MacroExpansionDepthExceeded { depth } => {
                format!("macro expansion exceeded maximum depth ({depth})")
            }
            // Non-exhaustive pattern: future variants
            _ => String::from("compilation error"),
        }
    }

    #[inline]
    fn notes(&self, _interner: &Interner) -> Vec<Note> {
        let mut notes = Vec::new();

        match self.kind {
            Kind::TooManyConstants => {
                notes.push(Note::text_static(
                    "each chunk can hold at most 65536 constants",
                ));
                notes.push(Note::help_static("split the code into smaller functions"));
            }
            Kind::TooManyRegisters => {
                notes.push(Note::text_static(
                    "function requires more than 256 local variables or temporaries",
                ));
                notes.push(Note::help_static(
                    "simplify the function or break it into smaller pieces",
                ));
            }
            Kind::JumpTooLarge => {
                notes.push(Note::text_static(
                    "a conditional or loop body is too large to encode",
                ));
                notes.push(Note::help_static("break the code into smaller functions"));
            }
            Kind::EmptyCall => {
                notes.push(Note::text_static("() is not a valid function call"));
                notes.push(Note::help_static(
                    "use nil for an empty value, or call a function like (identity nil)",
                ));
            }
            Kind::NotImplemented { .. } => {
                notes.push(Note::text_static(
                    "this feature is planned but not yet available",
                ));
            }
            Kind::InvalidSpecialForm { form, .. } => {
                // Add syntax hints for common special forms
                let hint = match form {
                    "if" => Some("syntax: (if test then) or (if test then else)"),
                    "def" => Some("syntax: (def name value)"),
                    "let" => Some("syntax: (let [bindings...] body...)"),
                    "fn" => Some("syntax: (fn [params...] body...)"),
                    "defn" => Some("syntax: (defn name [params...] body...)"),
                    "do" => Some("syntax: (do expr1 expr2 ...)"),
                    "quote" => Some("syntax: (quote expr) or 'expr"),
                    "defmacro" => Some("syntax: (defmacro name [params...] body...)"),
                    _ => None,
                };
                if let Some(syntax) = hint {
                    notes.push(Note::text_static(syntax));
                }
            }
            Kind::InvalidMacroResult { .. } => {
                notes.push(Note::text_static(
                    "macros must return values that can be converted back to AST",
                ));
                notes.push(Note::help_static(
                    "ensure the macro returns lists, vectors, symbols, or literals",
                ));
            }
            Kind::MacroExpansionFailed { .. } => {
                notes.push(Note::text_static(
                    "the macro transformer function threw an error",
                ));
            }
            Kind::MacroExpansionDepthExceeded { .. } => {
                notes.push(Note::text_static(
                    "this typically indicates infinite macro recursion",
                ));
                notes.push(Note::help_static(
                    "check if the macro expands to code that calls itself",
                ));
            }
            // Non-exhaustive pattern: future variants
            _ => {}
        }

        notes
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::string::ToString;

    use lona_core::source::Id as SourceId;
    use lona_core::span::Span;

    use super::*;

    /// Helper to create a test location.
    fn test_location() -> SourceLocation {
        SourceLocation::new(SourceId::new(0_u32), Span::new(0_usize, 1_usize))
    }

    #[test]
    fn too_many_constants_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::TooManyConstants, test_location());
        assert_eq!(
            error.message(&interner),
            "too many constants in chunk (maximum 65535)"
        );
    }

    #[test]
    fn too_many_registers_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::TooManyRegisters, test_location());
        assert_eq!(
            error.message(&interner),
            "too many registers needed (maximum 255)"
        );
    }

    #[test]
    fn jump_too_large_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::JumpTooLarge, test_location());
        assert_eq!(error.message(&interner), "jump offset too large");
    }

    #[test]
    fn empty_call_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::EmptyCall, test_location());
        assert_eq!(
            error.message(&interner),
            "empty list cannot be called as function"
        );
    }

    #[test]
    fn not_implemented_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::NotImplemented {
                feature: "closures",
            },
            test_location(),
        );
        assert_eq!(error.message(&interner), "not implemented: closures");
    }

    #[test]
    fn invalid_special_form_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::InvalidSpecialForm {
                form: "if",
                message: "missing then branch",
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "invalid 'if' form: missing then branch"
        );
    }

    #[test]
    fn invalid_macro_result_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::InvalidMacroResult {
                message: "function cannot be converted to AST".to_string(),
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "invalid macro result: function cannot be converted to AST"
        );
    }

    #[test]
    fn macro_expansion_failed_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::MacroExpansionFailed {
                message: "division by zero".to_string(),
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "macro expansion failed: division by zero"
        );
    }

    #[test]
    fn macro_expansion_depth_exceeded_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::MacroExpansionDepthExceeded { depth: 256_usize },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "macro expansion exceeded maximum depth (256)"
        );
    }

    #[test]
    fn too_many_constants_notes() {
        let interner = Interner::new();
        let error = Error::new(Kind::TooManyConstants, test_location());
        let notes = error.notes(&interner);
        assert_eq!(notes.len(), 2_usize);
        assert!(matches!(&notes[0], Note::Text(msg) if msg.contains("65536")));
    }

    #[test]
    fn invalid_special_form_if_notes() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::InvalidSpecialForm {
                form: "if",
                message: "test",
            },
            test_location(),
        );
        let notes = error.notes(&interner);
        assert!(
            notes
                .iter()
                .any(|note| matches!(note, Note::Text(msg) if msg.contains("if test then")))
        );
    }

    #[test]
    fn macro_expansion_depth_exceeded_notes() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::MacroExpansionDepthExceeded { depth: 256_usize },
            test_location(),
        );
        let notes = error.notes(&interner);
        assert!(
            notes
                .iter()
                .any(|note| matches!(note, Note::Text(msg) if msg.contains("infinite")))
        );
    }

    #[test]
    fn severity_is_error() {
        let error = Error::new(Kind::TooManyConstants, test_location());
        assert_eq!(error.severity(), Severity::Error);
    }

    #[test]
    fn location_accessor() {
        let location = SourceLocation::new(SourceId::new(42_u32), Span::new(10_usize, 20_usize));
        let error = Error::new(Kind::EmptyCall, location);
        assert_eq!(error.location(), location);
    }

    #[test]
    fn variant_name_matches_kind() {
        let error = Error::new(Kind::TooManyConstants, test_location());
        assert_eq!(error.variant_name(), "TooManyConstants");

        let error2 = Error::new(Kind::EmptyCall, test_location());
        assert_eq!(error2.variant_name(), "EmptyCall");
    }
}
