// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Parser for Lonala S-expressions.
//!
//! The parser transforms a stream of tokens into an Abstract Syntax Tree (AST).
//! It handles reader macros (quote, syntax-quote, unquote, unquote-splicing)
//! by expanding them to their canonical list forms.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::ast::{Ast, Spanned};
use crate::error::{Error, Kind as ErrorKind, Span};
use crate::lexer::Lexer;
use crate::token::Kind as TokenKind;

#[cfg(test)]
mod tests;

/// Parser for Lonala S-expressions.
///
/// Transforms tokens from the lexer into an AST. Uses recursive descent
/// parsing to handle nested expressions and collections.
pub struct Parser<'src> {
    /// The underlying lexer.
    lexer: Lexer<'src>,
    /// The source string (for span extraction).
    source: &'src str,
}

impl<'src> Parser<'src> {
    /// Creates a new parser for the given source code.
    #[inline]
    #[must_use]
    pub const fn new(source: &'src str) -> Self {
        Self {
            lexer: Lexer::new(source),
            source,
        }
    }

    /// Parses all expressions from the source.
    ///
    /// Returns a vector of spanned AST nodes, one for each top-level expression.
    #[inline]
    #[must_use = "parsing result should be used"]
    pub fn parse(&mut self) -> Result<Vec<Spanned<Ast>>, Error> {
        let mut exprs = Vec::new();
        while self.lexer.peek().is_some() {
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }

    /// Parses a single expression from the source.
    ///
    /// Returns an error if there are no expressions or if parsing fails.
    #[inline]
    #[must_use = "parsing result should be used"]
    pub fn parse_one(&mut self) -> Result<Spanned<Ast>, Error> {
        if self.lexer.peek().is_none() {
            return Err(Error::new(
                ErrorKind::UnexpectedEof {
                    expected: "expression",
                },
                Span::new(self.source.len(), self.source.len()),
            ));
        }
        self.parse_expr()
    }

    /// Parses a single expression.
    fn parse_expr(&mut self) -> Result<Spanned<Ast>, Error> {
        let token = match self.lexer.peek() {
            Some(&Ok(ref token)) => token.clone(),
            Some(&Err(ref err)) => return Err(err.clone()),
            None => {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof {
                        expected: "expression",
                    },
                    Span::new(self.source.len(), self.source.len()),
                ));
            }
        };

