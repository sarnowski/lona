// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Token parsing tests for the Lonala lexer.

extern crate alloc;

use alloc::vec;

use crate::error::Span;
use crate::lexer::Lexer;
use crate::token::Kind as TokenKind;

use super::{kinds, lex};

// ==================== Empty and Whitespace ====================

#[test]
fn empty_input() {
    assert!(lex("").is_empty());
}

#[test]
fn whitespace_only() {
    assert!(lex("   \t\n\r  ").is_empty());
}

#[test]
fn commas_are_whitespace() {
    assert!(lex("  ,  ,  ").is_empty());
}

// ==================== Comments ====================

#[test]
fn comment_only() {
    assert!(lex("; this is a comment").is_empty());
}

#[test]
fn comment_at_end_of_line() {
    let tokens = lex("42 ; comment\n43");
    assert_eq!(tokens.len(), 2_usize);
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("42"));
    assert_eq!(tokens.get(1_usize).map(|token| token.lexeme), Some("43"));
}

// ==================== Delimiters ====================

#[test]
fn delimiters() {
    assert_eq!(
        kinds("()[]{}"),
        vec![
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBracket,
            TokenKind::RightBracket,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
        ]
    );
}

// ==================== Integers ====================

#[test]
fn integer_decimal() {
    let tokens = lex("42 0 123");
    assert_eq!(tokens.len(), 3_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Integer));
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("42"));
    assert_eq!(tokens.get(1_usize).map(|token| token.lexeme), Some("0"));
    assert_eq!(tokens.get(2_usize).map(|token| token.lexeme), Some("123"));
}

#[test]
fn integer_negative() {
    let tokens = lex("-42");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Integer)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("-42"));
}

#[test]
fn integer_hex() {
    let tokens = lex("0xFF 0x1a2B");
    assert_eq!(tokens.len(), 2_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Integer));
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("0xFF"));
    assert_eq!(
        tokens.get(1_usize).map(|token| token.lexeme),
        Some("0x1a2B")
    );
}

#[test]
fn integer_binary() {
    let tokens = lex("0b1010 0B11");
    assert_eq!(tokens.len(), 2_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Integer));
}

#[test]
fn integer_octal() {
    let tokens = lex("0o755 0O17");
    assert_eq!(tokens.len(), 2_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Integer));
}

// ==================== Floats ====================

#[test]
fn float_simple() {
    let tokens = lex("3.14 0.5");
    assert_eq!(tokens.len(), 2_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Float));
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("3.14"));
}

#[test]
fn float_negative() {
    let tokens = lex("-3.14");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Float)
    );
}

#[test]
fn float_scientific() {
    let tokens = lex("1e10 2.5e-3 1E+5");
    assert_eq!(tokens.len(), 3_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Float));
}

#[test]
fn float_special_nan() {
    let tokens = lex("##NaN");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Float)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("##NaN"));
}

#[test]
fn float_special_inf() {
    let tokens = lex("##Inf ##-Inf");
    assert_eq!(tokens.len(), 2_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Float));
}

// ==================== Strings ====================

#[test]
fn string_empty() {
    let tokens = lex(r#""""#);
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::String)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("\"\""));
}

#[test]
fn string_simple() {
    let tokens = lex(r#""hello""#);
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("\"hello\""));
}

#[test]
fn string_with_escapes() {
    let tokens = lex(r#""hello\nworld""#);
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::String)
    );
}

#[test]
fn string_with_unicode_escape() {
    let tokens = lex(r#""\u0041""#);
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::String)
    );
}

#[test]
fn string_with_escaped_quote() {
    let tokens = lex(r#""say \"hi\"""#);
    assert_eq!(tokens.len(), 1_usize);
}

// ==================== Booleans and Nil ====================

#[test]
fn boolean_true() {
    let tokens = lex("true");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::True)
    );
}

#[test]
fn boolean_false() {
    let tokens = lex("false");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::False)
    );
}

#[test]
fn nil_literal() {
    let tokens = lex("nil");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(tokens.first().map(|token| token.kind), Some(TokenKind::Nil));
}

// ==================== Symbols ====================

#[test]
fn symbol_simple() {
    let tokens = lex("foo bar baz");
    assert_eq!(tokens.len(), 3_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Symbol));
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("foo"));
}

