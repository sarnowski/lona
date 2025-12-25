// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for case difference and no-suggestion scenarios.

use lona_core::symbol::Interner;
use lona_kernel::vm::{Error as VmError, ErrorKind as VmKind};
use lonala_human::{Config, render};

use super::{create_registry, loc};

// =============================================================================
// Case Differences
// =============================================================================

#[test]
fn suggestion_for_case_difference_all_caps() {
    // PRINT → print
    let (registry, source_id) = create_registry("<repl>", "(PRINT 42)");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("PRINT");
    let correct = interner.intern("print");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 1, 6),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined function 'PRINT'"));
    assert!(output.contains("= help: did you mean 'print'?"));
}

#[test]
fn suggestion_for_case_difference_pascal_case() {
    // Print → print (PascalCase)
    let (registry, source_id) = create_registry("<repl>", "(Print 42)");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("Print");
    let correct = interner.intern("print");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 1, 6),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined function 'Print'"));
    assert!(output.contains("= help: did you mean 'print'?"));
}

#[test]
fn suggestion_for_case_difference_mixed_case() {
    // myVar → my-var (different naming convention)
    let (registry, source_id) = create_registry("<repl>", "myVar");
    let mut interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("myVar");
    let correct = interner.intern("my-var");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 0, 5),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("= help: did you mean 'my-var'?"));
}

// =============================================================================
// No Suggestion Available
// =============================================================================

#[test]
fn no_suggestion_when_none_available() {
    let (registry, source_id) = create_registry("<repl>", "totally-unknown-symbol");
    let mut interner = Interner::new();
    let config = Config::new();

    let unknown = interner.intern("totally-unknown-symbol");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: unknown,
            suggestion: None,
        },
        loc(source_id, 0, 22),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined symbol 'totally-unknown-symbol'"));
    assert!(
        !output.contains("did you mean"),
        "Should not have suggestion when none available"
    );
}

#[test]
fn no_suggestion_for_function_when_none_available() {
    let (registry, source_id) = create_registry("<repl>", "(completely-unknown 42)");
    let mut interner = Interner::new();
    let config = Config::new();

    let unknown = interner.intern("completely-unknown");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: unknown,
            suggestion: None,
        },
        loc(source_id, 1, 19),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined function 'completely-unknown'"));
    assert!(
        !output.contains("did you mean"),
        "Should not have suggestion when none available"
    );
}
