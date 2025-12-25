// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Compiler error integration tests.

use alloc::string::String;

use lona_core::symbol::Interner;
use lonala_compiler::error::{Error as CompilerError, Kind as CompilerKind};
use lonala_human::{Config, render};

use super::{create_registry, loc};

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
