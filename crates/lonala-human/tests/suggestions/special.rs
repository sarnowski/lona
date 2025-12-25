// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for special characters, unicode, edge cases, and format consistency.

use lona_core::symbol::Interner;
use lona_kernel::vm::{Error as VmError, ErrorKind as VmKind};
use lonala_human::{Config, render};

use super::{create_registry, loc};

// =============================================================================
// Special Characters
// =============================================================================

#[test]
fn suggestion_with_special_characters_hyphen() {
    // my-functon → my-function (typo in hyphenated name)
    let (registry, source_id) = create_registry("<repl>", "(my-functon 42)");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("my-functon");
    let correct = interner.intern("my-function");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 1, 11),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined function 'my-functon'"));
    assert!(output.contains("= help: did you mean 'my-function'?"));
}

#[test]
fn suggestion_with_special_characters_question_mark() {
    // emty? → empty? (predicate with question mark)
    let (registry, source_id) = create_registry("<repl>", "(emty? [])");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("emty?");
    let correct = interner.intern("empty?");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 1, 6),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined function 'emty?'"));
    assert!(output.contains("= help: did you mean 'empty?'?"));
}

#[test]
fn suggestion_with_special_characters_asterisk() {
    // *foo → *bar (with sigil)
    let (registry, source_id) = create_registry("<repl>", "*foo");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("*foo");
    let correct = interner.intern("*bar");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 0, 4),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined symbol '*foo'"));
    assert!(output.contains("= help: did you mean '*bar'?"));
}

// =============================================================================
// Multiple Errors
// =============================================================================

#[test]
fn multiple_errors_with_different_suggestions() {
    let (registry, source_id) = create_registry("<repl>", "(fooo (barr 42))");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo1 = interner.intern("fooo");
    let correct1 = interner.intern("foo");
    let typo2 = interner.intern("barr");
    let correct2 = interner.intern("bar");

    // First error
    let error1 = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo1,
            suggestion: Some(correct1),
        },
        loc(source_id, 1, 5),
    );

    // Second error
    let error2 = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo2,
            suggestion: Some(correct2),
        },
        loc(source_id, 7, 11),
    );

    let output1 = render(&error1, &registry, &interner, &config);
    let output2 = render(&error2, &registry, &interner, &config);

    assert!(output1.contains("= help: did you mean 'foo'?"));
    assert!(output2.contains("= help: did you mean 'bar'?"));
}

// =============================================================================
// Long Names
// =============================================================================

#[test]
fn suggestion_for_long_function_name() {
    let (registry, source_id) =
        create_registry("<repl>", "(calculate-the-total-sum-of-values 1 2 3)");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("calculate-the-total-sum-of-values");
    let correct = interner.intern("calculate-the-total-sum-of-items");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 1, 34),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined function 'calculate-the-total-sum-of-values'"));
    assert!(output.contains("= help: did you mean 'calculate-the-total-sum-of-items'?"));
}

#[test]
fn suggestion_for_short_name() {
    // x → y (single character)
    let (registry, source_id) = create_registry("<repl>", "x");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("x");
    let correct = interner.intern("y");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 0, 1),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined symbol 'x'"));
    assert!(output.contains("= help: did you mean 'y'?"));
}

// =============================================================================
// Unicode
// =============================================================================

#[test]
fn suggestion_with_unicode_symbols() {
    // Japanese hiragana example
    let (registry, source_id) = create_registry("<repl>", "こんにちわ");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("こんにちわ");
    let correct = interner.intern("こんにちは");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 0, 15), // 5 characters * 3 bytes each
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined symbol 'こんにちわ'"));
    assert!(output.contains("= help: did you mean 'こんにちは'?"));
}

#[test]
fn suggestion_with_emoji() {
    // Symbol containing emoji
    let (registry, source_id) = create_registry("<repl>", "rocket-launch🚀");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("rocket-launch🚀");
    let correct = interner.intern("rocket-launch");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 0, 17), // "rocket-launch" + emoji bytes
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("rocket-launch🚀"));
    assert!(output.contains("= help: did you mean 'rocket-launch'?"));
}

