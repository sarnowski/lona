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
use crate::error::{Error, Kind as ErrorKind, SourceId, SourceLocation, Span};
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
    /// Identifier for this source (for error reporting).
    source_id: SourceId,
}

impl<'src> Parser<'src> {
    /// Creates a new parser for the given source code.
    ///
    /// The `source_id` identifies which source is being parsed for error reporting.
    #[inline]
    #[must_use]
    pub const fn new(source: &'src str, source_id: SourceId) -> Self {
        Self {
            lexer: Lexer::new(source, source_id),
            source,
            source_id,
        }
    }

    /// Returns the source ID for this parser.
    #[inline]
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Creates a source location from a span.
    #[inline]
    #[must_use]
    const fn location(&self, span: Span) -> SourceLocation {
        SourceLocation::new(self.source_id, span)
    }

    /// Creates a source location from start and end positions.
    #[inline]
    #[must_use]
    const fn location_from(&self, start: usize, end: usize) -> SourceLocation {
        self.location(Span::new(start, end))
    }

    /// Creates a `Spanned` with `full_span` starting at `trivia_start`.
    ///
    /// The `full_span` runs from `trivia_start` (before any leading
    /// whitespace/comments) to `span.end`.
    #[inline]
    const fn spanned_with_trivia<T>(node: T, span: Span, trivia_start: usize) -> Spanned<T> {
        let full_span = Span::new(trivia_start, span.end);
        Spanned::with_full_span(node, span, full_span)
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
                self.location_from(self.source.len(), self.source.len()),
            ));
        }
        self.parse_expr()
    }

    /// Parses a single expression.
    fn parse_expr(&mut self) -> Result<Spanned<Ast>, Error> {
        // Peek triggers trivia skip, so capture trivia_start after peek
        let token = match self.lexer.peek() {
            Some(&Ok(ref token)) => token.clone(),
            Some(&Err(ref err)) => return Err(err.clone()),
            None => {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof {
                        expected: "expression",
                    },
                    self.location_from(self.source.len(), self.source.len()),
                ));
            }
        };

        // Capture trivia_start after peek (peek triggers trivia skip)
        let trivia_start = self.lexer.trivia_start();

        match token.kind {
            // Delimiters - start collections
            TokenKind::LeftParen => self.parse_list(trivia_start),
            TokenKind::LeftBracket => self.parse_vector(trivia_start),
            TokenKind::LeftBrace => self.parse_map(trivia_start),
            TokenKind::SetStart => self.parse_set(trivia_start),

            // Reader macros
            TokenKind::Quote => self.parse_reader_macro("quote", trivia_start),
            TokenKind::SyntaxQuote => self.parse_reader_macro("syntax-quote", trivia_start),
            TokenKind::Unquote => self.parse_reader_macro("unquote", trivia_start),
            TokenKind::UnquoteSplice => self.parse_reader_macro("unquote-splicing", trivia_start),
            TokenKind::Caret => self.parse_metadata(trivia_start),

            // Atoms
            TokenKind::Integer
            | TokenKind::Float
            | TokenKind::String
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::Symbol
            | TokenKind::Keyword => self.parse_atom(trivia_start),

            // Unexpected closing delimiters
            TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                Err(Error::new(
                    ErrorKind::UnexpectedToken {
                        expected: "expression",
                        found: token.kind.description(),
                    },
                    self.location(token.span),
                ))
            }
        }
    }

    /// Parses a list `(...)`.
    fn parse_list(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftParen, TokenKind::RightParen, '(', ')')?;
        Ok(Self::spanned_with_trivia(
            Ast::list(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses a vector `[...]`.
    fn parse_vector(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftBracket, TokenKind::RightBracket, '[', ']')?;
        Ok(Self::spanned_with_trivia(
            Ast::vector(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses a map `{...}`.
    fn parse_map(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
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
    fn parse_set(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
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

    /// Parses a reader macro and expands it to its canonical form.
    ///
    /// For example, `'x` becomes `(quote x)`.
    fn parse_reader_macro(
        &mut self,
        symbol_name: &str,
        trivia_start: usize,
    ) -> Result<Spanned<Ast>, Error> {
        // Consume the reader macro token
        let macro_token = self.advance()?;
        let start = macro_token.span.start;

        // Check that an expression follows
        match self.lexer.peek() {
            None => {
                return Err(Error::new(
                    ErrorKind::ReaderMacroMissingExpr,
                    self.location(macro_token.span),
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
                        self.location(macro_token.span),
                    ));
                }
            }
        }

        // Parse the inner expression (gets its own trivia tracking via parse_expr)
        let inner = self.parse_expr()?;
        let end = inner.span.end;

        // Create the symbol for the reader macro
        let symbol_span = macro_token.span;
        let symbol = Spanned::new(Ast::symbol(symbol_name), symbol_span);

        // Build the list form: (symbol inner)
        let elements = alloc::vec![symbol, inner];
        let span = Span::new(start, end);

        // Use trivia_start from before the reader macro character
        Ok(Self::spanned_with_trivia(
            Ast::list(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses a metadata annotation and the following form.
    ///
    /// Handles:
    /// - `^{:key val} form` - direct map
    /// - `^:keyword form` - shorthand for `^{:keyword true} form`
    /// - `^:a ^:b form` - multiple metadata items (merged right-to-left)
    fn parse_metadata(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        // Consume the initial caret
        let caret_token = self.advance()?;
        let start = caret_token.span.start;

        // Collect all metadata forms (there may be multiple ^)
        let mut meta_forms = Vec::new();
        meta_forms.push(self.parse_single_metadata()?);

        // Check for additional ^ annotations
        loop {
            match self.lexer.peek() {
                Some(&Ok(ref token)) if token.kind == TokenKind::Caret => {
                    let _: crate::token::Token<'_> = self.advance()?; // consume ^
                    meta_forms.push(self.parse_single_metadata()?);
                }
                _ => break,
            }
        }

        // Parse the target form
        let value = self.parse_expr()?;
        let end = value.span.end;

        // Merge metadata (right-to-left: later entries win)
        let merged_meta = Self::merge_metadata_forms(&meta_forms, start);

        let span = Span::new(start, end);
        Ok(Self::spanned_with_trivia(
            Ast::with_meta(merged_meta, value),
            span,
            trivia_start,
        ))
    }

    /// Parses a single metadata item after `^`.
    ///
    /// Returns a `Spanned<Ast>` containing a map.
    fn parse_single_metadata(&mut self) -> Result<Spanned<Ast>, Error> {
        // Capture trivia_start before peeking
        let trivia_start = self.lexer.trivia_start();

        let token = match self.lexer.peek() {
            Some(&Ok(ref token)) => token.clone(),
            Some(&Err(ref err)) => return Err(err.clone()),
            None => {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof {
                        expected: "metadata (map or keyword)",
                    },
                    self.location_from(self.source.len(), self.source.len()),
                ));
            }
        };

        match token.kind {
            // Direct map: ^{:key val}
            TokenKind::LeftBrace => self.parse_map(trivia_start),

            // Keyword shorthand: ^:keyword → {:keyword true}
            TokenKind::Keyword => {
                let keyword_token = self.advance()?;
                let keyword_name = keyword_token.lexeme.get(1_usize..).unwrap_or("");

                // Create {:keyword true}
                let keyword_span = keyword_token.span;
                let keyword_ast = Spanned::new(Ast::keyword(keyword_name), keyword_span);
                let true_ast = Spanned::new(Ast::bool(true), keyword_span);

                let map_span = keyword_span;
                Ok(Spanned::new(
                    Ast::map(alloc::vec![keyword_ast, true_ast]),
                    map_span,
                ))
            }

            // Error: invalid metadata form
            TokenKind::LeftParen
            | TokenKind::RightParen
            | TokenKind::LeftBracket
            | TokenKind::RightBracket
            | TokenKind::RightBrace
            | TokenKind::SetStart
            | TokenKind::Integer
            | TokenKind::Float
            | TokenKind::String
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::Symbol
            | TokenKind::Quote
            | TokenKind::SyntaxQuote
            | TokenKind::Unquote
            | TokenKind::UnquoteSplice
            | TokenKind::Caret => Err(Error::new(
                ErrorKind::InvalidMetadataForm {
                    found: token.kind.description(),
                },
                self.location(token.span),
            )),
        }
    }

    /// Merges multiple metadata maps into one.
    ///
    /// Later entries override earlier ones for duplicate keys (right-to-left).
    fn merge_metadata_forms(forms: &[Spanned<Ast>], start: usize) -> Spanned<Ast> {
        // Single form - return as-is
        if let &[ref single] = forms {
            return single.clone();
        }

        // Collect all key-value pairs from all maps
        let mut all_pairs: Vec<Spanned<Ast>> = Vec::new();
        for form in forms {
            if let Ast::Map(ref pairs) = form.node {
                all_pairs.extend(pairs.iter().cloned());
            }
        }

        // Deduplicate: keep last occurrence of each key (right-to-left means later wins)
        // Process pairs in reverse, keeping first occurrence seen (which is last in original order)
        let mut seen_keys: Vec<Ast> = Vec::new();
        let mut deduped: Vec<Spanned<Ast>> = Vec::new();

        // Iterate in reverse over key-value pairs
        let pairs_vec: Vec<_> = all_pairs.into_iter().collect();
        let mut idx = pairs_vec.len();
        while idx >= 2_usize {
            idx = idx.saturating_sub(2_usize);
            // These gets are safe because idx is always < pairs_vec.len() - 1
            // and we control idx to always be a valid even index
            if let (Some(key), Some(val)) = (
                pairs_vec.get(idx),
                pairs_vec.get(idx.saturating_add(1_usize)),
            ) && !seen_keys.contains(&key.node)
            {
                seen_keys.push(key.node.clone());
                // Push value then key so that after final reverse they're in correct order
                deduped.push(val.clone());
                deduped.push(key.clone());
            }
        }

        // Reverse to restore correct order
        deduped.reverse();

        // Determine span for merged map
        let end = forms.last().map_or(start, |form| form.span.end);
        let span = Span::new(start, end);

        Spanned::new(Ast::map(deduped), span)
    }

    /// Parses an atom (literal, symbol, or keyword).
    fn parse_atom(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let token = self.advance()?;
        let token_location = self.location(token.span);

        let ast = match token.kind {
            TokenKind::Integer => {
                let value = Self::parse_integer(token.lexeme, token_location)?;
                Ast::integer(value)
            }
            TokenKind::Float => {
                let value = Self::parse_float(token.lexeme, token_location)?;
                Ast::float(value)
            }
            TokenKind::String => {
                let value = self.process_string(token.lexeme, token.span)?;
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
            | TokenKind::SetStart
            | TokenKind::RightParen
            | TokenKind::RightBracket
            | TokenKind::RightBrace
            | TokenKind::Quote
            | TokenKind::SyntaxQuote
            | TokenKind::Unquote
            | TokenKind::UnquoteSplice
            | TokenKind::Caret => {
                return Err(Error::new(
                    ErrorKind::UnexpectedToken {
                        expected: "atom",
                        found: token.kind.description(),
                    },
                    token_location,
                ));
            }
        };

        Ok(Self::spanned_with_trivia(ast, token.span, trivia_start))
    }

    /// Parses an integer literal from its lexeme.
    fn parse_integer(lexeme: &str, location: SourceLocation) -> Result<i64, Error> {
        let make_err = || Error::new(ErrorKind::InvalidNumber, location);

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
    fn parse_float(lexeme: &str, location: SourceLocation) -> Result<f64, Error> {
        // Handle special float literals
        match lexeme {
            "##NaN" => return Ok(f64::NAN),
            "##Inf" => return Ok(f64::INFINITY),
            "##-Inf" => return Ok(f64::NEG_INFINITY),
            _ => {}
        }

        lexeme
            .parse::<f64>()
            .map_err(|_err| Error::new(ErrorKind::InvalidNumber, location))
    }

    /// Processes escape sequences in a string literal.
    ///
    /// The lexeme includes the surrounding quotes. This function returns
    /// the string content with escapes processed. The `token_span` is used
    /// to calculate accurate error positions within the source.
    fn process_string(&self, lexeme: &str, token_span: Span) -> Result<String, Error> {
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
                        let error_location = self.location_from(escape_start, hex_end);
                        let code_point = u32::from_str_radix(&hex, 16_u32).map_err(|_err| {
                            Error::new(ErrorKind::InvalidUnicodeEscape, error_location)
                        })?;
                        let ch = char::from_u32(code_point).ok_or_else(|| {
                            Error::new(ErrorKind::InvalidUnicodeEscape, error_location)
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
                            self.location_from(escape_start, escape_end),
                        ));
                    }
                    None => {
                        // This shouldn't happen if the lexer validated the string
                        return Err(Error::new(
                            ErrorKind::UnterminatedString,
                            self.location_from(escape_start, token_span.end),
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
                self.location_from(self.source.len(), self.source.len()),
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
                self.location(token.span),
            ))
        }
    }
}

/// Parses all expressions from the source string.
///
/// This is a convenience function that creates a parser and parses all
/// top-level expressions.
///
/// The `source_id` identifies which source is being parsed for error reporting.
#[inline]
#[must_use = "parsing result should be used"]
pub fn parse(source: &str, source_id: SourceId) -> Result<Vec<Spanned<Ast>>, Error> {
    Parser::new(source, source_id).parse()
}

/// Parses a single expression from the source string.
///
/// This is a convenience function that creates a parser and parses one
/// expression. Returns an error if there are no expressions.
///
/// The `source_id` identifies which source is being parsed for error reporting.
#[inline]
#[must_use = "parsing result should be used"]
pub fn parse_one(source: &str, source_id: SourceId) -> Result<Spanned<Ast>, Error> {
    Parser::new(source, source_id).parse_one()
}
