// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Error types for lexical analysis and parsing.
//!
//! This module provides error types and location information for reporting
//! issues encountered during tokenization and parsing of Lonala source code.
//!
//! # Design Principles
//!
//! - **Structured data, not strings**: Errors carry typed data; formatting happens in `lonala-human`
//! - **Source locations always**: Every error includes `source::Location`
//! - **No Display on Error**: Formatting is centralized in `lonala-human` crate

use core::fmt;

// Re-export Span and source types from lona-core for consistency.
pub use lona_core::source::{self, Id as SourceId, Location as SourceLocation};
pub use lona_core::span::Span;

/// Kinds of errors that can occur during lexing and parsing.
///
/// Each variant captures the specific nature of the error with all context
/// needed for formatting. NO human-readable strings should be stored here.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    // ========== Lexer errors ==========
    /// Encountered a character that cannot start any token.
    UnexpectedCharacter(char),
    /// String literal reached end of input without closing quote.
    UnterminatedString,
    /// Invalid escape sequence in string (e.g., `\q`).
    InvalidEscapeSequence(char),
    /// Malformed numeric literal.
    InvalidNumber,
    /// Invalid unicode escape sequence (`\uXXXX`).
    InvalidUnicodeEscape,

    // ========== Parser errors ==========
    /// Unexpected token encountered during parsing.
    UnexpectedToken {
        /// Description of what was expected.
        expected: &'static str,
        /// Description of what was found.
        found: &'static str,
    },
    /// Closing delimiter doesn't match the opening delimiter.
    UnmatchedDelimiter {
        /// The opening delimiter character.
        opener: char,
        /// Location of the opening delimiter (for "to match X at line Y").
        opener_location: SourceLocation,
        /// The expected closing delimiter.
        expected: char,
        /// The actual closing delimiter found.
        found: char,
    },
    /// Unexpected end of input during parsing.
    UnexpectedEof {
        /// Description of what was expected.
        expected: &'static str,
    },
    /// Map literal has an odd number of elements.
    OddMapEntries,
    /// Reader macro not followed by an expression.
    ReaderMacroMissingExpr,
}

impl Kind {
    /// Returns the variant name for error identification.
    ///
    /// Used as a stable error identifier in formatted output (e.g., `error[UnmatchedDelimiter]`).
    #[inline]
    #[must_use]
    pub const fn variant_name(&self) -> &'static str {
        match *self {
            Self::UnexpectedCharacter(_) => "UnexpectedCharacter",
            Self::UnterminatedString => "UnterminatedString",
            Self::InvalidEscapeSequence(_) => "InvalidEscapeSequence",
            Self::InvalidNumber => "InvalidNumber",
            Self::InvalidUnicodeEscape => "InvalidUnicodeEscape",
            Self::UnexpectedToken { .. } => "UnexpectedToken",
            Self::UnmatchedDelimiter { .. } => "UnmatchedDelimiter",
            Self::UnexpectedEof { .. } => "UnexpectedEof",
            Self::OddMapEntries => "OddMapEntries",
            Self::ReaderMacroMissingExpr => "ReaderMacroMissingExpr",
        }
    }
}

impl fmt::Display for Kind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            // Lexer errors
            Self::UnexpectedCharacter(ch) => write!(f, "unexpected character '{ch}'"),
            Self::UnterminatedString => write!(f, "unterminated string literal"),
            Self::InvalidEscapeSequence(ch) => {
                write!(f, "invalid escape sequence '\\{ch}'")
            }
            Self::InvalidNumber => write!(f, "invalid numeric literal"),
            Self::InvalidUnicodeEscape => write!(f, "invalid unicode escape sequence"),
            // Parser errors
            Self::UnexpectedToken { expected, found } => {
                write!(f, "unexpected {found}, expected {expected}")
            }
            Self::UnmatchedDelimiter {
                opener,
                expected,
                found,
                ..
            } => {
                write!(
                    f,
                    "mismatched delimiter: '{opener}' opened, expected '{expected}' but found '{found}'"
                )
            }
            Self::UnexpectedEof { expected } => {
                write!(f, "unexpected end of input, expected {expected}")
            }
            Self::OddMapEntries => {
                write!(f, "map literal must have an even number of elements")
            }
            Self::ReaderMacroMissingExpr => {
                write!(f, "reader macro must be followed by an expression")
            }
        }
    }
}

/// An error encountered during lexical analysis or parsing.
///
/// Combines an error kind with its full source location, enabling helpful error
/// messages that can point to the exact position in the correct source file.
///
/// # Note
///
/// This type does NOT implement `Display`. All formatting is centralized in
/// the `lonala-human` crate to ensure consistent error presentation across
/// REPL and future LSP implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Error {
    /// The kind of error.
    pub kind: Kind,
    /// Full source location (source ID + byte span).
    pub location: SourceLocation,
}

impl Error {
    /// Creates a new error with the given kind and source location.
    #[inline]
    #[must_use]
    pub const fn new(kind: Kind, location: SourceLocation) -> Self {
        Self { kind, location }
    }

    /// Returns the span within the source where the error occurred.
    #[inline]
    #[must_use]
    pub const fn span(&self) -> Span {
        self.location.span
    }

