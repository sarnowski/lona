// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Lonala lexer.

mod error_tests;
mod token_tests;

extern crate alloc;

use alloc::vec::Vec;

use crate::lexer::tokenize;
use crate::token::{Kind as TokenKind, Token};

/// Helper to tokenize and unwrap.
pub fn lex(source: &str) -> Vec<Token<'_>> {
    tokenize(source).expect("lexing should succeed")
}

/// Helper to get token kinds.
pub fn kinds(source: &str) -> Vec<TokenKind> {
    lex(source).into_iter().map(|token| token.kind).collect()
}
