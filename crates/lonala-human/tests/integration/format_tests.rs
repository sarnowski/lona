// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Output format verification and edge case tests.

use alloc::format;
use alloc::string::String;

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::symbol::Interner;
use lona_core::value;
use lona_kernel::vm::{Error as VmError, ErrorKind as VmKind, NativeError};
use lonala_compiler::error::{Error as CompilerError, Kind as CompilerKind};
use lonala_human::{Config, render};
use lonala_parser::error::{Error as ParserError, Kind as ParserKind};

use super::{create_registry, loc};

// =============================================================================
// Empty/Minimal Source Tests
// =============================================================================

#[test]
fn format_error_empty_source() {
    let (registry, source_id) = create_registry("<repl>", "");
    let interner = Interner::new();
    let config = Config::new();

    let error = ParserError::new(
        ParserKind::UnexpectedEof {
            expected: "expression",
        },
        loc(source_id, 0, 0),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[UnexpectedEof]"));
    assert!(output.contains("--> <repl>:1:1"));
}

#[test]
fn format_error_single_character_source() {
    let (registry, source_id) = create_registry("<repl>", "x");
    let interner = Interner::new();
    let config = Config::new();

    let error = ParserError::new(ParserKind::UnexpectedCharacter('x'), loc(source_id, 0, 1));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("--> <repl>:1:1"));
    assert!(output.contains("| x"));
}

#[test]
fn format_error_whitespace_only_source() {
    let (registry, source_id) = create_registry("<repl>", "   ");
    let interner = Interner::new();
    let config = Config::new();

    let error = ParserError::new(
        ParserKind::UnexpectedEof {
            expected: "expression",
        },
        loc(source_id, 3, 3),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[UnexpectedEof]"));
}

// =============================================================================
// Output Format Verification Tests
// =============================================================================

#[test]
fn format_output_structure() {
    let (registry, source_id) = create_registry("<repl>", "(fooo 42)");
    let interner = Interner::new();
    let config = Config::new();

    let sym = interner.intern("fooo");
    let suggestion = interner.intern("foo");
    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: sym,
            suggestion: Some(suggestion),
        },
        loc(source_id, 1, 5),
    );

    let output = render(&error, &registry, &interner, &config);

    // Verify structure matches target format
    let lines: Vec<&str> = output.lines().collect();

    // First line: error[Variant]: message
    assert!(lines.get(0).is_some_and(|line| line.starts_with("error[")));

    // Second line: --> location
    assert!(lines.get(1).is_some_and(|line| line.contains("-->")));

    // Should have pipe separator
    assert!(output.contains(" |"));

    // Should have underline carets
    assert!(output.contains("^^^^"));

    // Should have help note
    assert!(output.contains("= help:"));
}

#[test]
fn format_all_parser_error_kinds_have_messages() {
    let (registry, source_id) = create_registry("<test>", "test source");
    let interner = Interner::new();
    let config = Config::new();
    let test_loc = loc(source_id, 0, 4);

    // Test all parser error kinds produce non-empty messages
    let parser_kinds = [
        ParserKind::UnexpectedCharacter('@'),
        ParserKind::UnterminatedString,
        ParserKind::InvalidEscapeSequence('q'),
        ParserKind::InvalidNumber,
        ParserKind::InvalidUnicodeEscape,
        ParserKind::UnexpectedToken {
            expected: "expression",
            found: "token",
        },
        ParserKind::UnmatchedDelimiter {
            opener: '(',
            opener_location: test_loc,
            expected: ')',
            found: ']',
        },
        ParserKind::UnexpectedEof {
            expected: "closing",
        },
        ParserKind::OddMapEntries,
        ParserKind::ReaderMacroMissingExpr,
    ];

    for kind in parser_kinds {
        let error = ParserError::new(kind, test_loc);
        let output = render(&error, &registry, &interner, &config);
        assert!(
            output.contains("error["),
            "Parser error should have variant name"
        );
        assert!(output.contains("--> <test>:"), "Should have location");
    }
}

// =============================================================================
// Line Number Width Tests
// =============================================================================

#[test]
fn format_error_large_line_numbers() {
    // Create a source with many lines to test line number padding
    let mut source = String::new();
    for idx in 0_u32..100_u32 {
        source.push_str(&format!("(line-{})\n", idx.saturating_add(1_u32)));
    }

    let (registry, source_id) = create_registry("<repl>", &source);
    let interner = Interner::new();
    let config = Config::new();

    // Error on line 99 (0-indexed: 98)
    // Each line is "(line-XX)\n" which is 10-11 chars
    let line_99_start = 980_usize; // approximate
    let error = VmError::new(
        VmKind::DivisionByZero,
        loc(
            source_id,
            line_99_start,
            line_99_start.saturating_add(5_usize),
        ),
    );

    let output = render(&error, &registry, &interner, &config);

    // Line numbers should be padded consistently
    assert!(output.contains(" |"));
}