        match token.kind {
            // Delimiters - start collections
            TokenKind::LeftParen => self.parse_list(),
            TokenKind::LeftBracket => self.parse_vector(),
            TokenKind::LeftBrace => self.parse_map(),

            // Reader macros
            TokenKind::Quote => self.parse_reader_macro("quote"),
            TokenKind::SyntaxQuote => self.parse_reader_macro("syntax-quote"),
            TokenKind::Unquote => self.parse_reader_macro("unquote"),
            TokenKind::UnquoteSplice => self.parse_reader_macro("unquote-splicing"),

            // Atoms
            TokenKind::Integer
            | TokenKind::Float
            | TokenKind::String
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::Symbol
            | TokenKind::Keyword => self.parse_atom(),

            // Unexpected closing delimiters
            TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                Err(Error::new(
                    ErrorKind::UnexpectedToken {
                        expected: "expression",
                        found: token.kind.description(),
                    },
                    token.span,
                ))
            }
        }
    }

    /// Parses a list `(...)`.
    fn parse_list(&mut self) -> Result<Spanned<Ast>, Error> {
        self.parse_collection(TokenKind::LeftParen, TokenKind::RightParen, '(', ')')
            .map(|(elements, span)| Spanned::new(Ast::list(elements), span))
    }

    /// Parses a vector `[...]`.
    fn parse_vector(&mut self) -> Result<Spanned<Ast>, Error> {
        self.parse_collection(TokenKind::LeftBracket, TokenKind::RightBracket, '[', ']')
            .map(|(elements, span)| Spanned::new(Ast::vector(elements), span))
    }

    /// Parses a map `{...}`.
    fn parse_map(&mut self) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftBrace, TokenKind::RightBrace, '{', '}')?;

        // Maps must have an even number of elements
        if !elements.len().is_multiple_of(2_usize) {
            return Err(Error::new(ErrorKind::OddMapEntries, span));
        }

        Ok(Spanned::new(Ast::map(elements), span))
    }

    /// Helper to parse a collection with the given delimiters.
    fn parse_collection(
        &mut self,
        open_kind: TokenKind,
        close_kind: TokenKind,
        open_char: char,
        close_char: char,
    ) -> Result<(Vec<Spanned<Ast>>, Span), Error> {
        // Consume opening delimiter
        let open_token = self.expect_token(open_kind)?;
        let start = open_token.span.start;

        let mut elements = Vec::new();

        loop {
            match self.lexer.peek() {
                None => {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof {
                            expected: close_kind.description(),
                        },
                        Span::new(self.source.len(), self.source.len()),
                    ));
                }
                Some(&Err(ref err)) => return Err(err.clone()),
                Some(&Ok(ref token)) if token.kind == close_kind => {
                    // Consume closing delimiter
                    let close_token = self.advance()?;
                    let span = Span::new(start, close_token.span.end);
                    return Ok((elements, span));
                }
                Some(&Ok(ref token)) => {
                    // Check for mismatched delimiters
                    if matches!(
                        token.kind,
                        TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace
                    ) && token.kind != close_kind
                    {
                        let found_char = match token.kind {
                            TokenKind::RightParen => ')',
                            TokenKind::RightBracket => ']',
                            // RightBrace is the only remaining option
                            TokenKind::RightBrace | _ => '}',
                        };
                        return Err(Error::new(
                            ErrorKind::UnmatchedDelimiter {
                                opener: open_char,
                                expected: close_char,
                                found: found_char,
                            },
                            token.span,
                        ));
                    }

                    // Parse element
                    elements.push(self.parse_expr()?);
                }
            }
        }
    }

    /// Parses a reader macro and expands it to its canonical form.
    ///
    /// For example, `'x` becomes `(quote x)`.
    fn parse_reader_macro(&mut self, symbol_name: &str) -> Result<Spanned<Ast>, Error> {
        // Consume the reader macro token
        let macro_token = self.advance()?;
        let start = macro_token.span.start;

        // Check that an expression follows
        match self.lexer.peek() {
            None => {
                return Err(Error::new(
                    ErrorKind::ReaderMacroMissingExpr,
                    macro_token.span,
                ));
            }
            Some(&Err(ref err)) => return Err(err.clone()),
            Some(&Ok(ref token)) => {
                // Closing delimiters are not valid here
                if matches!(
                    token.kind,
                    TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace
                ) {
                    return Err(Error::new(
                        ErrorKind::ReaderMacroMissingExpr,
                        macro_token.span,
                    ));
                }
            }
        }

        // Parse the inner expression
        let inner = self.parse_expr()?;
        let end = inner.span.end;

        // Create the symbol for the reader macro
        let symbol_span = macro_token.span;
        let symbol = Spanned::new(Ast::symbol(symbol_name), symbol_span);

        // Build the list form: (symbol inner)
        let elements = alloc::vec![symbol, inner];
        let span = Span::new(start, end);

        Ok(Spanned::new(Ast::list(elements), span))
    }

    /// Parses an atom (literal, symbol, or keyword).
    fn parse_atom(&mut self) -> Result<Spanned<Ast>, Error> {
        let token = self.advance()?;

        let ast = match token.kind {
            TokenKind::Integer => {
                let value = Self::parse_integer(token.lexeme)?;
                Ast::integer(value)
            }
            TokenKind::Float => {
                let value = Self::parse_float(token.lexeme)?;
                Ast::float(value)
            }
            TokenKind::String => {
                let value = Self::process_string(token.lexeme, token.span)?;
                Ast::string(value)
            }
            TokenKind::True => Ast::bool(true),
            TokenKind::False => Ast::bool(false),
            TokenKind::Nil => Ast::nil(),
            TokenKind::Symbol => Ast::symbol(token.lexeme),
            TokenKind::Keyword => {
                // Remove the leading colon from the keyword
                let name = token.lexeme.get(1_usize..).unwrap_or("");
                Ast::keyword(name)
            }
            // parse_atom is only called for atom token kinds from parse_expr
            TokenKind::LeftParen
            | TokenKind::LeftBracket
            | TokenKind::LeftBrace
            | TokenKind::RightParen
            | TokenKind::RightBracket
            | TokenKind::RightBrace
            | TokenKind::Quote
            | TokenKind::SyntaxQuote
            | TokenKind::Unquote
            | TokenKind::UnquoteSplice => {
                return Err(Error::new(
                    ErrorKind::UnexpectedToken {
                        expected: "atom",
                        found: token.kind.description(),
                    },
                    token.span,
                ));
            }
        };

        Ok(Spanned::new(ast, token.span))
    }

    /// Parses an integer literal from its lexeme.
    fn parse_integer(lexeme: &str) -> Result<i64, Error> {
        let make_err = || Error::new(ErrorKind::InvalidNumber, Span::new(0_usize, lexeme.len()));

        // Handle different bases
        if lexeme.len() >= 2_usize {
            let prefix = lexeme.get(..2_usize).unwrap_or("");
            let (radix, skip) = match prefix.to_ascii_lowercase().as_str() {
                "0x" => (16_u32, 2_usize),
                "0b" => (2_u32, 2_usize),
                "0o" => (8_u32, 2_usize),
                _ => (10_u32, 0_usize),
            };

            if skip > 0_usize {
                let digits = lexeme.get(skip..).unwrap_or("");
                return i64::from_str_radix(digits, radix).map_err(|_err| make_err());
            }
        }

        // Decimal
        lexeme.parse::<i64>().map_err(|_err| make_err())
    }

    /// Parses a float literal from its lexeme.
    fn parse_float(lexeme: &str) -> Result<f64, Error> {
        // Handle special float literals
        match lexeme {
            "##NaN" => return Ok(f64::NAN),
            "##Inf" => return Ok(f64::INFINITY),
            "##-Inf" => return Ok(f64::NEG_INFINITY),
            _ => {}
        }

        lexeme
            .parse::<f64>()
            .map_err(|_err| Error::new(ErrorKind::InvalidNumber, Span::new(0_usize, lexeme.len())))
    }

    /// Processes escape sequences in a string literal.
    ///
    /// The lexeme includes the surrounding quotes. This function returns
    /// the string content with escapes processed. The `token_span` is used
    /// to calculate accurate error positions within the source.
    fn process_string(lexeme: &str, token_span: Span) -> Result<String, Error> {
        // Remove surrounding quotes
        let content = lexeme
            .get(1_usize..lexeme.len().saturating_sub(1_usize))
            .unwrap_or("");

        let mut result = String::new();
        let mut chars = content.char_indices();

        while let Some((byte_offset, ch)) = chars.next() {
            if ch == '\\' {
                // Calculate source position: token start + opening quote + offset
                let escape_start = token_span
                    .start
                    .saturating_add(1_usize)
                    .saturating_add(byte_offset);

                match chars.next() {
                    Some((_, '\\')) => result.push('\\'),
                    Some((_, '"')) => result.push('"'),
                    Some((_, 'n')) => result.push('\n'),
                    Some((_, 't')) => result.push('\t'),
                    Some((_, 'r')) => result.push('\r'),
                    Some((_, '0')) => result.push('\0'),
                    Some((u_offset, 'u')) => {
                        // Unicode escape: \uXXXX
                        let unicode_start = token_span
                            .start
                            .saturating_add(1_usize)
                            .saturating_add(u_offset);
                        let mut hex = String::new();
                        let mut hex_end = unicode_start.saturating_add(1_usize);
                        for _ in 0_u8..4_u8 {
                            if let Some((offset, digit)) = chars.next() {
                                hex.push(digit);
                                hex_end = token_span
                                    .start
                                    .saturating_add(1_usize)
                                    .saturating_add(offset)
                                    .saturating_add(digit.len_utf8());
                            }
                        }
                        let error_span = Span::new(escape_start, hex_end);
                        let code_point = u32::from_str_radix(&hex, 16_u32).map_err(|_err| {
                            Error::new(ErrorKind::InvalidUnicodeEscape, error_span)
                        })?;
                        let ch = char::from_u32(code_point).ok_or_else(|| {
                            Error::new(ErrorKind::InvalidUnicodeEscape, error_span)
                        })?;
                        result.push(ch);
                    }
                    Some((end_offset, other)) => {
                        // This shouldn't happen if the lexer validated escapes correctly
                        let escape_end = token_span
                            .start
                            .saturating_add(1_usize)
                            .saturating_add(end_offset)
                            .saturating_add(other.len_utf8());
                        return Err(Error::new(
                            ErrorKind::InvalidEscapeSequence(other),
                            Span::new(escape_start, escape_end),
                        ));
                    }
                    None => {
                        // This shouldn't happen if the lexer validated the string
                        return Err(Error::new(
                            ErrorKind::UnterminatedString,
                            Span::new(escape_start, token_span.end),
                        ));
                    }
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    /// Consumes and returns the next token, returning an error if at EOF.
    fn advance(&mut self) -> Result<crate::token::Token<'src>, Error> {
        match self.lexer.next() {
            Some(Ok(token)) => Ok(token),
            Some(Err(err)) => Err(err),
            None => Err(Error::new(
                ErrorKind::UnexpectedEof { expected: "token" },
                Span::new(self.source.len(), self.source.len()),
            )),
        }
    }

    /// Consumes the next token, expecting it to be of the given kind.
    fn expect_token(&mut self, expected: TokenKind) -> Result<crate::token::Token<'src>, Error> {
        let token = self.advance()?;
        if token.kind == expected {
            Ok(token)
        } else {
            Err(Error::new(
                ErrorKind::UnexpectedToken {
                    expected: expected.description(),
                    found: token.kind.description(),
                },
                token.span,
            ))
        }
    }
}

/// Parses all expressions from the source string.
///
/// This is a convenience function that creates a parser and parses all
/// top-level expressions.
#[inline]
#[must_use = "parsing result should be used"]
pub fn parse(source: &str) -> Result<Vec<Spanned<Ast>>, Error> {
    Parser::new(source).parse()
}

/// Parses a single expression from the source string.
///
/// This is a convenience function that creates a parser and parses one
/// expression. Returns an error if there are no expressions.
#[inline]
#[must_use = "parsing result should be used"]
pub fn parse_one(source: &str) -> Result<Spanned<Ast>, Error> {
    Parser::new(source).parse_one()
}
