// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Parser for Lonala source code.
//!
//! Converts a token stream into Lonala values.

use super::lexer::{LexError, Lexer, Token};
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Err, Ok};

/// Maximum number of elements in a list literal.
///
/// This limit exists because we collect list elements on the stack before
/// building the cons list. A future optimization could build the list
/// incrementally to remove this limit.
const MAX_LIST_ELEMENTS: usize = 64;

/// Parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Unexpected end of input.
    UnexpectedEof,
    /// Unexpected token.
    UnexpectedToken(Token),
    /// Unmatched right parenthesis.
    UnmatchedRParen,
    /// Out of memory.
    OutOfMemory,
    /// List literal exceeds maximum element count.
    ListTooLong,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of input"),
            Self::UnexpectedToken(t) => write!(f, "unexpected token: {t:?}"),
            Self::UnmatchedRParen => write!(f, "unmatched )"),
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::ListTooLong => write!(f, "list exceeds {MAX_LIST_ELEMENTS} elements"),
        }
    }
}

/// Combined read error (lexer + parser).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// Lexer error.
    Lex(LexError),
    /// Parser error.
    Parse(ParseError),
}

impl core::fmt::Display for ReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Lex(e) => write!(f, "{e}"),
            Self::Parse(e) => write!(f, "{e}"),
        }
    }
}

impl From<LexError> for ReadError {
    fn from(e: LexError) -> Self {
        Self::Lex(e)
    }
}

impl From<ParseError> for ReadError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

/// Parser state.
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    /// Lookahead token.
    lookahead: Option<Token>,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given input.
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            lexer: Lexer::new(input),
            lookahead: None,
        }
    }

    /// Read one expression.
    ///
    /// Returns `None` if at end of input.
    ///
    /// # Errors
    ///
    /// Returns `ReadError` if the input contains invalid syntax or if
    /// memory allocation fails.
    pub fn read<M: MemorySpace>(
        &mut self,
        proc: &mut Process,
        mem: &mut M,
    ) -> Result<Option<Value>, ReadError> {
        let token = match self.peek()? {
            Some(t) => t.clone(),
            None => return Ok(None),
        };
        self.advance();

        match token {
            Token::Nil => Ok(Some(Value::nil())),
            Token::True => Ok(Some(Value::bool(true))),
            Token::False => Ok(Some(Value::bool(false))),
            Token::Int(n) => Ok(Some(Value::int(n))),
            Token::String(s) => {
                let value = proc
                    .alloc_string(mem, s.as_str())
                    .ok_or(ParseError::OutOfMemory)?;
                Ok(Some(value))
            }
            Token::Symbol(s) => {
                let value = proc
                    .alloc_symbol(mem, s.as_str())
                    .ok_or(ParseError::OutOfMemory)?;
                Ok(Some(value))
            }
            Token::Quote => {
                // 'expr => (quote expr)
                let expr = self.read(proc, mem)?.ok_or(ParseError::UnexpectedEof)?;
                let quote_sym = proc
                    .alloc_symbol(mem, "quote")
                    .ok_or(ParseError::OutOfMemory)?;
                // Build (quote expr) = Pair(quote, Pair(expr, nil))
                let inner = proc
                    .alloc_pair(mem, expr, Value::nil())
                    .ok_or(ParseError::OutOfMemory)?;
                let outer = proc
                    .alloc_pair(mem, quote_sym, inner)
                    .ok_or(ParseError::OutOfMemory)?;
                Ok(Some(outer))
            }
            Token::LParen => self.read_list(proc, mem),
            Token::RParen => Err(ParseError::UnmatchedRParen.into()),
        }
    }

    fn read_list<M: MemorySpace>(
        &mut self,
        proc: &mut Process,
        mem: &mut M,
    ) -> Result<Option<Value>, ReadError> {
        // Collect elements on stack before building cons list
        let mut elements = [Value::nil(); MAX_LIST_ELEMENTS];
        let mut count = 0;

        loop {
            match self.peek()? {
                None => return Err(ParseError::UnexpectedEof.into()),
                Some(Token::RParen) => {
                    self.advance();
                    break;
                }
                Some(_) => {
                    if count >= elements.len() {
                        return Err(ParseError::ListTooLong.into());
                    }
                    let elem = self.read(proc, mem)?.ok_or(ParseError::UnexpectedEof)?;
                    elements[count] = elem;
                    count += 1;
                }
            }
        }

        // Build list from back to front: (a b c) = Pair(a, Pair(b, Pair(c, nil)))
        let mut result = Value::nil();
        for i in (0..count).rev() {
            result = proc
                .alloc_pair(mem, elements[i], result)
                .ok_or(ParseError::OutOfMemory)?;
        }

        Ok(Some(result))
    }

    fn peek(&mut self) -> Result<Option<&Token>, LexError> {
        if self.lookahead.is_none() {
            self.lookahead = self.lexer.next_token()?;
        }
        Ok(self.lookahead.as_ref())
    }

    const fn advance(&mut self) {
        self.lookahead = None;
    }
}

/// Read a single expression from a string.
///
/// # Errors
///
/// Returns an error if the input contains invalid syntax.
pub fn read<M: MemorySpace>(
    input: &str,
    proc: &mut Process,
    mem: &mut M,
) -> Result<Option<Value>, ReadError> {
    let mut parser = Parser::new(input);
    parser.read(proc, mem)
}
