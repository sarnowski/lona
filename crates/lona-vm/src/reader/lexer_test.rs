// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the Lonala lexer.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::lexer::tokenize;
use super::{LexError, Token, TokenString};

#[test]
fn lex_parens() {
    assert_eq!(tokenize("()").unwrap(), vec![Token::LParen, Token::RParen]);
    assert_eq!(
        tokenize("(())").unwrap(),
        vec![Token::LParen, Token::LParen, Token::RParen, Token::RParen]
    );
}

#[test]
fn lex_quote() {
    assert_eq!(tokenize("'").unwrap(), vec![Token::Quote]);
    assert_eq!(
        tokenize("'()").unwrap(),
        vec![Token::Quote, Token::LParen, Token::RParen]
    );
}

#[test]
fn lex_nil() {
    assert_eq!(tokenize("nil").unwrap(), vec![Token::Nil]);
}

#[test]
fn lex_booleans() {
    assert_eq!(tokenize("true").unwrap(), vec![Token::True]);
    assert_eq!(tokenize("false").unwrap(), vec![Token::False]);
}

#[test]
fn lex_integers() {
    assert_eq!(tokenize("0").unwrap(), vec![Token::Int(0)]);
    assert_eq!(tokenize("42").unwrap(), vec![Token::Int(42)]);
    assert_eq!(tokenize("123456").unwrap(), vec![Token::Int(123_456)]);
    assert_eq!(tokenize("-1").unwrap(), vec![Token::Int(-1)]);
    assert_eq!(tokenize("-999").unwrap(), vec![Token::Int(-999)]);
}

#[test]
fn lex_strings() {
    assert_eq!(
        tokenize("\"hello\"").unwrap(),
        vec![Token::String(TokenString::try_from_str("hello").unwrap())]
    );
    assert_eq!(
        tokenize("\"\"").unwrap(),
        vec![Token::String(TokenString::try_from_str("").unwrap())]
    );
    assert_eq!(
        tokenize("\"a b c\"").unwrap(),
        vec![Token::String(TokenString::try_from_str("a b c").unwrap())]
    );
}

#[test]
fn lex_string_escapes() {
    assert_eq!(
        tokenize("\"a\\nb\"").unwrap(),
        vec![Token::String(TokenString::try_from_str("a\nb").unwrap())]
    );
    assert_eq!(
        tokenize("\"\\t\\r\\\\\\\"\"").unwrap(),
        vec![Token::String(
            TokenString::try_from_str("\t\r\\\"").unwrap()
        )]
    );
}

#[test]
fn lex_symbols() {
    assert_eq!(
        tokenize("foo").unwrap(),
        vec![Token::Symbol(TokenString::try_from_str("foo").unwrap())]
    );
    assert_eq!(
        tokenize("+").unwrap(),
        vec![Token::Symbol(TokenString::try_from_str("+").unwrap())]
    );
    assert_eq!(
        tokenize("my-func").unwrap(),
        vec![Token::Symbol(TokenString::try_from_str("my-func").unwrap())]
    );
    assert_eq!(
        tokenize("x1").unwrap(),
        vec![Token::Symbol(TokenString::try_from_str("x1").unwrap())]
    );
}

#[test]
fn lex_whitespace() {
    assert_eq!(
        tokenize("  (  )  ").unwrap(),
        vec![Token::LParen, Token::RParen]
    );
    assert_eq!(
        tokenize("(\n\t)").unwrap(),
        vec![Token::LParen, Token::RParen]
    );
}

#[test]
fn lex_commas_as_whitespace() {
    assert_eq!(
        tokenize("(1, 2, 3)").unwrap(),
        vec![
            Token::LParen,
            Token::Int(1),
            Token::Int(2),
            Token::Int(3),
            Token::RParen
        ]
    );
}

#[test]
fn lex_comments() {
    assert_eq!(tokenize("; comment\n42").unwrap(), vec![Token::Int(42)]);
    assert_eq!(
        tokenize("1 ; inline\n2").unwrap(),
        vec![Token::Int(1), Token::Int(2)]
    );
}

#[test]
fn lex_complex() {
    assert_eq!(
        tokenize("(+ 1 2)").unwrap(),
        vec![
            Token::LParen,
            Token::Symbol(TokenString::try_from_str("+").unwrap()),
            Token::Int(1),
            Token::Int(2),
            Token::RParen
        ]
    );

    assert_eq!(
        tokenize("'(1 \"two\" nil)").unwrap(),
        vec![
            Token::Quote,
            Token::LParen,
            Token::Int(1),
            Token::String(TokenString::try_from_str("two").unwrap()),
            Token::Nil,
            Token::RParen
        ]
    );
}

#[test]
fn lex_unterminated_string() {
    assert_eq!(
        tokenize("\"hello").unwrap_err(),
        LexError::UnterminatedString
    );
}

#[test]
fn lex_invalid_escape() {
    assert_eq!(
        tokenize("\"\\x\"").unwrap_err(),
        LexError::InvalidEscape('x')
    );
}

#[test]
fn lex_unexpected_char() {
    assert_eq!(tokenize("[").unwrap_err(), LexError::UnexpectedChar('['));
}

#[test]
fn token_string_basic() {
    let s = TokenString::try_from_str("hello").unwrap();
    assert_eq!(s.as_str(), "hello");
}

#[test]
fn token_string_empty() {
    let s = TokenString::new();
    assert_eq!(s.as_str(), "");
}

#[test]
fn token_string_push() {
    let mut s = TokenString::new();
    assert!(s.push('a'));
    assert!(s.push('b'));
    assert_eq!(s.as_str(), "ab");
}

#[test]
fn token_string_too_long() {
    assert!(TokenString::try_from_str(&"x".repeat(64)).is_none());
    assert!(TokenString::try_from_str(&"x".repeat(63)).is_some());
}
