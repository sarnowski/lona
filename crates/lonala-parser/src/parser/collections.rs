// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Collection parsing (lists, vectors, maps, sets).

extern crate alloc;

use alloc::vec::Vec;

use crate::ast::{Ast, Spanned};
use crate::error::{Error, Kind as ErrorKind, Span};
use crate::token::Kind as TokenKind;

use super::Parser;

impl Parser<'_> {
    /// Parses a list `(...)`.
    pub(super) fn parse_list(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftParen, TokenKind::RightParen, '(', ')')?;
        Ok(Self::spanned_with_trivia(
            Ast::list(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses a vector `[...]`.
    pub(super) fn parse_vector(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftBracket, TokenKind::RightBracket, '[', ']')?;
        Ok(Self::spanned_with_trivia(
            Ast::vector(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses a map `{...}`.
    pub(super) fn parse_map(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftBrace, TokenKind::RightBrace, '{', '}')?;

        // Maps must have an even number of elements
        if !elements.len().is_multiple_of(2_usize) {
            return Err(Error::new(ErrorKind::OddMapEntries, self.location(span)));
        }

        // Check for duplicate keys (keys are at indices 0, 2, 4, ...)
        // Uses O(n²) comparison but n is typically small for literal maps
        let keys: Vec<&Spanned<Ast>> = elements.iter().step_by(2_usize).collect();
        for (i, key) in keys.iter().enumerate() {
            for prev in keys.iter().take(i) {
                if key.node == prev.node {
                    return Err(Error::new(
                        ErrorKind::DuplicateMapKey,
                        self.location(key.span),
                    ));
                }
            }
        }

        Ok(Self::spanned_with_trivia(
            Ast::map(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses a set `#{...}`.
    pub(super) fn parse_set(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::SetStart, TokenKind::RightBrace, '#', '}')?;

        // Check for duplicate AST nodes (O(n²) but n is typically small)
        for (i, elem) in elements.iter().enumerate() {
            for prev in elements.iter().take(i) {
                if elem.node == prev.node {
                    return Err(Error::new(
                        ErrorKind::DuplicateSetElement,
                        self.location(elem.span),
                    ));
                }
            }
        }

        Ok(Self::spanned_with_trivia(
            Ast::set(elements),
            span,
            trivia_start,
        ))
    }

    /// Helper to parse a collection with the given delimiters.
    pub(super) fn parse_collection(
        &mut self,
        open_kind: TokenKind,
        close_kind: TokenKind,
        open_char: char,
        close_char: char,
    ) -> Result<(Vec<Spanned<Ast>>, Span), Error> {
        // Consume opening delimiter
        let open_token = self.expect_token(open_kind)?;
        let start = open_token.span.start;
        let opener_location = self.location(open_token.span);

        let mut elements = Vec::new();

        loop {
            match self.lexer.peek() {
                None => {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof {
                            expected: close_kind.description(),
                        },
                        self.location_from(self.source.len(), self.source.len()),
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
                        // Copy span before calling self.location() to avoid borrow conflict
                        let error_span = token.span;
                        return Err(Error::new(
                            ErrorKind::UnmatchedDelimiter {
                                opener: open_char,
                                opener_location,
                                expected: close_char,
                                found: found_char,
                            },
                            self.location(error_span),
                        ));
                    }

                    // Parse element
                    elements.push(self.parse_expr()?);
                }
            }
        }
    }
}