    /// Returns the source ID where the error occurred.
    #[inline]
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.location.source
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

    /// Helper to create a test source location.
    fn test_location(start: usize, end: usize) -> SourceLocation {
        SourceLocation::new(SourceId::new(0_u32), Span::new(start, end))
    }

    // ==================== Span Tests ====================

    #[test]
    fn span_new_and_accessors() {
        let span = Span::new(10_usize, 20_usize);
        assert_eq!(span.start, 10_usize);
        assert_eq!(span.end, 20_usize);
        assert_eq!(span.len(), 10_usize);
        assert!(!span.is_empty());
    }

    #[test]
    fn span_empty() {
        let span = Span::new(5_usize, 5_usize);
        assert!(span.is_empty());
        assert_eq!(span.len(), 0_usize);
    }

    #[test]
    fn span_display() {
        let span = Span::new(10_usize, 20_usize);
        assert_eq!(format!("{span}"), "10..20");
    }

    // ==================== Kind Display Tests ====================

    #[test]
    fn error_kind_display_lexer_errors() {
        assert_eq!(
            format!("{}", Kind::UnexpectedCharacter('@')),
            "unexpected character '@'"
        );
        assert_eq!(
            format!("{}", Kind::UnterminatedString),
            "unterminated string literal"
        );
        assert_eq!(
            format!("{}", Kind::InvalidEscapeSequence('q')),
            "invalid escape sequence '\\q'"
        );
        assert_eq!(
            format!("{}", Kind::InvalidNumber),
            "invalid numeric literal"
        );
        assert_eq!(
            format!("{}", Kind::InvalidUnicodeEscape),
            "invalid unicode escape sequence"
        );
    }

    #[test]
    fn error_kind_display_parser_errors() {
        assert_eq!(
            format!(
                "{}",
                Kind::UnexpectedToken {
                    expected: "expression",
                    found: "right parenthesis"
                }
            ),
            "unexpected right parenthesis, expected expression"
        );
        // Create a location for the opener in UnmatchedDelimiter
        let opener_loc = test_location(0_usize, 1_usize);
        assert_eq!(
            format!(
                "{}",
                Kind::UnmatchedDelimiter {
                    opener: '(',
                    opener_location: opener_loc,
                    expected: ')',
                    found: ']'
                }
            ),
            "mismatched delimiter: '(' opened, expected ')' but found ']'"
        );
        assert_eq!(
            format!(
                "{}",
                Kind::UnexpectedEof {
                    expected: "closing delimiter"
                }
            ),
            "unexpected end of input, expected closing delimiter"
        );
        assert_eq!(
            format!("{}", Kind::OddMapEntries),
            "map literal must have an even number of elements"
        );
        assert_eq!(
            format!("{}", Kind::ReaderMacroMissingExpr),
            "reader macro must be followed by an expression"
        );
    }

    // ==================== Kind variant_name() Tests ====================

    #[test]
    fn kind_variant_name() {
        assert_eq!(
            Kind::UnexpectedCharacter('@').variant_name(),
            "UnexpectedCharacter"
        );
        assert_eq!(
            Kind::UnterminatedString.variant_name(),
            "UnterminatedString"
        );
        assert_eq!(
            Kind::InvalidEscapeSequence('q').variant_name(),
            "InvalidEscapeSequence"
        );
        assert_eq!(Kind::InvalidNumber.variant_name(), "InvalidNumber");
        assert_eq!(
            Kind::InvalidUnicodeEscape.variant_name(),
            "InvalidUnicodeEscape"
        );
        assert_eq!(
            Kind::UnexpectedToken {
                expected: "x",
                found: "y"
            }
            .variant_name(),
            "UnexpectedToken"
        );
        assert_eq!(
            Kind::UnmatchedDelimiter {
                opener: '(',
                opener_location: test_location(0_usize, 1_usize),
                expected: ')',
                found: ']'
            }
            .variant_name(),
            "UnmatchedDelimiter"
        );
        assert_eq!(
            Kind::UnexpectedEof { expected: "x" }.variant_name(),
            "UnexpectedEof"
        );
        assert_eq!(Kind::OddMapEntries.variant_name(), "OddMapEntries");
        assert_eq!(
            Kind::ReaderMacroMissingExpr.variant_name(),
            "ReaderMacroMissingExpr"
        );
    }

    // ==================== Error Tests ====================

    #[test]
    fn error_new_and_accessors() {
        let location = test_location(5_usize, 6_usize);
        let error = Error::new(Kind::UnexpectedCharacter('@'), location);
        assert_eq!(error.kind, Kind::UnexpectedCharacter('@'));
        assert_eq!(error.span(), Span::new(5_usize, 6_usize));
        assert_eq!(error.source_id(), SourceId::new(0_u32));
    }

    #[test]
    fn error_location_field() {
        let source_id = SourceId::new(42_u32);
        let span = Span::new(10_usize, 20_usize);
        let location = SourceLocation::new(source_id, span);
        let error = Error::new(Kind::InvalidNumber, location);
        assert_eq!(error.location.source, source_id);
        assert_eq!(error.location.span, span);
    }
}
