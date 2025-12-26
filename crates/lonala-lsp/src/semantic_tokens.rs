// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Semantic token classification for syntax highlighting.
//!
//! This module maps Lonala tokens to LSP semantic token types,
//! enabling rich syntax highlighting in editors.

use lona_core::source;
use lonala_human::docs::is_special_form;
use lonala_parser::Lexer;
use lonala_parser::token::Kind as TokenKind;
use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenType, SemanticTokensFullOptions, SemanticTokensLegend,
    SemanticTokensOptions,
};

use crate::document::Document;

/// Token type indices matching the legend order in [`legend()`].
///
/// The indices must match the position in the `token_types` vector:
/// - 0: `KEYWORD` (special forms, true/false/nil)
/// - 1: `FUNCTION` (reserved for future use)
/// - 2: `VARIABLE` (symbols)
/// - 3: `NUMBER` (integers, floats)
/// - 4: `STRING` (string literals)
/// - 5: `ENUM_MEMBER` (keywords like `:foo`)
/// - 6: `COMMENT` (reserved - lexer skips comments)
/// - 7: `OPERATOR` (quote operators)
pub mod types {
    /// Special forms like `def`, `if`, `fn`, and literals `true`/`false`/`nil`.
    pub const KEYWORD: u32 = 0;
    /// Symbols/variables.
    pub const VARIABLE: u32 = 2;
    /// Numeric literals.
    pub const NUMBER: u32 = 3;
    /// String literals.
    pub const STRING: u32 = 4;
    /// Keywords like `:foo`.
    pub const ENUM_MEMBER: u32 = 5;
    /// Quote operators (`'`, `` ` ``, `~`, `~@`, `#'`, `^`).
    pub const OPERATOR: u32 = 7;
}

/// Returns the semantic token legend for capability registration.
#[inline]
#[must_use]
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::NUMBER,
            SemanticTokenType::STRING,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::OPERATOR,
        ],
        token_modifiers: vec![],
    }
}

/// Returns semantic token options for server capabilities.
#[inline]
#[must_use]
pub fn options() -> SemanticTokensOptions {
    SemanticTokensOptions {
        legend: legend(),
        full: Some(SemanticTokensFullOptions::Bool(true)),
        ..Default::default()
    }
}

/// Compute semantic tokens for a document.
#[inline]
#[must_use]
pub fn compute(doc: &Document) -> Vec<SemanticToken> {
    // Use a dummy source ID since we don't need it for token classification
    let source_id = source::Id::new(0_u32);
    let lexer = Lexer::new(&doc.content, source_id);

    let mut tokens = Vec::new();
    let mut prev_line = 0_u32;
    let mut prev_start = 0_u32;

    for result in lexer {
        let Ok(token) = result else { continue };

        let Some(token_type) = classify_token(&token) else {
            continue;
        };

        let Some(pos) = doc.line_index.offset_to_line_col(token.span.start) else {
            continue;
        };

        let length = token
            .span
            .end
            .saturating_sub(token.span.start)
            .try_into()
            .unwrap_or(u32::MAX);

        let delta_line = pos.line.saturating_sub(prev_line);
        let delta_start = if delta_line == 0 {
            pos.column.saturating_sub(prev_start)
        } else {
            pos.column
        };

        tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });

        prev_line = pos.line;
        prev_start = pos.column;
    }

    tokens
}

fn classify_token(token: &lonala_parser::Token<'_>) -> Option<u32> {
    match token.kind {
        TokenKind::String => Some(types::STRING),
        TokenKind::Integer | TokenKind::Float => Some(types::NUMBER),
        TokenKind::True | TokenKind::False | TokenKind::Nil => Some(types::KEYWORD),
        TokenKind::Keyword => Some(types::ENUM_MEMBER),
        TokenKind::Quote
        | TokenKind::SyntaxQuote
        | TokenKind::Unquote
        | TokenKind::UnquoteSplice
        | TokenKind::VarQuote
        | TokenKind::Caret => Some(types::OPERATOR),
        TokenKind::Symbol => {
            if is_special_form(token.lexeme) {
                Some(types::KEYWORD)
            } else {
                Some(types::VARIABLE)
            }
        }
        // Delimiters don't need semantic highlighting (editor handles via TextMate grammar).
        // List all known variants explicitly, plus wildcard for future variants.
        TokenKind::LeftParen
        | TokenKind::RightParen
        | TokenKind::LeftBracket
        | TokenKind::RightBracket
        | TokenKind::LeftBrace
        | TokenKind::RightBrace
        | TokenKind::SetStart
        | TokenKind::AnonFnStart
        | TokenKind::Discard
        | _ => None,
    }
}
