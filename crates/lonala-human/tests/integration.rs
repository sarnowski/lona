// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Integration tests for end-to-end error formatting.
//!
//! These tests verify that the `lonala-human` crate correctly formats errors
//! from all error sources (parser, compiler, VM) into Rust-style diagnostic
//! messages with source context, underlines, and helpful notes.
//!
//! # Test Scenarios
//!
//! - Parser errors (all kinds)
//! - Compiler errors (all kinds)
//! - VM errors (all kinds)
//! - Multi-line error spans
//! - Unicode in source code
//! - Empty/minimal sources
//! - Context lines (before and after)

#![cfg(feature = "alloc")]

extern crate alloc;

use alloc::string::{String, ToString};

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::source::{Id as SourceId, Location, Registry};
use lona_core::span::Span;
use lona_core::symbol::Interner;
use lona_core::value;
use lona_kernel::vm::{Error as VmError, ErrorKind as VmKind, NativeError};
use lonala_compiler::error::{Error as CompilerError, Kind as CompilerKind};
use lonala_human::{Config, render};
use lonala_parser::error::{Error as ParserError, Kind as ParserKind};

// =============================================================================
// Helper Functions
// =============================================================================

/// Creates a test source registry with a single source.
fn create_registry(name: &str, content: &str) -> (Registry, SourceId) {
    let mut registry = Registry::new();
    let source_id = registry
        .add(name.to_string(), content.to_string())
        .expect("should add source");
    (registry, source_id)
}

/// Creates a location for a span in the given source.
fn loc(source_id: SourceId, start: usize, end: usize) -> Location {
    Location::new(source_id, Span::new(start, end))
}

// =============================================================================
// Parser Error Integration Tests
// =============================================================================

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

// =============================================================================
// Compiler Error Integration Tests
// =============================================================================

#[test]
fn format_compiler_too_many_constants() {
    let (registry, source_id) = create_registry("<repl>", "(def x 42)");
    let interner = Interner::new();
    let config = Config::new();

    let error = CompilerError::new(CompilerKind::TooManyConstants, loc(source_id, 0, 10));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[TooManyConstants]"));
    assert!(output.contains("too many constants"));
    assert!(output.contains("maximum 65535"));
    assert!(output.contains("= help:"));
}

#[test]
fn format_compiler_too_many_registers() {
    let (registry, source_id) = create_registry("<repl>", "(fn [] ...)");
    let interner = Interner::new();
    let config = Config::new();

    let error = CompilerError::new(CompilerKind::TooManyRegisters, loc(source_id, 0, 11));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[TooManyRegisters]"));
    assert!(output.contains("too many registers"));
}

#[test]
fn format_compiler_empty_call() {
    let (registry, source_id) = create_registry("<repl>", "()");
    let interner = Interner::new();
    let config = Config::new();

    let error = CompilerError::new(CompilerKind::EmptyCall, loc(source_id, 0, 2));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[EmptyCall]"));
    assert!(output.contains("empty list cannot be called as function"));
    assert!(output.contains("= note:"));
}

