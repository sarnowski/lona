// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Lexical analyzer for Lonala source code.
//!
//! The lexer converts source text into a stream of tokens, handling
//! whitespace, comments, and all Lonala lexical elements. It implements
//! `Iterator` for lazy, streaming tokenization.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

mod numbers;
mod strings;
#[cfg(test)]
mod tests;

use crate::error::{Error, Kind as ErrorKind, SourceId, SourceLocation, Span};
use crate::token::{Kind as TokenKind, Token};

/// Lexical analyzer for Lonala source code.
///
/// The lexer maintains a position in the source string and produces tokens
/// on demand via the `Iterator` trait. It handles:
/// - Whitespace (spaces, tabs, newlines, commas)
/// - Comments (`;` to end of line)
/// - All Lonala token types (delimiters, literals, identifiers, reader macros)
pub struct Lexer<'src> {
    /// The source code being lexed.
    source: &'src str,
    /// Identifier for this source (for error reporting).
    source_id: SourceId,
    /// Current byte position in source.
    position: usize,
    /// Cached next token for peek support.
    ///
    /// Uses `Option<Option<T>>` intentionally to distinguish:
    /// - `None`: not peeked yet
    /// - `Some(None)`: peeked and found EOF
    /// - `Some(Some(token))`: peeked and found token
    #[expect(
        clippy::option_option,
        reason = "[approved] Intentional: distinguishes 'not peeked' from 'peeked EOF'"
    )]
    peeked: Option<Option<Result<Token<'src>, Error>>>,
}

impl<'src> Lexer<'src> {
    /// Creates a new lexer for the given source code.
    ///
    /// The `source_id` identifies which source this lexer is processing,
    /// and will be included in all error locations for precise error reporting.
    #[inline]
    #[must_use]
    pub const fn new(source: &'src str, source_id: SourceId) -> Self {
        Self {
            source,
            source_id,
            position: 0_usize,
            peeked: None,
        }
    }

    /// Returns the source ID for this lexer.
    #[inline]
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Creates a source location from a span within this source.
    #[inline]
    #[must_use]
    pub(crate) const fn location(&self, span: Span) -> SourceLocation {
        SourceLocation::new(self.source_id, span)
    }

    /// Creates a source location from start and end positions.
    #[inline]
    #[must_use]
    pub(crate) const fn location_from(&self, start: usize, end: usize) -> SourceLocation {
        self.location(Span::new(start, end))
    }

