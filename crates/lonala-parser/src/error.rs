// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Error types for lexical analysis.
//!
//! This module provides error types and location information for reporting
//! issues encountered during tokenization of Lonala source code.

use core::fmt;

/// A byte range in source code for error reporting.
///
/// Spans are half-open intervals `[start, end)` representing byte offsets
/// into the source string. They enable precise error messages that can
/// highlight the problematic portion of the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offset of the start (inclusive).
    pub start: usize,
    /// Byte offset of the end (exclusive).
    pub end: usize,
}

impl Span {
    /// Creates a new span from start to end byte offsets.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the length of this span in bytes.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if this span has zero length.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Kinds of errors that can occur during lexing.
///
/// Each variant captures the specific nature of the lexical error,
/// enabling precise error messages and potential recovery strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
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
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter(ch) => write!(f, "unexpected character '{ch}'"),
            Self::UnterminatedString => write!(f, "unterminated string literal"),
            Self::InvalidEscapeSequence(ch) => write!(f, "invalid escape sequence '\\{ch}'"),
            Self::InvalidNumber => write!(f, "invalid numeric literal"),
            Self::InvalidUnicodeEscape => write!(f, "invalid unicode escape sequence"),
        }
    }
}

/// An error encountered during lexical analysis.
///
/// Combines an error kind with its location in the source, enabling
/// helpful error messages that point to the exact position of the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// The kind of error.
    pub kind: Kind,
    /// Location in source where the error occurred.
    pub span: Span,
}

impl Error {
    /// Creates a new lexer error.
    pub const fn new(kind: Kind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.kind, self.span)
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
    fn error_kind_display() {
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
    fn error_display() {
        let error = Error::new(Kind::UnexpectedCharacter('@'), Span::new(5_usize, 6_usize));
        assert_eq!(format!("{error}"), "unexpected character '@' at 5..6");
    }
}