#[test]
fn symbol_operators() {
    let tokens = lex("+ - * / < > = <= >= !=");
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Symbol));
}

#[test]
fn symbol_with_special_chars() {
    let tokens = lex("update! empty? ->arrow *special*");
    assert_eq!(tokens.len(), 4_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Symbol));
}

#[test]
fn symbol_namespaced() {
    let tokens = lex("ns/name foo.bar/baz");
    assert_eq!(tokens.len(), 2_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Symbol));
}

// ==================== Keywords ====================

#[test]
fn keyword_simple() {
    let tokens = lex(":foo :bar");
    assert_eq!(tokens.len(), 2_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Keyword));
    assert_eq!(tokens.first().map(|token| token.lexeme), Some(":foo"));
}

#[test]
fn keyword_namespaced() {
    let tokens = lex(":ns/name");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Keyword)
    );
}

#[test]
fn keyword_kebab_case() {
    let tokens = lex(":kebab-case");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.lexeme),
        Some(":kebab-case")
    );
}

// ==================== Reader Macros ====================

#[test]
fn quote() {
    let tokens = lex("'x");
    assert_eq!(tokens.len(), 2_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Quote)
    );
    assert_eq!(
        tokens.get(1_usize).map(|token| token.kind),
        Some(TokenKind::Symbol)
    );
}

#[test]
fn syntax_quote() {
    let tokens = lex("`x");
    assert_eq!(tokens.len(), 2_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::SyntaxQuote)
    );
}

#[test]
fn unquote() {
    let tokens = lex("~x");
    assert_eq!(tokens.len(), 2_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Unquote)
    );
}

#[test]
fn unquote_splice() {
    let tokens = lex("~@x");
    assert_eq!(tokens.len(), 2_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::UnquoteSplice)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("~@"));
}

// ==================== Complex Expressions ====================

#[test]
fn simple_list() {
    let tokens = lex("(+ 1 2)");
    assert_eq!(tokens.len(), 5_usize);
    assert_eq!(
        kinds("(+ 1 2)"),
        vec![
            TokenKind::LeftParen,
            TokenKind::Symbol,
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::RightParen,
        ]
    );
}

#[test]
fn nested_list() {
    let tokens = lex("(def x (+ 1 2))");
    assert_eq!(tokens.len(), 9_usize);
}

#[test]
fn map_literal() {
    let tokens = lex("{:a 1 :b 2}");
    assert_eq!(tokens.len(), 6_usize);
    assert_eq!(
        kinds("{:a 1 :b 2}"),
        vec![
            TokenKind::LeftBrace,
            TokenKind::Keyword,
            TokenKind::Integer,
            TokenKind::Keyword,
            TokenKind::Integer,
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn vector_literal() {
    let tokens = lex("[1 2 3]");
    assert_eq!(tokens.len(), 5_usize);
}

#[test]
fn quoted_list() {
    let tokens = lex("'(1 2 3)");
    assert_eq!(tokens.len(), 6_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Quote)
    );
}

#[test]
fn function_definition() {
    let tokens = lex("(defn foo [x] x)");
    assert_eq!(tokens.len(), 8_usize);
}

// ==================== Span Tests ====================

#[test]
fn span_tracking() {
    let tokens = lex("foo bar");
    assert_eq!(
        tokens.first().map(|token| token.span),
        Some(Span::new(0, 3))
    );
    assert_eq!(
        tokens.get(1_usize).map(|token| token.span),
        Some(Span::new(4, 7))
    );
}

#[test]
fn span_with_unicode() {
    let tokens = lex("hello"); // simple ascii first
    assert_eq!(tokens.first().map(|token| token.span.len()), Some(5_usize));
}

// ==================== Peek Tests ====================

#[test]
fn peek_does_not_consume() {
    let mut lexer = Lexer::new("foo bar");
    let peeked = lexer.peek().cloned();
    let next = lexer.next();
    assert_eq!(
        peeked
            .as_ref()
            .and_then(|result| result.as_ref().ok().map(|token| token.lexeme)),
        Some("foo")
    );
    assert_eq!(
        next.as_ref()
            .and_then(|result| result.as_ref().ok().map(|token| token.lexeme)),
        Some("foo")
    );
}

#[test]
fn peek_at_end_returns_none() {
    let mut lexer = Lexer::new("");
    assert!(lexer.peek().is_none());
}
