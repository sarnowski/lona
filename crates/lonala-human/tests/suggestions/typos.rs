// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for typo suggestions: extra/missing/swapped/wrong characters.

use lona_core::symbol::Interner;
use lona_kernel::vm::{Error as VmError, ErrorKind as VmKind};
use lonala_human::{Config, render};

use super::{create_registry, loc};

#[test]
fn suggestion_for_typo_extra_character() {
    // fooo → foo (extra 'o')
    let (registry, source_id) = create_registry("<repl>", "fooo");
    let interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("fooo");
    let correct = interner.intern("foo");

    let error = VmError::new(
        VmKind::UndefinedGlobal {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 0, 4),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(
        output.contains("undefined symbol 'fooo'"),
        "Should show the typo'd name"
    );
    assert!(
        output.contains("= help: did you mean 'foo'?"),
        "Should suggest the correct name"
    );
}

#[test]
fn suggestion_for_typo_missing_character() {
    // prin → print (missing 't')
    let (registry, source_id) = create_registry("<repl>", "(prin 42)");
    let interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("prin");
    let correct = interner.intern("print");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 1, 5),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(
        output.contains("undefined function 'prin'"),
        "Should show the typo'd name"
    );
    assert!(
        output.contains("= help: did you mean 'print'?"),
        "Should suggest the correct name"
    );
}

#[test]
fn suggestion_for_typo_swapped_characters() {
    // pirnt → print (swapped 'r' and 'i')
    let (registry, source_id) = create_registry("<repl>", "(pirnt \"hello\")");
    let interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("pirnt");
    let correct = interner.intern("print");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 1, 6),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("undefined function 'pirnt'"));
    assert!(output.contains("= help: did you mean 'print'?"));
}

#[test]
fn suggestion_for_typo_wrong_character() {
    // priny → print (wrong final character)
    let (registry, source_id) = create_registry("<repl>", "(priny 42)");
    let interner = Interner::new();
    let config = Config::new();

    let typo = interner.intern("priny");
    let correct = interner.intern("print");

    let error = VmError::new(
        VmKind::UndefinedFunction {
            symbol: typo,
            suggestion: Some(correct),
        },
        loc(source_id, 1, 6),
    );

    let output = render(&error, &registry, &interner, &config);

    assert!(output.contains("= help: did you mean 'print'?"));
}
