// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! UTF-8 support tests for the Lonala lexer.
//!
//! Tests that UTF-8 symbols and keywords are properly tokenized,
//! and that dangerous Unicode characters are rejected.

extern crate alloc;

use alloc::vec;

use crate::error::Kind as ErrorKind;
use crate::lexer::tokenize;
use crate::token::Kind as TokenKind;

use super::{TEST_SOURCE_ID, kinds, lex};

// ==================== UTF-8 Symbols ====================

#[test]
fn symbol_with_accented_chars() {
    // French word "café" should be a valid symbol
    let tokens = lex("café");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Symbol)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("café"));
}

#[test]
fn symbol_with_greek_letters() {
    // Greek letters for mathematical symbols
    let tokens = lex("λ α β γ");
    assert_eq!(tokens.len(), 4_usize);
    assert!(tokens.iter().all(|token| token.kind == TokenKind::Symbol));
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("λ"));
}

#[test]
fn symbol_with_cyrillic() {
    // Cyrillic word "привет" (hello)
    let tokens = lex("привет");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Symbol)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("привет"));
}

#[test]
fn symbol_with_cjk() {
    // Japanese word "日本語" (Japanese)
    let tokens = lex("日本語");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Symbol)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("日本語"));
}

#[test]
fn symbol_mixed_ascii_and_unicode() {
    // Mixed ASCII and Unicode: "caféLatté"
    let tokens = lex("caféLatté");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Symbol)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("caféLatté"));
}

#[test]
fn symbol_with_unicode_digits() {
    // Symbol can contain Unicode digits (after first char)
    // Arabic-Indic digit 5: ٥
    let tokens = lex("x٥");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Symbol)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("x٥"));
}

// ==================== UTF-8 Keywords ====================

#[test]
fn keyword_with_accented_chars() {
    let tokens = lex(":café");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Keyword)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some(":café"));
}

#[test]
fn keyword_with_greek_letters() {
    let tokens = lex(":λ");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Keyword)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some(":λ"));
}

// ==================== UTF-8 in String Literals ====================

#[test]
fn string_with_utf8() {
    // UTF-8 characters inside strings should be accepted
    let tokens = lex(r#""café""#);
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::String)
    );
    assert_eq!(tokens.first().map(|token| token.lexeme), Some("\"café\""));
}

#[test]
fn string_with_cjk() {
    let tokens = lex(r#""日本語""#);
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::String)
    );
}

#[test]
fn string_with_emoji() {
    // Emoji in strings (not allowed in symbols but fine in strings)
    let tokens = lex(r#""hello 🎉 world""#);
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::String)
    );
}

// ==================== UTF-8 Span Tracking ====================

#[test]
fn span_tracks_utf8_bytes_correctly() {
    // "é" is 2 bytes in UTF-8 (0xC3 0xA9)
    // "café" is 5 bytes: c(1) + a(1) + f(1) + é(2) = 5
    let tokens = lex("café x");
    assert_eq!(tokens.len(), 2_usize);

    // First token "café" spans bytes 0..5
    let first = tokens.first().expect("should have first token");
    assert_eq!(first.lexeme, "café");
    assert_eq!(first.span.start, 0_usize);
    assert_eq!(first.span.end, 5_usize);

    // Second token "x" starts at byte 6 (after space)
    let second = tokens.get(1_usize).expect("should have second token");
    assert_eq!(second.lexeme, "x");
    assert_eq!(second.span.start, 6_usize);
    assert_eq!(second.span.end, 7_usize);
}

// ==================== Dangerous Unicode Rejection ====================

#[test]
fn error_zero_width_space() {
    // Zero-width space (U+200B) should be rejected
    let result = tokenize("foo\u{200B}bar", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{200B}'));
}

#[test]
fn error_zero_width_non_joiner() {
    // Zero-width non-joiner (U+200C) should be rejected
    let result = tokenize("x\u{200C}y", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{200C}'));
}

#[test]
fn error_zero_width_joiner() {
    // Zero-width joiner (U+200D) should be rejected
    let result = tokenize("a\u{200D}b", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{200D}'));
}

#[test]
fn error_byte_order_mark() {
    // BOM / ZWNBSP (U+FEFF) should be rejected
    let result = tokenize("\u{FEFF}x", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{FEFF}'));
}

#[test]
fn error_left_to_right_mark() {
    // LTR mark (U+200E) - bidirectional control
    let result = tokenize("foo\u{200E}bar", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{200E}'));
}

#[test]
fn error_right_to_left_mark() {
    // RTL mark (U+200F) - bidirectional control
    let result = tokenize("foo\u{200F}bar", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{200F}'));
}

#[test]
fn error_left_to_right_override() {
    // LRO (U+202D) - Trojan Source attack vector
    let result = tokenize("x\u{202D}y", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{202D}'));
}

#[test]
fn error_right_to_left_override() {
    // RLO (U+202E) - Trojan Source attack vector
    let result = tokenize("x\u{202E}y", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{202E}'));
}

#[test]
fn error_soft_hyphen() {
    // Soft hyphen (U+00AD) - invisible formatting
    let result = tokenize("foo\u{00AD}bar", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{00AD}'));
}

#[test]
fn error_word_joiner() {
    // Word joiner (U+2060) - invisible
    let result = tokenize("x\u{2060}y", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{2060}'));
}

// ==================== Dangerous Unicode in Comments ====================

#[test]
fn error_zero_width_space_in_comment() {
    // Zero-width space hidden in a comment should still be rejected
    let result = tokenize("; comment with\u{200B}hidden text", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{200B}'));
}

#[test]
fn error_rtl_override_in_comment() {
    // RTL override in comment (could visually hide code)
    let result = tokenize("; \u{202E}reversed comment", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{202E}'));
}

#[test]
fn error_bidi_control_in_comment() {
    // Bidirectional control in comment
    let result = tokenize("; hidden\u{200F}rtl", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::DangerousUnicode('\u{200F}'));
}

#[test]
fn safe_utf8_in_comment() {
    // Regular UTF-8 characters in comments should be fine
    let tokens = lex("; café λ 日本語\n42");
    assert_eq!(tokens.len(), 1_usize);
    assert_eq!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Integer)
    );
}

// ==================== UTF-8 in Expressions ====================

#[test]
fn utf8_in_list() {
    let tokens = lex("(def café 42)");
    assert_eq!(tokens.len(), 5_usize);
    assert_eq!(
        kinds("(def café 42)"),
        vec![
            TokenKind::LeftParen,
            TokenKind::Symbol,
            TokenKind::Symbol,
            TokenKind::Integer,
            TokenKind::RightParen,
        ]
    );
    // Check that "café" is the third token
    assert_eq!(tokens.get(2_usize).map(|token| token.lexeme), Some("café"));
}

#[test]
fn utf8_in_map() {
    let tokens = lex("{:café 1 :λ 2}");
    assert_eq!(tokens.len(), 6_usize);
    assert_eq!(
        kinds("{:café 1 :λ 2}"),
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
