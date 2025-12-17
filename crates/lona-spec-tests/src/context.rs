// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Test context for specification tests.
//!
//! Provides `SpecTestContext` for evaluating Lonala expressions with
//! persistent state across multi-step tests. This simulates REPL behavior
//! where globals and macros persist between evaluations.

use alloc::string::String;

use lona_core::integer::Integer;
use lona_core::ratio::Ratio;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_kernel::vm::{Globals, MacroExpander, Vm};
use lonala_compiler::MacroRegistry;

/// Test context maintaining state across multi-step evaluations.
///
/// This context provides a REPL-like environment where:
/// - Global bindings persist between evaluations
/// - Macro definitions persist between evaluations
/// - Collection primitives are pre-registered
pub struct SpecTestContext {
    interner: Interner,
    globals: Globals,
    macros: MacroRegistry,
}

impl SpecTestContext {
    /// Creates a new test context with collection primitives registered.
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            globals: Globals::new(),
            macros: MacroRegistry::new(),
        }
    }

    /// Evaluates a Lonala expression with full macro expansion support.
    pub fn eval(&mut self, source: &str) -> Result<Value, String> {
        // Create macro expander
        let mut expander = MacroExpander::new();

        // Compile with macro expansion
        let chunk = lonala_compiler::compile_with_expansion(
            source,
            &mut self.interner,
            &mut self.macros,
            &mut expander,
        )
        .map_err(|err| alloc::format!("compile error: {err:?}"))?;

        // Pre-intern primitive symbols before creating VM
        let collection_symbols =
            lona_kernel::vm::collections::intern_primitives(&mut self.interner);
        let introspection_symbols =
            lona_kernel::vm::introspection::intern_primitives(&mut self.interner);

        // Create VM and restore persistent globals
        let mut vm = Vm::new(&self.interner);
        *vm.globals_mut() = self.globals.clone();

        // Register print function
        if let Some(print_sym) = self.interner.get("print") {
            vm.update_print_symbol(print_sym);
            vm.set_global(print_sym, Value::Symbol(print_sym));
        }

        // Register collection primitives
        lona_kernel::vm::collections::register_primitives(&mut vm, &collection_symbols);

        // Set up macro introspection functions
        vm.set_macro_registry(&self.macros);
        lona_kernel::vm::introspection::register_primitives(&mut vm, &introspection_symbols);

        // Execute
        let result = vm
            .execute(&chunk)
            .map_err(|err| alloc::format!("runtime error: {err:?}"))?;

        // Save globals back
        self.globals = vm.globals().clone();

        Ok(result)
    }

    /// Asserts that an expression evaluates to the expected value.
    pub fn assert_eval(&mut self, source: &str, expected: &Value, spec_ref: &str) {
        match self.eval(source) {
            Ok(result) => {
                assert_eq!(result, *expected, "{spec_ref}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to an integer.
    pub fn assert_int(&mut self, source: &str, expected: i64, spec_ref: &str) {
        self.assert_eval(
            source,
            &Value::Integer(Integer::from_i64(expected)),
            spec_ref,
        );
    }

    /// Asserts that an expression evaluates to a boolean.
    pub fn assert_bool(&mut self, source: &str, expected: bool, spec_ref: &str) {
        self.assert_eval(source, &Value::Bool(expected), spec_ref);
    }

    /// Asserts that an expression evaluates to nil.
    pub fn assert_nil(&mut self, source: &str, spec_ref: &str) {
        self.assert_eval(source, &Value::Nil, spec_ref);
    }

    /// Asserts that an expression evaluates to a float.
    pub fn assert_float(&mut self, source: &str, expected: f64, spec_ref: &str) {
        self.assert_eval(source, &Value::Float(expected), spec_ref);
    }

    /// Asserts that an expression evaluates to a ratio.
    pub fn assert_ratio(&mut self, source: &str, numer: i64, denom: i64, spec_ref: &str) {
        let numer_int = Integer::from_i64(numer);
        let denom_int = Integer::from_i64(denom);
        let expected = Ratio::new(&numer_int, &denom_int);
        self.assert_eval(source, &Value::Ratio(expected), spec_ref);
    }

    /// Asserts that an expression evaluates to a string.
    pub fn assert_string(&mut self, source: &str, expected: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::String(result)) => {
                assert_eq!(result.as_str(), expected, "{spec_ref}");
            }
            Ok(other) => {
                panic!("{spec_ref}: expected string, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression produces an error.
    pub fn assert_error(&mut self, source: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(result) => {
                panic!("{spec_ref}: expected error, got {result:?}");
            }
            Err(_err) => {
                // Expected - error occurred
            }
        }
    }

    /// Asserts that an expression produces an error containing a substring.
    pub fn assert_error_contains(&mut self, source: &str, contains: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(result) => {
                panic!("{spec_ref}: expected error, got {result:?}");
            }
            Err(err) => {
                assert!(
                    err.contains(contains),
                    "{spec_ref}: error '{err}' should contain '{contains}'"
                );
            }
        }
    }

    /// Asserts that an expression evaluates to a symbol.
    pub fn assert_symbol(&mut self, source: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Symbol(_sym)) => {
                // Expected - got a symbol
            }
            Ok(other) => {
                panic!("{spec_ref}: expected symbol, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a list.
    pub fn assert_list(&mut self, source: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::List(_list)) => {
                // Expected - got a list
            }
            Ok(other) => {
                panic!("{spec_ref}: expected list, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a vector.
    pub fn assert_vector(&mut self, source: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Vector(_vec)) => {
                // Expected - got a vector
            }
            Ok(other) => {
                panic!("{spec_ref}: expected vector, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a function.
    pub fn assert_function(&mut self, source: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Function(_func)) => {
                // Expected - got a function
            }
            Ok(other) => {
                panic!("{spec_ref}: expected function, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }
}