// =============================================================================
// Numeric Suffixes
// =============================================================================

#[test]
fn suggestion_for_numeric_suffix_typo() {
    // var1 → var2
    let (registry, source_id) = create_registry("<repl>", "var1");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("var1");
    let correct = interner.intern("var2");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 0, 4),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined symbol 'var1'"));
    assert!(output.contains("= help: did you mean 'var2'?"));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn suggestion_same_as_typo_displays_correctly() {
    // Edge case: suggestion is the same as the typo
    // This shouldn't happen in practice, but formatting should still work
    let (registry, source_id) = create_registry("<repl>", "foo");
    let mut interner = Interner::new();
    let config = Config::new();

    let sym = interner.intern("foo");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: sym,
            suggestion: Some(sym), // Same symbol
        },
        loc(source_id, 0, 3),
    );

    let output = render(&error, &registry, &interner, &config);

    // Should still format correctly even if suggestion equals the typo
    assert!(output.contains("undefined symbol 'foo'"));
    assert!(output.contains("= help: did you mean 'foo'?"));
}

#[test]
fn suggestion_with_empty_string_symbol() {
    // Edge case: empty string symbol (shouldn't happen in practice)
    let (registry, source_id) = create_registry("<repl>", "");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("");
    let correct = interner.intern("something");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 0, 0),
    );

    let output = render(&error, &registry, &interner, &config);

    // Should handle empty symbols gracefully
    assert!(output.contains("undefined symbol ''"));
    assert!(output.contains("= help: did you mean 'something'?"));
}

#[test]
fn error_variant_name_displayed_correctly() {
    let (registry, source_id) = create_registry("<repl>", "typo");
    let mut interner = Interner::new();
    let config = Config::new();

    let sym = interner.intern("typo");
    let suggestion = interner.intern("type");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: sym,
            suggestion: Some(suggestion),
        },
        loc(source_id, 0, 4),
    );

    let output = render(&error, &registry, &interner, &config);

    // Should have the correct variant name in brackets
    assert!(output.contains("error[UndefinedGlobal]"));
}

#[test]
fn suggestion_location_displayed_correctly() {
    let (registry, source_id) = create_registry("<test-file>", "(defn test [] (fooo))");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("fooo");
    let correct = interner.intern("foo");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 15, 19),
    );

    let output = render(&error, &registry, &interner, &config);

    // Should show correct location
    assert!(output.contains("--> <test-file>:1:16"));
    assert!(output.contains("= help: did you mean 'foo'?"));
}

// =============================================================================
// Format Consistency Tests
// =============================================================================

#[test]
fn suggestion_output_format_matches_spec() {
    // Verify the exact format matches the PLAN.md specification
    let (registry, source_id) = create_registry("<repl>", "(fooo 42)");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("fooo");
    let correct = interner.intern("foo");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 1, 5),
    );

    let output = render(&error, &registry, &interner, &config);

    // Verify structure matches:
    // error[UndefinedFunction]: undefined function 'fooo'
    //   --> <repl>:1:2
    //    |
    //  1 | (fooo 42)
    //    |  ^^^^
    //    |
    //    = help: did you mean 'foo'?

    let lines: Vec<&str> = output.lines().collect();

    // Line 0: error header
    assert!(
        lines
            .get(0)
            .is_some_and(|line| line.starts_with("error[UndefinedFunction]:")),
        "First line should be error header"
    );

    // Line 1: location
    assert!(
        lines.get(1).is_some_and(|line| line.contains("-->")),
        "Second line should be location"
    );

    // Should have pipe separators
    assert!(output.contains(" |"), "Should have pipe separators");

    // Should have carets for underline
    assert!(output.contains("^^^^"), "Should have underline carets");

    // Should have help note
    assert!(
        output.contains("= help: did you mean 'foo'?"),
        "Should have help note with suggestion"
    );
}
