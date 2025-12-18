// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! String and keyword parsing for the Lonala lexer.
//!
//! This module contains methods for parsing string literals and keywords.

use crate::error::{Error, Kind as ErrorKind, Span};
use crate::token::{Kind as TokenKind, Token};

use super::{Lexer, is_symbol_continue, is_symbol_start};

impl<'src> Lexer<'src> {
    /// Parses a string literal.
    pub(super) fn string_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume opening "

        loop {
            match self.current_char() {
                None => {
                    return Err(Error::new(
                        ErrorKind::UnterminatedString,
                        Span::new(start, self.position),
                    ));
                }
                Some('"') => {
                    self.advance(); // consume closing "
                    break;
                }
                Some('\\') => {
                    self.advance(); // consume backslash
                    match self.current_char() {
                        None => {
                            return Err(Error::new(
                                ErrorKind::UnterminatedString,
                                Span::new(start, self.position),
                            ));
                        }
                        Some('\\' | '"' | 'n' | 't' | 'r' | '0') => {
                            self.advance();
                        }
                        Some('u') => {
                            self.advance(); // consume 'u'
                            // Expect exactly 4 hex digits
                            for _ in 0_u8..4_u8 {
                                match self.current_char() {
                                    Some(char) if char.is_ascii_hexdigit() => {
                                        self.advance();
                                    }
                                    _ => {
                                        return Err(Error::new(
                                            ErrorKind::InvalidUnicodeEscape,
                                            Span::new(start, self.position),
                                        ));
                                    }
                                }
                            }
                        }
                        Some(ch) => {
                            return Err(Error::new(
                                ErrorKind::InvalidEscapeSequence(ch),
                                Span::new(start, self.position),
                            ));
                        }
                    }
                }
                Some(_) => {
                    self.advance();
                }
            }
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::String,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses a keyword (`:foo`, `:ns/name`).
    pub(super) fn keyword_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume :

        // Keywords must have at least one character after the colon
        if !self.current_char().is_some_and(is_symbol_start) {
            // Bare colon is an error
            return Err(Error::new(
                ErrorKind::UnexpectedCharacter(':'),
                Span::new(start, self.position),
            ));
        }

        // Consume the rest of the keyword (symbol characters)
        while self.current_char().is_some_and(is_symbol_continue) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Keyword,
            lexeme,
            Span::new(start, self.position),
        ))
    }
}
