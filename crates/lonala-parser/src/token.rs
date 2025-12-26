// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Token types produced by the lexer.
//!
//! This module defines the token kinds and token structures that represent
//! the lexical elements of Lonala source code. Tokens carry both their
//! semantic type and a reference to the source text for zero-copy parsing.

use crate::error::Span;

/// The kind of token produced by the lexer.
///
/// Tokens are categorized into delimiters, literals, identifiers,
/// and reader macros following Clojure/LISP syntax conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    // Delimiters
    /// Left parenthesis `(`.
    LeftParen,
    /// Right parenthesis `)`.
    RightParen,
    /// Left bracket `[`.
    LeftBracket,
    /// Right bracket `]`.
    RightBracket,
    /// Left brace `{`.
    LeftBrace,
    /// Right brace `}`.
    RightBrace,
    /// Set start `#{`.
    SetStart,
    /// Anonymous function start `#(`.
    AnonFnStart,
    /// Discard `#_` (discards the next form).
    Discard,

    // Literals
    /// Integer literal (e.g., `42`, `-17`, `0xFF`, `0b1010`, `0o755`).
    Integer,
    /// Floating-point literal (e.g., `3.14`, `-0.5`, `1e10`, `##NaN`, `##Inf`).
    Float,
    /// String literal (e.g., `"hello"`).
    String,
    /// Boolean `true`.
    True,
    /// Boolean `false`.
    False,
    /// Nil literal.
    Nil,

    // Identifiers
    /// Symbol (e.g., `foo`, `+`, `-`, `ns/name`, `update!`, `empty?`).
    Symbol,
    /// Keyword (e.g., `:foo`, `:ns/name`).
    Keyword,

    // Reader macros
    /// Quote `'`.
    Quote,
    /// Syntax quote `` ` ``.
    SyntaxQuote,
    /// Unquote `~`.
    Unquote,
    /// Unquote-splice `~@`.
    UnquoteSplice,
    /// Caret `^` for metadata.
    Caret,
}

impl Kind {
    /// Returns a human-readable description of this token kind.
    #[inline]
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::LeftParen => "left parenthesis",
            Self::RightParen => "right parenthesis",
            Self::LeftBracket => "left bracket",
            Self::RightBracket => "right bracket",
            Self::LeftBrace => "left brace",
            Self::RightBrace => "right brace",
            Self::SetStart => "set start",
            Self::AnonFnStart => "anonymous function start",
            Self::Discard => "discard",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::String => "string",
            Self::True => "true",
            Self::False => "false",
            Self::Nil => "nil",
            Self::Symbol => "symbol",
            Self::Keyword => "keyword",
            Self::Quote => "quote",
            Self::SyntaxQuote => "syntax quote",
            Self::Unquote => "unquote",
            Self::UnquoteSplice => "unquote-splice",
            Self::Caret => "caret",
        }
    }
}

/// A token produced by the lexer.
///
/// Tokens carry a reference to the source text (`lexeme`) rather than
/// copying it, enabling zero-copy parsing. The lifetime parameter ties
/// the token to the source string's lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Token<'src> {
    /// The kind of token.
    pub kind: Kind,
    /// The source text of this token.
    pub lexeme: &'src str,
    /// Location in the source.
    pub span: Span,
}

impl<'src> Token<'src> {
    /// Creates a new token.
    #[inline]
    #[must_use]
    pub const fn new(kind: Kind, lexeme: &'src str, span: Span) -> Self {
        Self { kind, lexeme, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_kind_description() {
        assert_eq!(Kind::LeftParen.description(), "left parenthesis");
        assert_eq!(Kind::Integer.description(), "integer");
        assert_eq!(Kind::Symbol.description(), "symbol");
        assert_eq!(Kind::Quote.description(), "quote");
    }

    #[test]
    fn token_creation() {
        let token = Token::new(Kind::Integer, "42", Span::new(0_usize, 2_usize));
        assert_eq!(token.kind, Kind::Integer);
        assert_eq!(token.lexeme, "42");
        assert_eq!(token.span.start, 0_usize);
        assert_eq!(token.span.end, 2_usize);
    }

    #[test]
    fn token_equality() {
        let t1 = Token::new(Kind::Symbol, "foo", Span::new(0_usize, 3_usize));
        let t2 = Token::new(Kind::Symbol, "foo", Span::new(0_usize, 3_usize));
        let t3 = Token::new(Kind::Symbol, "bar", Span::new(0_usize, 3_usize));
        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
    }
}