#[test]
fn format_compiler_not_implemented() {
    let (registry, source_id) = create_registry("<repl>", "(closure x)");
    let interner = Interner::new();
    let config = Config::new();

    let error = CompilerError::new(
        CompilerKind::NotImplemented {
            feature: "closures",
        },
        loc(source_id, 0, 11),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[NotImplemented]"));
    assert!(output.contains("not implemented: closures"));
}

#[test]
fn format_compiler_invalid_special_form() {
    let (registry, source_id) = create_registry("<repl>", "(if)");
    let interner = Interner::new();
    let config = Config::new();

    let error = CompilerError::new(
        CompilerKind::InvalidSpecialForm {
            form: "if",
            message: "missing test expression",
        },
        loc(source_id, 0, 4),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[InvalidSpecialForm]"));
    assert!(output.contains("invalid 'if' form"));
    assert!(output.contains("= note:")); // syntax hint
}

#[test]
fn format_compiler_macro_expansion_depth_exceeded() {
    let (registry, source_id) = create_registry("<repl>", "(recursive-macro)");
    let interner = Interner::new();
    let config = Config::new();

    let error = CompilerError::new(
        CompilerKind::MacroExpansionDepthExceeded { depth: 256_usize },
        loc(source_id, 0, 17),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[MacroExpansionDepthExceeded]"));
    assert!(output.contains("256"));
    assert!(output.contains("= note:"));
}

// =============================================================================
// VM Error Integration Tests
// =============================================================================

#[test]
fn format_vm_invalid_opcode() {
    let (registry, source_id) = create_registry("<repl>", "(foo)");
    let interner = Interner::new();
    let config = Config::new();

    let error = VmError::new(
        VmKind::InvalidOpcode {
            byte: 0xFF_u8,
            pc: 42_usize,
        },
        loc(source_id, 0, 5),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[InvalidOpcode]"));
    assert!(output.contains("0xFF"));
    assert!(output.contains("42"));
}

#[test]
fn format_vm_undefined_global() {
    let (registry, source_id) = create_registry("<repl>", "fooo");
    let mut interner = Interner::new();
    let config = Config::new();

    let sym = interner.intern("fooo");
    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: sym,
            suggestion: None,
        },
        loc(source_id, 0, 4),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[UndefinedGlobal]"));
    assert!(output.contains("undefined symbol 'fooo'"));
}

#[test]
fn format_vm_undefined_global_with_suggestion() {
    let (registry, source_id) = create_registry("<repl>", "fooo");
    let mut interner = Interner::new();
    let config = Config::new();

    let sym = interner.intern("fooo");
    let suggestion = interner.intern("foo");
    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: sym,
            suggestion: Some(suggestion),
        },
        loc(source_id, 0, 4),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[UndefinedGlobal]"));
    assert!(output.contains("= help: did you mean 'foo'?"));
}

#[test]
fn format_vm_type_error() {
    let (registry, source_id) = create_registry("<repl>", "(+ \"hello\" 5)");
    let interner = Interner::new();
    let config = Config::new();

    let error = VmError::new(
        VmKind::TypeError {
            operation: "+",
            expected: TypeExpectation::Numeric,
            got: value::Kind::String,
            operand: Some(0_u8),
        },
        loc(source_id, 0, 14),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[TypeError]"));
    assert!(output.contains("type error in '+'"));
    assert!(output.contains("numeric"));
    assert!(output.contains("string"));
    assert!(output.contains("= note:")); // operand note
}

#[test]
fn format_vm_division_by_zero() {
    let (registry, source_id) = create_registry("<repl>", "(/ 10 0)");
    let interner = Interner::new();
    let config = Config::new();

    let error = VmError::new(VmKind::DivisionByZero, loc(source_id, 0, 8));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[DivisionByZero]"));
    assert!(output.contains("division by zero"));
    assert!(output.contains("= note:"));
}

#[test]
fn format_vm_stack_overflow() {
    let (registry, source_id) = create_registry("<repl>", "(recurse)");
    let interner = Interner::new();
    let config = Config::new();

    let error = VmError::new(
        VmKind::StackOverflow {
            max_depth: 256_usize,
        },
        loc(source_id, 0, 9),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[StackOverflow]"));
    assert!(output.contains("256"));
    assert!(output.contains("= note:"));
    assert!(output.contains("= help:"));
}

#[test]
fn format_vm_not_callable() {
    let (registry, source_id) = create_registry("<repl>", "(42)");
    let interner = Interner::new();
    let config = Config::new();

    let error = VmError::new(
        VmKind::NotCallable {
            got: value::Kind::Integer,
        },
        loc(source_id, 0, 4),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[NotCallable]"));
    assert!(output.contains("cannot call value of type integer"));
}

#[test]
fn format_vm_arity_mismatch_exact() {
    let (registry, source_id) = create_registry("<repl>", "(add 1 2 3)");
    let mut interner = Interner::new();
    let config = Config::new();

    let sym = interner.intern("add");
    let error = VmError::new(
        VmKind::ArityMismatch {
            callable: Some(sym),
            expected: ArityExpectation::Exact(2_u8),
            got: 3_u8,
        },
        loc(source_id, 0, 11),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[ArityMismatch]"));
    assert!(output.contains("function 'add'"));
    assert!(output.contains("expected 2, got 3"));
}

#[test]
fn format_vm_arity_mismatch_at_least() {
    let (registry, source_id) = create_registry("<repl>", "(print)");
    let interner = Interner::new();
    let config = Config::new();

    let error = VmError::new(
        VmKind::ArityMismatch {
            callable: None,
            expected: ArityExpectation::AtLeast(1_u8),
            got: 0_u8,
        },
        loc(source_id, 0, 7),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[ArityMismatch]"));
    assert!(output.contains("at least 1"));
}

#[test]
fn format_vm_native_error_arity() {
    let (registry, source_id) = create_registry("<repl>", "(native-fn 1)");
    let interner = Interner::new();
    let config = Config::new();

    let error = VmError::new(
        VmKind::Native {
            error: NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(2_u8),
                got: 1_u8,
            },
        },
        loc(source_id, 0, 13),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[NativeError]"));
    assert!(output.contains("native function"));
}

#[test]
fn format_vm_native_error_type() {
    let (registry, source_id) = create_registry("<repl>", "(native-fn true)");
    let interner = Interner::new();
    let config = Config::new();

    let error = VmError::new(
        VmKind::Native {
            error: NativeError::TypeError {
                expected: TypeExpectation::Numeric,
                got: value::Kind::Bool,
                arg_index: 0_u8,
            },
        },
        loc(source_id, 0, 16),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[NativeError]"));
    assert!(output.contains("type error"));
    assert!(output.contains("first argument"));
}

// =============================================================================
// Multi-Line Error Span Tests
// =============================================================================

#[test]
fn format_error_on_second_line() {
    let source = "(defn add [a b]\n  (+ a b))";
    let (registry, source_id) = create_registry("<repl>", source);
    let interner = Interner::new();
    let config = Config::new();

    // Error on line 2, at the '+'
    let error = VmError::new(VmKind::DivisionByZero, loc(source_id, 18, 19));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("--> <repl>:2:3"));
    assert!(output.contains("(+ a b))"));
}

#[test]
fn format_error_with_context_before() {
    let source = "(defn add [a b]\n  ; Add two numbers\n  (+ a b))";
    let (registry, source_id) = create_registry("<repl>", source);
    let interner = Interner::new();
    let config = Config::new().with_context_before(2_u32);

    // Error on line 3
    let error = VmError::new(VmKind::DivisionByZero, loc(source_id, 38, 39));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("--> <repl>:3:3"));
    // Context should show lines before
    assert!(output.contains("(defn add"));
    assert!(output.contains("; Add two numbers"));
    assert!(output.contains("(+ a b))"));
}

#[test]
fn format_error_with_context_after() {
    let source = "(def x 1)\n(def y 2)\n(def z 3)";
    let (registry, source_id) = create_registry("<repl>", source);
    let interner = Interner::new();
    let config = Config::new().with_context_after(2_u32);

    // Error on line 1
    let error = VmError::new(VmKind::DivisionByZero, loc(source_id, 0, 9));

    let output = render(&error, &registry, &interner, &config);

    // Context should show lines after
    assert!(output.contains("(def x 1)"));
    assert!(output.contains("(def y 2)"));
    assert!(output.contains("(def z 3)"));
}

#[test]
fn format_multiline_span() {
    let source = "(defn foo\n  []\n  bar)";
    let (registry, source_id) = create_registry("<repl>", source);
    let interner = Interner::new();
    let config = Config::new();

    // Span covers multiple lines
    let error = VmError::new(VmKind::DivisionByZero, loc(source_id, 0, 21));

    let output = render(&error, &registry, &interner, &config);

    // Should show the first line at minimum
    assert!(output.contains("(defn foo"));
}

// =============================================================================
// Unicode in Source Code Tests
// =============================================================================

#[test]
fn format_error_with_unicode_source() {
    // Japanese "hello" followed by code
    let source = "; こんにちは\n(foo)";
    let (registry, source_id) = create_registry("<repl>", source);
    let interner = Interner::new();
    let config = Config::new();

    // Error at (foo) on line 2
    // UTF-8: each Japanese character is 3 bytes, so 5 chars = 15 bytes + "; " (2) + newline (1)
    let error = VmError::new(VmKind::DivisionByZero, loc(source_id, 18, 23));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("--> <repl>:2:1"));
    assert!(output.contains("(foo)"));
}

#[test]
fn format_error_in_unicode_string() {
    let source = "(print \"日本語\")";
    let (registry, source_id) = create_registry("<repl>", source);
    let interner = Interner::new();
    let config = Config::new();

    let error = VmError::new(VmKind::DivisionByZero, loc(source_id, 0, 18));

    let output = render(&error, &registry, &interner, &config);

    // Should handle multi-byte characters
    assert!(output.contains("日本語"));
    assert!(output.contains('^'));
}

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
// Unknown Source Tests
// =============================================================================

#[test]
fn format_error_unknown_source() {
    let registry = Registry::new();
    let interner = Interner::new();
    let config = Config::new();

    // Use a source ID that doesn't exist
    let error = VmError::new(VmKind::DivisionByZero, loc(SourceId::new(999_u32), 0, 5));

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("error[DivisionByZero]"));
    assert!(output.contains("<unknown source>"));
}

// =============================================================================
// Output Format Verification Tests
// =============================================================================

#[test]
fn format_output_structure() {
    let (registry, source_id) = create_registry("<repl>", "(fooo 42)");
    let mut interner = Interner::new();
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

#[test]
fn format_all_compiler_error_kinds_have_messages() {
    let (registry, source_id) = create_registry("<test>", "test source");
    let interner = Interner::new();
    let config = Config::new();
    let test_loc = loc(source_id, 0, 4);

    // Test all compiler error kinds produce non-empty messages
    let compiler_kinds: [CompilerKind; 8] = [
        CompilerKind::TooManyConstants,
        CompilerKind::TooManyRegisters,
        CompilerKind::JumpTooLarge,
        CompilerKind::EmptyCall,
        CompilerKind::NotImplemented { feature: "test" },
        CompilerKind::InvalidSpecialForm {
            form: "if",
            message: "test",
        },
        CompilerKind::InvalidMacroResult {
            message: String::from("test"),
        },
        CompilerKind::MacroExpansionDepthExceeded { depth: 256 },
    ];

    for kind in compiler_kinds {
        let error = CompilerError::new(kind, test_loc);
        let output = render(&error, &registry, &interner, &config);
        assert!(
            output.contains("error["),
            "Compiler error should have variant name"
        );
        assert!(output.contains("--> <test>:"), "Should have location");
    }
}

#[test]
fn format_all_vm_error_kinds_have_messages() {
    let (registry, source_id) = create_registry("<test>", "test source");
    let mut interner = Interner::new();
    let config = Config::new();
    let test_loc = loc(source_id, 0, 4);

    let sym = interner.intern("test");

    // Test all VM error kinds produce non-empty messages
    let vm_kinds: [VmKind; 11] = [
        VmKind::InvalidOpcode { byte: 0xFF, pc: 0 },
        VmKind::UndefinedGlobal {
            symbol: sym,
            suggestion: None,
        },
        VmKind::UndefinedFunction {
            symbol: sym,
            suggestion: None,
        },
        VmKind::TypeError {
            operation: "+",
            expected: TypeExpectation::Numeric,
            got: value::Kind::String,
            operand: None,
        },
        VmKind::DivisionByZero,
        VmKind::StackOverflow { max_depth: 256 },
        VmKind::NotCallable {
            got: value::Kind::Integer,
        },
        VmKind::InvalidConstant { index: 0 },
        VmKind::InvalidRegister { index: 0 },
        VmKind::Native {
            error: NativeError::Error("test"),
        },
        VmKind::ArityMismatch {
            callable: None,
            expected: ArityExpectation::Exact(2),
            got: 1,
        },
    ];

    for kind in vm_kinds {
        let error = VmError::new(kind, test_loc);
        let output = render(&error, &registry, &interner, &config);
        assert!(
            output.contains("error["),
            "VM error should have variant name"
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
        source.push_str(&alloc::format!("(line-{})\n", idx.saturating_add(1_u32)));
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
