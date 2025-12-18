// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Number parsing for the Lonala lexer.
//!
//! This module contains methods for parsing numeric literals including:
//! - Decimal integers
//! - Hexadecimal (0x...), binary (0b...), and octal (0o...) integers
//! - Floating point numbers with optional exponent notation

use crate::error::{Error, Kind as ErrorKind, Span};
use crate::token::{Kind as TokenKind, Token};

use super::Lexer;

impl<'src> Lexer<'src> {
    /// Parses a number starting with a digit.
    pub(super) fn number_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        // Check for base prefixes (0x, 0b, 0o)
        if self.current_char() == Some('0') {
            self.advance();
            match self.current_char() {
                Some('x' | 'X') => return self.hex_number(start),
                Some('b' | 'B') => return self.binary_number(start),
                Some('o' | 'O') => return self.octal_number(start),
                Some('.') => return self.float_after_integer(start),
                Some('e' | 'E') => return self.float_exponent(start),
                Some(char) if char.is_ascii_digit() => {
                    // Continue parsing as decimal
                }
                _ => {
                    // Just '0'
                    let lexeme = self.source.get(start..self.position).unwrap_or("");
                    return Ok(Token::new(
                        TokenKind::Integer,
                        lexeme,
                        Span::new(start, self.position),
                    ));
                }
            }
        }

        // Consume decimal digits
        while self
            .current_char()
            .is_some_and(|char| char.is_ascii_digit())
        {
            self.advance();
        }

        // Check for float
        match self.current_char() {
            Some('.') => self.float_after_integer(start),
            Some('e' | 'E') => self.float_exponent(start),
            _ => {
                let lexeme = self.source.get(start..self.position).unwrap_or("");
                Ok(Token::new(
                    TokenKind::Integer,
                    lexeme,
                    Span::new(start, self.position),
                ))
            }
        }
    }

    /// Parses a hexadecimal number after seeing `0x` or `0X`.
    fn hex_number(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume x/X

        // Must have at least one hex digit
        if !self
            .current_char()
            .is_some_and(|char| char.is_ascii_hexdigit())
        {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                self.location_from(start, self.position),
            ));
        }

        while self
            .current_char()
            .is_some_and(|char| char.is_ascii_hexdigit())
        {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Integer,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses a binary number after seeing `0b` or `0B`.
    fn binary_number(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume b/B

        // Must have at least one binary digit
        if !self
            .current_char()
            .is_some_and(|char| char == '0' || char == '1')
        {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                self.location_from(start, self.position),
            ));
        }

        while self
            .current_char()
            .is_some_and(|char| char == '0' || char == '1')
        {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Integer,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses an octal number after seeing `0o` or `0O`.
    fn octal_number(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume o/O

        // Must have at least one octal digit
        if !self
            .current_char()
            .is_some_and(|char| ('0'..='7').contains(&char))
        {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                self.location_from(start, self.position),
            ));
        }

        while self
            .current_char()
            .is_some_and(|char| ('0'..='7').contains(&char))
        {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Integer,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses the fractional part of a float after seeing `.`.
    fn float_after_integer(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume .

        // Must have at least one digit after the dot
        if !self
            .current_char()
            .is_some_and(|char| char.is_ascii_digit())
        {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                self.location_from(start, self.position),
            ));
        }

        while self
            .current_char()
            .is_some_and(|char| char.is_ascii_digit())
        {
            self.advance();
        }

        // Check for exponent
        if self
            .current_char()
            .is_some_and(|char| char == 'e' || char == 'E')
        {
            return self.float_exponent(start);
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Float,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses the exponent part of a float after seeing `e` or `E`.
    fn float_exponent(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume e/E

        // Optional sign
        if self
            .current_char()
            .is_some_and(|char| char == '+' || char == '-')
        {
            self.advance();
        }

        // Must have at least one digit
        if !self
            .current_char()
            .is_some_and(|char| char.is_ascii_digit())
        {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                self.location_from(start, self.position),
            ));
        }

        while self
            .current_char()
            .is_some_and(|char| char.is_ascii_digit())
        {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Float,
            lexeme,
            Span::new(start, self.position),
        ))
    }
}
