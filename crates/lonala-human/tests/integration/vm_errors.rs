// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! VM error integration tests.

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::source::{Id as SourceId, Registry};
use lona_core::symbol::Interner;
use lona_core::value;
use lona_kernel::vm::{Error as VmError, ErrorKind as VmKind, NativeError};
use lonala_human::{Config, render};

use super::{create_registry, loc};

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
    let interner = Interner::new();
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
    let interner = Interner::new();
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
    let interner = Interner::new();
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

#[test]
fn format_all_vm_error_kinds_have_messages() {
    let (registry, source_id) = create_registry("<test>", "test source");
    let interner = Interner::new();
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
