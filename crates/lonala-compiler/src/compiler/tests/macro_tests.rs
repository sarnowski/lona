// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for macro compilation and expansion.

extern crate alloc;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_bx, decode_op};
use lona_core::symbol;

use super::TEST_SOURCE_ID;
use crate::compiler::{MacroRegistry, compile};
use crate::error::{Error, Kind as ErrorKind};

// =========================================================================
// Special Form: defmacro
// =========================================================================

#[test]
fn defmacro_basic_definition() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro identity [x] x)", TEST_SOURCE_ID).unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_ok());
}

#[test]
fn defmacro_stores_in_registry() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro my-macro [x] x)", TEST_SOURCE_ID).unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    assert!(compiler.is_macro_by_name("my-macro"));

    let macro_def = compiler.get_macro_by_name("my-macro").unwrap();
    assert_eq!(macro_def.first_body().unwrap().arity, 1);
    assert_eq!(macro_def.name(), "my-macro");
}

#[test]
fn defmacro_requires_name() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    // (defmacro [x] x) - first arg is vector, not symbol
    let exprs = lonala_parser::parse("(defmacro [x] x)", TEST_SOURCE_ID).unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_err());

    if let Err(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    }) = result
    {
        assert_eq!(form, "defmacro");
        assert!(message.contains("symbol"));
    } else {
        panic!("expected InvalidSpecialForm error for defmacro with non-symbol name");
    }
}

#[test]
fn defmacro_requires_params_vector() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    // (defmacro foo x x) - params is symbol, not vector
    let exprs = lonala_parser::parse("(defmacro foo x x)", TEST_SOURCE_ID).unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_err());

    if let Err(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    }) = result
    {
        assert_eq!(form, "defmacro");
        // Multi-arity support changed error message
        assert!(message.contains("[params]") || message.contains("vector"));
    } else {
        panic!("expected InvalidSpecialForm error for defmacro with non-vector params");
    }
}

#[test]
fn defmacro_requires_body() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro foo [x])", TEST_SOURCE_ID).unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_err());

    if let Err(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    }) = result
    {
        assert_eq!(form, "defmacro");
        assert!(message.contains("empty"));
    } else {
        panic!("expected InvalidSpecialForm error for defmacro without body");
    }
}

#[test]
fn defmacro_multiple_params() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro swap [a b] b)", TEST_SOURCE_ID).unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    let macro_def = compiler.get_macro_by_name("swap").unwrap();
    assert_eq!(macro_def.first_body().unwrap().arity, 2);
}

#[test]
fn defmacro_zero_params() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse("(defmacro always-nil [] nil)", TEST_SOURCE_ID).unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    let macro_def = compiler.get_macro_by_name("always-nil").unwrap();
    assert_eq!(macro_def.first_body().unwrap().arity, 0);
}

#[test]
fn defmacro_with_quasiquote() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    // unless macro that uses quasiquote
    let exprs = lonala_parser::parse(
        "(defmacro unless [test body] `(if (not ~test) ~body nil))",
        TEST_SOURCE_ID,
    )
    .unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_ok());

    assert!(compiler.is_macro_by_name("unless"));
}

#[test]
fn defmacro_multiple_body_expressions() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    // Macro with multiple body expressions (last is return value)
    let exprs = lonala_parser::parse(
        "(defmacro with-logging [expr] (print \"expanding\") expr)",
        TEST_SOURCE_ID,
    )
    .unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let result = compiler.compile_program(&exprs);
    assert!(result.is_ok());
}

#[test]
fn defmacro_returns_symbol() {
    let mut interner = symbol::Interner::new();
    let chunk = compile("(defmacro foo [x] x)", TEST_SOURCE_ID, &mut interner).unwrap();
    let code = chunk.code();

    // The defmacro expression should return the symbol 'foo.
    // Verify the bytecode loads the symbol constant before returning.
    // (VM execution test is in lona-kernel integration tests)
    let last_loadk = code
        .iter()
        .rev()
        .skip(1) // Skip Return
        .find(|&&instr| decode_op(instr) == Some(Opcode::LoadK))
        .expect("expected LoadK instruction");

    let const_idx = decode_bx(*last_loadk);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) {
        assert_eq!(interner.resolve(*sym_id), "foo");
    } else {
        panic!("expected Symbol constant for defmacro return value");
    }
}

