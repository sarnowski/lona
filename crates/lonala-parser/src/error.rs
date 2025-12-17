// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Error types for lexical analysis and parsing.
//!
//! This module provides error types and location information for reporting
//! issues encountered during tokenization and parsing of Lonala source code.

use core::fmt;

// Re-export Span from lona-core for consistency across the compiler pipeline.
pub use lona_core::span::Span;

/// Kinds of errors that can occur during lexing and parsing.
///
/// Each variant captures the specific nature of the error,
/// enabling precise error messages and potential recovery strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    // Lexer errors
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

    // Parser errors
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
/// Combines an error kind with its location in the source, enabling
/// helpful error messages that point to the exact position of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Error {
    /// The kind of error.
    pub kind: Kind,
    /// Location in source where the error occurred.
    pub span: Span,
}

impl Error {
    /// Creates a new lexer error.
    #[inline]
    #[must_use]
    pub const fn new(kind: Kind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { ref kind, span } = *self;
        write!(f, "{kind} at {span}")
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

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
        assert_eq!(
            format!(
                "{}",
                Kind::UnmatchedDelimiter {
                    opener: '(',
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

    #[test]
    fn error_display() {
        let error = Error::new(Kind::UnexpectedCharacter('@'), Span::new(5_usize, 6_usize));
        assert_eq!(format!("{error}"), "unexpected character '@' at 5..6");
    }
}
