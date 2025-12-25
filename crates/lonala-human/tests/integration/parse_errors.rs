// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Parser error integration tests.

use lona_core::symbol::Interner;
use lonala_human::{Config, render};
use lonala_parser::error::{Error as ParserError, Kind as ParserKind};

use super::{create_registry, loc};

#[test]
fn format_parser_unexpected_character() {
    let (registry, source_id) = create_registry("<repl>", "(@foo)");
    let interner = Interner::new();
    let config = Config::new();

    // '@' is at position 1
    let error = ParserError::new(ParserKind::UnexpectedCharacter('@'), loc(source_id, 1, 2));

    let output = render(&error, &registry, &interner, &config);

    assert!(
        output.contains("error[UnexpectedCharacter]"),
        "Output should contain variant name"
    );
    assert!(
        output.contains("unexpected character '@'"),
        "Output should contain message"
    );
    assert!(
        output.contains("--> <repl>:1:2"),
        "Output should contain location"
    );
    assert!(
        output.contains("(@foo)"),
        "Output should contain source line"
    );
    assert!(output.contains('^'), "Output should contain underline");
}

#[test]
fn format_parser_unterminated_string() {
    let (registry, source_id) = create_registry("<repl>", "(print \"hello)");
    let interner = Interner::new();
    let config = Config::new();

    // String starts at position 7
    let error = ParserError::new(ParserKind::UnterminatedString, loc(source_id, 7, 14));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[UnterminatedString]"));
    assert!(output.contains("unterminated string literal"));
    assert!(output.contains("--> <repl>:1:8"));
    assert!(output.contains("= note:"));
}

#[test]
fn format_parser_invalid_escape_sequence() {
    let (registry, source_id) = create_registry("<repl>", r#""hello\qworld""#);
    let interner = Interner::new();
    let config = Config::new();

    // \q is at positions 6-7
    let error = ParserError::new(ParserKind::InvalidEscapeSequence('q'), loc(source_id, 6, 8));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[InvalidEscapeSequence]"));
    assert!(output.contains("invalid escape sequence '\\q'"));
    assert!(output.contains("= note:"));
}

#[test]
fn format_parser_invalid_number() {
    let (registry, source_id) = create_registry("<repl>", "(+ 123abc 5)");
    let interner = Interner::new();
    let config = Config::new();

    // Invalid number at positions 3-9
    let error = ParserError::new(ParserKind::InvalidNumber, loc(source_id, 3, 9));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[InvalidNumber]"));
    assert!(output.contains("invalid numeric literal"));
}

#[test]
fn format_parser_unmatched_delimiter() {
    let (registry, source_id) = create_registry("<repl>", "(foo [bar)");
    let interner = Interner::new();
    let config = Config::new();

    let opener_location = loc(source_id, 5, 6); // '['
    let error = ParserError::new(
        ParserKind::UnmatchedDelimiter {
            opener: '[',
            opener_location,
            expected: ']',
            found: ')',
        },
        loc(source_id, 9, 10), // ')'
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[UnmatchedDelimiter]"));
    assert!(output.contains("expected ']' to match '['"));
    assert!(output.contains("found ')'"));
}

#[test]
fn format_parser_unexpected_eof() {
    let (registry, source_id) = create_registry("<repl>", "(foo bar");
    let interner = Interner::new();
    let config = Config::new();

    // EOF at the end
    let error = ParserError::new(
        ParserKind::UnexpectedEof {
            expected: "closing delimiter",
        },
        loc(source_id, 8, 8),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[UnexpectedEof]"));
    assert!(output.contains("unexpected end of input"));
    assert!(output.contains("= help:"));
}

#[test]
fn format_parser_odd_map_entries() {
    let (registry, source_id) = create_registry("<repl>", "{:a 1 :b}");
    let interner = Interner::new();
    let config = Config::new();

    let error = ParserError::new(ParserKind::OddMapEntries, loc(source_id, 0, 9));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[OddMapEntries]"));
    assert!(output.contains("map literal must have an even number of elements"));
    assert!(output.contains("= help:"));
}

#[test]
fn format_parser_reader_macro_missing_expr() {
    let (registry, source_id) = create_registry("<repl>", "'");
    let interner = Interner::new();
    let config = Config::new();

    let error = ParserError::new(ParserKind::ReaderMacroMissingExpr, loc(source_id, 0, 1));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[ReaderMacroMissingExpr]"));
    assert!(output.contains("reader macro must be followed by an expression"));
}