#[test]
fn defmacro_multiple_definitions() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse(
        "(do (defmacro foo [x] x) (defmacro bar [y] y))",
        TEST_SOURCE_ID,
    )
    .unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    assert!(compiler.is_macro_by_name("foo"));
    assert!(compiler.is_macro_by_name("bar"));
}

#[test]
fn defmacro_redefine() {
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let exprs = lonala_parser::parse(
        "(do (defmacro foo [x] x) (defmacro foo [x y] y))",
        TEST_SOURCE_ID,
    )
    .unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let _chunk = compiler.compile_program(&exprs).unwrap();

    let macro_def = compiler.get_macro_by_name("foo").unwrap();
    // Should have the latest definition (2 params)
    assert_eq!(macro_def.first_body().unwrap().arity, 2);
}

// =========================================================================
// Macro Expansion Behavior Tests
// =========================================================================

#[test]
fn macro_call_without_expander_compiles_as_function_call() {
    // Without an expander, macro calls are treated as regular function calls.
    // The macro is stored in the registry but not expanded at compile time.
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();

    // First, define a macro
    let def_exprs = lonala_parser::parse("(defmacro my-macro [x] x)", TEST_SOURCE_ID).unwrap();
    let mut compiler = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let _chunk = compiler.compile_program(&def_exprs).unwrap();

    // Verify macro is stored
    assert!(registry.contains(interner.intern("my-macro")));

    // Now compile code that calls the macro (without expander)
    // This should compile as a regular function call to "my-macro"
    let call_exprs = lonala_parser::parse("(my-macro 42)", TEST_SOURCE_ID).unwrap();
    let mut compiler2 = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let result = compiler2.compile_program(&call_exprs);

    // Should compile successfully (as a function call)
    assert!(result.is_ok());

    // The resulting bytecode should have GetGlobal for "my-macro"
    // (not macro expansion)
    let chunk = result.unwrap();
    let code = chunk.code();

    // Should have GetGlobal instruction (the macro is treated as a function)
    let has_get_global = code
        .iter()
        .any(|&instr| decode_op(instr) == Some(Opcode::GetGlobal));
    assert!(
        has_get_global,
        "macro call without expander should compile as GetGlobal"
    );
}

#[test]
fn macro_registry_persists_across_compilations() {
    // Verify that macros defined in one compilation are available in subsequent
    // compilations with the same registry.
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();

    // First compilation: define macros
    let exprs1 = lonala_parser::parse(
        "(do (defmacro m1 [x] x) (defmacro m2 [x y] y))",
        TEST_SOURCE_ID,
    )
    .unwrap();
    let mut compiler1 = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let _chunk1 = compiler1.compile_program(&exprs1).unwrap();

    // Verify macros are in registry
    assert_eq!(registry.len(), 2);
    assert!(registry.contains(interner.intern("m1")));
    assert!(registry.contains(interner.intern("m2")));

    // Second compilation: registry should still have the macros
    let exprs2 = lonala_parser::parse("(+ 1 2)", TEST_SOURCE_ID).unwrap();
    let mut compiler2 = crate::Compiler::new(&mut interner, &mut registry, TEST_SOURCE_ID);
    let _chunk2 = compiler2.compile_program(&exprs2).unwrap();

    // Macros should persist
    assert_eq!(registry.len(), 2);
    assert!(registry.contains(interner.intern("m1")));
    assert!(registry.contains(interner.intern("m2")));
}

