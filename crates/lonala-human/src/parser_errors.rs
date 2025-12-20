// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! `Diagnostic` implementation for parser errors.
//!
//! This module provides the bridge between parser error types and the
//! diagnostic formatting system, enabling Rust-style error messages for
//! lexer and parser errors.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use lona_core::source::Location as SourceLocation;
use lona_core::symbol::Interner;
use lonala_parser::error::{Error, Kind};

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
            Kind::UnexpectedCharacter(ch) => {
                format!("unexpected character '{ch}'")
            }
            Kind::UnterminatedString => String::from("unterminated string literal"),
            Kind::InvalidEscapeSequence(ch) => {
                format!("invalid escape sequence '\\{ch}'")
            }
            Kind::InvalidNumber => String::from("invalid numeric literal"),
            Kind::InvalidUnicodeEscape => String::from("invalid unicode escape sequence"),
            Kind::UnexpectedToken { expected, found } => {
                format!("unexpected {found}, expected {expected}")
            }
            Kind::UnmatchedDelimiter {
                opener,
                expected,
                found,
                ..
            } => {
                format!("expected '{expected}' to match '{opener}', but found '{found}'")
            }
            Kind::UnexpectedEof { expected } => {
                format!("unexpected end of input, expected {expected}")
            }
            Kind::OddMapEntries => String::from("map literal must have an even number of elements"),
            Kind::DuplicateSetElement => String::from("set literal contains duplicate element"),
            Kind::ReaderMacroMissingExpr => {
                String::from("reader macro must be followed by an expression")
            }
            // Non-exhaustive pattern: future variants
            _ => String::from("parse error"),
        }
    }

    #[inline]
    fn notes(&self, _interner: &Interner) -> Vec<Note> {
        let mut notes = Vec::new();

        match self.kind {
            Kind::UnexpectedCharacter(ch) => {
                if ch.is_ascii_control() {
                    notes.push(Note::text(format!(
                        "character has ASCII code {}",
                        u32::from(ch)
                    )));
                }
            }
            Kind::UnterminatedString => {
                notes.push(Note::text_static(
                    "add a closing '\"' to terminate the string",
                ));
            }
            Kind::InvalidEscapeSequence(ch) => {
                notes.push(Note::text_static(
                    "valid escape sequences are: \\n, \\r, \\t, \\\\, \\\", \\uXXXX",
                ));
                if ch == 'x' {
                    notes.push(Note::help_static(
                        "use \\uXXXX for unicode escapes, not \\xXX",
                    ));
                }
            }
            Kind::InvalidNumber => {
                notes.push(Note::text_static(
                    "numeric literals must be valid integers, floats, or ratios",
                ));
            }
            Kind::InvalidUnicodeEscape => {
                notes.push(Note::text_static(
                    "unicode escapes must have exactly 4 hex digits: \\uXXXX",
                ));
            }
            Kind::UnmatchedDelimiter {
                opener,
                opener_location,
                ..
            } => {
                notes.push(Note::defined_at(
                    format!("'{opener}' opened here"),
                    opener_location,
                ));
            }
            Kind::UnexpectedEof { .. } => {
                notes.push(Note::help_static(
                    "check for unclosed delimiters earlier in the file",
                ));
            }
            Kind::OddMapEntries => {
                notes.push(Note::text_static(
                    "maps are written as {:key1 value1 :key2 value2}",
                ));
                notes.push(Note::help_static(
                    "add a value for the last key, or remove the unpaired key",
                ));
            }
            Kind::ReaderMacroMissingExpr => {
                notes.push(Note::help_static(
                    "reader macros like ' (quote) must be followed by an expression",
                ));
            }
            // No additional notes for these variants (including future ones)
            Kind::UnexpectedToken { .. } | _ => {}
        }

        notes
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use lona_core::source::Id as SourceId;
    use lona_core::span::Span;

    use super::*;

    /// Helper to create a test location.
    fn test_location() -> SourceLocation {
        SourceLocation::new(SourceId::new(0_u32), Span::new(0_usize, 1_usize))
    }

    #[test]
    fn unexpected_character_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::UnexpectedCharacter('@'), test_location());
        assert_eq!(error.message(&interner), "unexpected character '@'");
    }

    #[test]
    fn unterminated_string_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::UnterminatedString, test_location());
        assert_eq!(error.message(&interner), "unterminated string literal");
    }

    #[test]
    fn invalid_escape_sequence_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::InvalidEscapeSequence('q'), test_location());
        assert_eq!(error.message(&interner), "invalid escape sequence '\\q'");
    }

    #[test]
    fn invalid_number_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::InvalidNumber, test_location());
        assert_eq!(error.message(&interner), "invalid numeric literal");
    }

    #[test]
    fn invalid_unicode_escape_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::InvalidUnicodeEscape, test_location());
        assert_eq!(error.message(&interner), "invalid unicode escape sequence");
    }

    #[test]
    fn unexpected_token_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::UnexpectedToken {
                expected: "expression",
                found: "right parenthesis",
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "unexpected right parenthesis, expected expression"
        );
    }

    #[test]
    fn unmatched_delimiter_message() {
        let interner = Interner::new();
        let opener_loc = test_location();
        let error = Error::new(
            Kind::UnmatchedDelimiter {
                opener: '(',
                opener_location: opener_loc,
                expected: ')',
                found: ']',
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "expected ')' to match '(', but found ']'"
        );
    }

    #[test]
    fn unexpected_eof_message() {
        let interner = Interner::new();
        let error = Error::new(
            Kind::UnexpectedEof {
                expected: "closing delimiter",
            },
            test_location(),
        );
        assert_eq!(
            error.message(&interner),
            "unexpected end of input, expected closing delimiter"
        );
    }

    #[test]
    fn odd_map_entries_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::OddMapEntries, test_location());
        assert_eq!(
            error.message(&interner),
            "map literal must have an even number of elements"
        );
    }

    #[test]
    fn reader_macro_missing_expr_message() {
        let interner = Interner::new();
        let error = Error::new(Kind::ReaderMacroMissingExpr, test_location());
        assert_eq!(
            error.message(&interner),
            "reader macro must be followed by an expression"
        );
    }

    #[test]
    fn unterminated_string_notes() {
        let interner = Interner::new();
        let error = Error::new(Kind::UnterminatedString, test_location());
        let notes = error.notes(&interner);
        assert_eq!(notes.len(), 1_usize);
        assert!(matches!(&notes[0], Note::Text(msg) if msg.contains("closing")));
    }

    #[test]
    fn invalid_escape_sequence_notes() {
        let interner = Interner::new();
        let error = Error::new(Kind::InvalidEscapeSequence('q'), test_location());
        let notes = error.notes(&interner);
        assert!(!notes.is_empty());
        assert!(matches!(&notes[0], Note::Text(msg) if msg.contains("valid escape")));
    }

    #[test]
    fn invalid_escape_sequence_x_hint() {
        let interner = Interner::new();
        let error = Error::new(Kind::InvalidEscapeSequence('x'), test_location());
        let notes = error.notes(&interner);
        assert!(
            notes
                .iter()
                .any(|note| matches!(note, Note::Help(msg) if msg.contains("\\uXXXX")))
        );
    }

    #[test]
    fn unmatched_delimiter_notes() {
        let interner = Interner::new();
        let opener_loc = SourceLocation::new(SourceId::new(0_u32), Span::new(5_usize, 6_usize));
        let error = Error::new(
            Kind::UnmatchedDelimiter {
                opener: '(',
                opener_location: opener_loc,
                expected: ')',
                found: ']',
            },
            test_location(),
        );
        let notes = error.notes(&interner);
        assert!(!notes.is_empty());
        assert!(notes.iter().any(
            |note| matches!(note, Note::DefinedAt { description, .. } if description.contains("opened here"))
        ));
    }

    #[test]
    fn odd_map_entries_notes() {
        let interner = Interner::new();
        let error = Error::new(Kind::OddMapEntries, test_location());
        let notes = error.notes(&interner);
        assert_eq!(notes.len(), 2_usize);
    }

    #[test]
    fn severity_is_error() {
        let error = Error::new(Kind::InvalidNumber, test_location());
        assert_eq!(error.severity(), Severity::Error);
    }

    #[test]
    fn location_accessor() {
        let location = SourceLocation::new(SourceId::new(42_u32), Span::new(10_usize, 20_usize));
        let error = Error::new(Kind::InvalidNumber, location);
        assert_eq!(error.location(), location);
    }

    #[test]
    fn variant_name_matches_kind() {
        let error = Error::new(Kind::InvalidNumber, test_location());
        assert_eq!(error.variant_name(), "InvalidNumber");

        let error2 = Error::new(Kind::UnterminatedString, test_location());
        assert_eq!(error2.variant_name(), "UnterminatedString");
    }
}
