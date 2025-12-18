// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Error case tests for the Lonala lexer.

use crate::error::{Kind as ErrorKind, SourceId};
use crate::lexer::tokenize;

/// Test source ID for error tests.
const TEST_SOURCE_ID: SourceId = SourceId::new(0_u32);

// ==================== Error Cases ====================

#[test]
fn error_unterminated_string() {
    let result = tokenize(r#""unterminated"#, TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::UnterminatedString);
    assert_eq!(err.source_id(), TEST_SOURCE_ID);
}

#[test]
fn error_invalid_escape() {
    let result = tokenize(r#""\q""#, TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidEscapeSequence('q'));
    assert_eq!(err.source_id(), TEST_SOURCE_ID);
}

#[test]
fn error_invalid_unicode_escape() {
    let result = tokenize(r#""\u00""#, TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidUnicodeEscape);
    assert_eq!(err.source_id(), TEST_SOURCE_ID);
}

#[test]
fn error_invalid_hex_number() {
    let result = tokenize("0x", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidNumber);
    assert_eq!(err.source_id(), TEST_SOURCE_ID);
}

#[test]
fn error_invalid_binary_number() {
    let result = tokenize("0b", TEST_SOURCE_ID);
    assert!(result.is_err());
}

#[test]
fn error_invalid_octal_number() {
    let result = tokenize("0o", TEST_SOURCE_ID);
    assert!(result.is_err());
}

#[test]
fn error_unexpected_character() {
    let result = tokenize("@", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::UnexpectedCharacter('@'));
    assert_eq!(err.source_id(), TEST_SOURCE_ID);
}

#[test]
fn error_bare_colon() {
    let result = tokenize(": ", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::UnexpectedCharacter(':'));
    assert_eq!(err.source_id(), TEST_SOURCE_ID);
}
