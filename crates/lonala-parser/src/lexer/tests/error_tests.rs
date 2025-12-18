// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Error case tests for the Lonala lexer.

use crate::error::Kind as ErrorKind;
use crate::lexer::tokenize;

// ==================== Error Cases ====================

#[test]
fn error_unterminated_string() {
    let result = tokenize(r#""unterminated"#);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::UnterminatedString);
}

#[test]
fn error_invalid_escape() {
    let result = tokenize(r#""\q""#);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidEscapeSequence('q'));
}

#[test]
fn error_invalid_unicode_escape() {
    let result = tokenize(r#""\u00""#);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidUnicodeEscape);
}

#[test]
fn error_invalid_hex_number() {
    let result = tokenize("0x");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidNumber);
}

#[test]
fn error_invalid_binary_number() {
    let result = tokenize("0b");
    assert!(result.is_err());
}

#[test]
fn error_invalid_octal_number() {
    let result = tokenize("0o");
    assert!(result.is_err());
}

#[test]
fn error_unexpected_character() {
    let result = tokenize("@");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::UnexpectedCharacter('@'));
}

#[test]
fn error_bare_colon() {
    let result = tokenize(": ");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::UnexpectedCharacter(':'));
}
