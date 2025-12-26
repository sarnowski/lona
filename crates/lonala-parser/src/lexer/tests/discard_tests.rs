// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the discard reader macro (`#_`) lexer support.

extern crate alloc;

use alloc::vec;

use crate::token::Kind as TokenKind;

use super::{kinds, lex};

// ==================== Discard Reader Macro Tests ====================

#[test]
fn discard_token() {
    let tokens = lex("#_");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Discard)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("#_"));
}

#[test]
fn discard_before_form() {
    let tokens = lex("#_42");
    assert_eq!(tokens.len(), 2_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Discard)
    );
    assert_eq!(
        tokens.get(1_usize).map(|token| token.kind),
        Some(TokenKind::Integer)
    );
}

#[test]
fn discard_chained() {
    let tokens = lex("#_#_1 2");
    assert_eq!(tokens.len(), 4_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Discard)
    );
    assert_eq!(
        tokens.get(1_usize).map(|token| token.kind),
        Some(TokenKind::Discard)
    );
    assert_eq!(
        tokens.get(2_usize).map(|token| token.kind),
        Some(TokenKind::Integer)
    );
    assert_eq!(
        tokens.get(3_usize).map(|token| token.kind),
        Some(TokenKind::Integer)
    );
}

#[test]
fn discard_in_collection() {
    // [1 #_2 3] should produce: [ 1 #_ 2 3 ]
    let tokens = lex("[1 #_2 3]");
    assert_eq!(tokens.len(), 6_usize);
    assert_eq!(
        kinds("[1 #_2 3]"),
        vec![
            TokenKind::LeftBracket,
            TokenKind::Integer,
            TokenKind::Discard,
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::RightBracket,
        ]
    );
}
