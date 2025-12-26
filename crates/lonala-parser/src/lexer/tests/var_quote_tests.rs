// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the var quote (`#'`) reader macro lexer support.

extern crate alloc;

use alloc::vec;

use super::{kinds, lex};
use crate::token::Kind as TokenKind;

#[test]
fn var_quote_simple() {
    let tokens = lex("#'x");
    assert_eq!(tokens.len(), 2_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::VarQuote)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("#'"));
    assert_eq!(
        tokens.get(1_usize).map(|token| token.kind),
        Some(TokenKind::Symbol)
    );
}

#[test]
fn var_quote_with_qualified_symbol() {
    let tokens = lex("#'foo/bar");
    assert_eq!(tokens.len(), 2_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::VarQuote)
    );
    assert_eq!(
        tokens.get(1_usize).map(|token| token.lexeme),
        Some("foo/bar")
    );
}

#[test]
fn var_quote_in_expression() {
    // (var-get #'x) has 5 tokens: (, var-get, #', x, )
    let tokens = lex("(var-get #'x)");
    assert_eq!(tokens.len(), 5_usize);
    assert_eq!(
        tokens.get(2_usize).map(|token| token.kind),
        Some(TokenKind::VarQuote)
    );
}

#[test]
fn other_hash_forms_still_work_after_var_quote() {
    // Ensure all other #-prefixed forms still work
    assert_eq!(kinds("#("), vec![TokenKind::AnonFnStart]);
    assert_eq!(kinds("#{"), vec![TokenKind::SetStart]);
    assert_eq!(kinds("#_"), vec![TokenKind::Discard]);
    assert_eq!(kinds("##NaN"), vec![TokenKind::Float]);
    assert_eq!(kinds("##Inf"), vec![TokenKind::Float]);
    assert_eq!(kinds("##-Inf"), vec![TokenKind::Float]);
}
