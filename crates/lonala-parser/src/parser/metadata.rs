// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Metadata annotation parsing.

extern crate alloc;

use alloc::vec::Vec;

use crate::ast::{Ast, Spanned};
use crate::error::{Error, Kind as ErrorKind, Span};
use crate::token::Kind as TokenKind;

use super::Parser;

impl Parser<'_> {
    /// Parses a metadata annotation and the following form.
    ///
    /// Handles:
    /// - `^{:key val} form` - direct map
    /// - `^:keyword form` - shorthand for `^{:keyword true} form`
    /// - `^:a ^:b form` - multiple metadata items (merged right-to-left)
    pub(super) fn parse_metadata(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
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
    pub(super) fn parse_single_metadata(&mut self) -> Result<Spanned<Ast>, Error> {
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
    pub(super) fn merge_metadata_forms(forms: &[Spanned<Ast>], start: usize) -> Spanned<Ast> {
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
}
