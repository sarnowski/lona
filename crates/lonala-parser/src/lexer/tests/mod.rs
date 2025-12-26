// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Lonala lexer.

mod discard_tests;
mod error_tests;
mod token_tests;

extern crate alloc;

use alloc::vec::Vec;

use crate::error::SourceId;
use crate::lexer::tokenize;
use crate::token::{Kind as TokenKind, Token};

/// Test source ID for all lexer tests.
const TEST_SOURCE_ID: SourceId = SourceId::new(0_u32);

/// Helper to tokenize and unwrap.
pub fn lex(source: &str) -> Vec<Token<'_>> {
    tokenize(source, TEST_SOURCE_ID).expect("lexing should succeed")
}

/// Helper to get token kinds.
pub fn kinds(source: &str) -> Vec<TokenKind> {
    lex(source).into_iter().map(|token| token.kind).collect()
}