    /// Peeks at the next token without consuming it.
    ///
    /// Returns `None` if there are no more tokens. The peeked token
    /// is cached and returned on the next call to `next()`.
    #[inline]
    pub fn peek(&mut self) -> Option<&Result<Token<'src>, Error>> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next_token());
        }
        self.peeked.as_ref()?.as_ref()
    }

    /// Returns the remaining unparsed source.
    fn remaining(&self) -> &'src str {
        self.source.get(self.position..).unwrap_or("")
    }

    /// Returns the current character without advancing.
    fn current_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    /// Advances the position by one character and returns it.
    fn advance(&mut self) -> Option<char> {
        let ch = self.current_char()?;
        self.position = self.position.saturating_add(ch.len_utf8());
        Some(ch)
    }

    /// Advances the position by `byte_count` bytes.
    const fn skip_bytes(&mut self, byte_count: usize) {
        self.position = self.position.saturating_add(byte_count);
    }

    /// Skips whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current_char() {
                // Whitespace (including comma, which is whitespace in Clojure)
                Some(' ' | '\t' | '\n' | '\r' | ',') => {
                    self.advance();
                }
                // Comment: ; to end of line
                Some(';') => {
                    while let Some(ch) = self.current_char() {
                        self.advance();
                        if ch == '\n' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    /// Produces the next token from the source.
    fn next_token(&mut self) -> Option<Result<Token<'src>, Error>> {
        self.skip_whitespace_and_comments();

        let start = self.position;
        let ch = self.current_char()?;

        let result = match ch {
            // Delimiters
            '(' => Ok(self.single_char_token(TokenKind::LeftParen, start)),
            ')' => Ok(self.single_char_token(TokenKind::RightParen, start)),
            '[' => Ok(self.single_char_token(TokenKind::LeftBracket, start)),
            ']' => Ok(self.single_char_token(TokenKind::RightBracket, start)),
            '{' => Ok(self.single_char_token(TokenKind::LeftBrace, start)),
            '}' => Ok(self.single_char_token(TokenKind::RightBrace, start)),

            // Reader macros
            '\'' => Ok(self.single_char_token(TokenKind::Quote, start)),
            '`' => Ok(self.single_char_token(TokenKind::SyntaxQuote, start)),
            '~' => Ok(self.tilde_token(start)),

            // String literal (delegated to strings module)
            '"' => self.string_token(start),

            // Keyword (delegated to strings module)
            ':' => self.keyword_token(start),

            // Special floats (##NaN, ##Inf, ##-Inf)
            '#' => self.hash_token(start),

            // Number or symbol starting with digit (delegated to numbers module)
            '0'..='9' => self.number_token(start),

            // Negative number or symbol starting with -
            '-' => self.minus_token(start),

            // Symbol starting with +
            '+' => Ok(self.plus_token(start)),

            // Any other symbol character
            _ if is_symbol_start(ch) => Ok(self.symbol_token(start)),

            // Unexpected character
            _ => {
                self.advance();
                Err(Error::new(
                    ErrorKind::UnexpectedCharacter(ch),
                    self.location_from(start, self.position),
                ))
            }
        };

        Some(result)
    }

    /// Creates a single-character token.
    fn single_char_token(&mut self, kind: TokenKind, start: usize) -> Token<'src> {
        self.advance();
        let end = self.position;
        let lexeme = self.source.get(start..end).unwrap_or("");
        Token::new(kind, lexeme, Span::new(start, end))
    }

    /// Handles `~` which could be `~` (Unquote) or `~@` (`UnquoteSplice`).
    fn tilde_token(&mut self, start: usize) -> Token<'src> {
        self.advance(); // consume ~
        if self.current_char() == Some('@') {
            self.advance(); // consume @
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            Token::new(
                TokenKind::UnquoteSplice,
                lexeme,
                Span::new(start, self.position),
            )
        } else {
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            Token::new(TokenKind::Unquote, lexeme, Span::new(start, self.position))
        }
    }

    /// Handles `#` which starts special float literals.
    fn hash_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        let remaining = self.remaining();

        // Check for ##NaN
        if remaining.starts_with("##NaN") {
            self.skip_bytes(5_usize);
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            return Ok(Token::new(
                TokenKind::Float,
                lexeme,
                Span::new(start, self.position),
            ));
        }

        // Check for ##Inf
        if remaining.starts_with("##Inf") {
            self.skip_bytes(5_usize);
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            return Ok(Token::new(
                TokenKind::Float,
                lexeme,
                Span::new(start, self.position),
            ));
        }

        // Check for ##-Inf
        if remaining.starts_with("##-Inf") {
            self.skip_bytes(6_usize);
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            return Ok(Token::new(
                TokenKind::Float,
                lexeme,
                Span::new(start, self.position),
            ));
        }

        // Unknown # sequence - treat as unexpected
        self.advance();
        Err(Error::new(
            ErrorKind::UnexpectedCharacter('#'),
            self.location_from(start, self.position),
        ))
    }

    /// Handles `-` which could start a negative number or be a symbol.
    fn minus_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume -

        // Check if followed by a digit (negative number)
        if self
            .current_char()
            .is_some_and(|char| char.is_ascii_digit())
        {
            return self.number_token(start);
        }

        // Otherwise it's a symbol (possibly just `-`)
        while self.current_char().is_some_and(is_symbol_continue) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Symbol,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Handles `+` which could be followed by digits or be a symbol.
    fn plus_token(&mut self, start: usize) -> Token<'src> {
        self.advance(); // consume +

        // Check if followed by a digit (positive number - but we keep +)
        if self
            .current_char()
            .is_some_and(|char| char.is_ascii_digit())
        {
            // Actually, in Lonala/Clojure, +42 is not a number literal
            // + followed by digits is the symbol + followed by a number
            // So we just return the + symbol
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            return Token::new(TokenKind::Symbol, lexeme, Span::new(start, self.position));
        }

        // Otherwise continue as symbol
        while self.current_char().is_some_and(is_symbol_continue) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Token::new(TokenKind::Symbol, lexeme, Span::new(start, self.position))
    }

    /// Parses a symbol.
    fn symbol_token(&mut self, start: usize) -> Token<'src> {
        while self.current_char().is_some_and(is_symbol_continue) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");

        // Check for reserved words
        let kind = match lexeme {
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            _ => TokenKind::Symbol,
        };

        Token::new(kind, lexeme, Span::new(start, self.position))
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Result<Token<'src>, Error>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.peeked.take().unwrap_or_else(|| self.next_token())
    }
}

/// Returns true if `ch` can start a symbol.
const fn is_symbol_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || matches!(
            ch,
            '_' | '+' | '-' | '*' | '/' | '<' | '>' | '=' | '!' | '?' | '&' | '%' | '^' | '.'
        )
}

/// Returns true if `ch` can continue a symbol.
const fn is_symbol_continue(ch: char) -> bool {
    is_symbol_start(ch) || ch.is_ascii_digit()
}

/// Tokenizes the entire source into a vector of tokens.
///
/// This is a convenience function that collects all tokens from the lexer.
/// Returns an error if any token fails to parse.
///
/// The `source_id` identifies which source is being tokenized for error reporting.
#[cfg(feature = "alloc")]
#[inline]
pub fn tokenize(source: &str, source_id: SourceId) -> Result<Vec<Token<'_>>, Error> {
    Lexer::new(source, source_id).collect()
}