#[test]
fn compile_with_registry_stores_macros() {
    // Test the compile_with_registry public API
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();

    // Compile code that defines a macro
    let _chunk = crate::compile_with_registry(
        "(defmacro test-macro [x] x)",
        TEST_SOURCE_ID,
        &mut interner,
        &mut registry,
    )
    .unwrap();

    // Macro should be in registry
    assert!(registry.contains(interner.intern("test-macro")));
}

// =========================================================================
// Macro Expansion Tests (with Mock Expanders)
// =========================================================================
// These tests verify the macro expansion infrastructure using mock expanders.
// Full integration tests with the VM-based expander are in lona-kernel.

/// Helper struct that implements MacroExpander for testing.
/// Returns the first argument unchanged (identity macro).
struct IdentityExpander;

impl crate::MacroExpander for IdentityExpander {
    fn expand(
        &mut self,
        _definition: &crate::MacroDefinition,
        args: alloc::vec::Vec<lona_core::value::Value>,
        _interner: &mut symbol::Interner,
    ) -> Result<lona_core::value::Value, crate::MacroExpansionError> {
        // Return first argument, or nil if no args
        Ok(args
            .into_iter()
            .next()
            .unwrap_or(lona_core::value::Value::Nil))
    }
}

#[test]
fn macro_expansion_with_mock_expander() {
    // Test that macro expansion works with a mock expander
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let mut expander = IdentityExpander;

    // First define the macro
    let _def_chunk = crate::compile_with_expansion(
        "(defmacro id [x] x)",
        TEST_SOURCE_ID,
        &mut interner,
        &mut registry,
        &mut expander,
    )
    .unwrap();

    // Verify macro is registered
    assert!(registry.contains(interner.intern("id")));

    // Now compile code that uses the macro
    // (id 42) should expand to 42 (via mock expander)
    let chunk = crate::compile_with_expansion(
        "(id 42)",
        TEST_SOURCE_ID,
        &mut interner,
        &mut registry,
        &mut expander,
    )
    .unwrap();

    // The expanded code should just load 42 and return it
    let code = chunk.code();
    assert_eq!(code.len(), 2); // LoadK + Return
    assert_eq!(decode_op(*code.get(0_usize).unwrap()), Some(Opcode::LoadK));
}

/// Helper expander that always returns a fixed value to test expansion tracking.
struct FixedValueExpander {
    expansion_count: core::cell::Cell<usize>,
}

impl crate::MacroExpander for FixedValueExpander {
    fn expand(
        &mut self,
        _definition: &crate::MacroDefinition,
        _args: alloc::vec::Vec<lona_core::value::Value>,
        _interner: &mut symbol::Interner,
    ) -> Result<lona_core::value::Value, crate::MacroExpansionError> {
        let count = self.expansion_count.get();
        self.expansion_count.set(count.saturating_add(1));
        // Always return 42 to stop recursion after first expansion
        Ok(lona_core::value::Value::Integer(
            lona_core::integer::Integer::from_i64(42),
        ))
    }
}

#[test]
fn macro_expansion_tracks_expansion_count() {
    // Test that expansion happens and is tracked
    let mut interner = symbol::Interner::new();
    let mut registry = MacroRegistry::new();
    let mut expander = FixedValueExpander {
        expansion_count: core::cell::Cell::new(0),
    };

    // Define a macro
    let _def_chunk = crate::compile_with_expansion(
        "(defmacro test-macro [x] x)",
        TEST_SOURCE_ID,
        &mut interner,
        &mut registry,
        &mut expander,
    )
    .unwrap();

    // Call the macro - expander returns 42, which terminates expansion
    let chunk = crate::compile_with_expansion(
        "(test-macro 1)",
        TEST_SOURCE_ID,
        &mut interner,
        &mut registry,
        &mut expander,
    )
    .unwrap();

    // Verify expansion happened
    assert!(expander.expansion_count.get() >= 1);

    // Verify result is 42 (from the expander)
    let code = chunk.code();
    let k_idx = decode_bx(*code.get(0_usize).unwrap());
    assert_eq!(chunk.get_constant(k_idx), Some(&Constant::Integer(42)));
}
