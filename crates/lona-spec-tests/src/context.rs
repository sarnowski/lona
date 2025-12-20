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
use lona_core::source;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_kernel::vm::{Globals, MacroExpander, Vm};
use lonala_compiler::MacroRegistry;

/// Default source ID for spec tests.
const TEST_SOURCE_ID: source::Id = source::Id::new(0_u32);

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
            TEST_SOURCE_ID,
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
        let arithmetic_symbols = lona_kernel::vm::intern_arithmetic_primitives(&mut self.interner);
        let comparison_symbols = lona_kernel::vm::intern_comparison_primitives(&mut self.interner);
        let type_predicate_symbols = lona_kernel::vm::intern_type_predicates(&mut self.interner);

        // Create VM and restore persistent globals
        let mut vm = Vm::new(&self.interner);
        *vm.globals_mut() = self.globals.clone();

        // Register collection primitives
        lona_kernel::vm::collections::register_primitives(&mut vm, &collection_symbols);

        // Register arithmetic primitives (first-class + and -)
        lona_kernel::vm::register_arithmetic_primitives(&mut vm, &arithmetic_symbols);

        // Register comparison primitives (first-class =, <, >, <=, >=)
        lona_kernel::vm::register_comparison_primitives(&mut vm, &comparison_symbols);

        // Register type predicate primitives (keyword?, etc.)
        lona_kernel::vm::register_type_predicates(&mut vm, &type_predicate_symbols);

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

    /// Asserts that an expression evaluates to a symbol (type check only).
    ///
    /// Prefer `assert_symbol_eq` when you know the expected symbol name.
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

    /// Asserts that an expression evaluates to a symbol with the expected name.
    pub fn assert_symbol_eq(&mut self, source: &str, expected_name: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Symbol(sym_id)) => {
                let actual_name = self.interner.resolve(sym_id);
                assert_eq!(
                    actual_name, expected_name,
                    "{spec_ref}: symbol name mismatch"
                );
            }
            Ok(other) => {
                panic!("{spec_ref}: expected symbol, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a list (type check only).
    ///
    /// Prefer `assert_list_eq` when you know the expected list contents.
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

    /// Asserts that an expression evaluates to a list with the expected contents.
    ///
    /// The `expected_source` is evaluated to produce the expected list value.
    pub fn assert_list_eq(&mut self, source: &str, expected_source: &str, spec_ref: &str) {
        let expected = match self.eval(expected_source) {
            Ok(Value::List(list)) => list,
            Ok(other) => {
                panic!("{spec_ref}: expected_source must produce a list, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: expected_source evaluation failed: {err}");
            }
        };

        match self.eval(source) {
            Ok(Value::List(actual)) => {
                assert_eq!(
                    actual, expected,
                    "{spec_ref}: list contents mismatch.\nActual: {actual:?}\nExpected: {expected:?}"
                );
            }
            Ok(other) => {
                panic!("{spec_ref}: expected list, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a list with the expected length.
    pub fn assert_list_len(&mut self, source: &str, expected_len: usize, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::List(list)) => {
                let actual_len = list.len();
                assert_eq!(actual_len, expected_len, "{spec_ref}: list length mismatch");
            }
            Ok(other) => {
                panic!("{spec_ref}: expected list, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a vector (type check only).
    ///
    /// Prefer `assert_vector_eq` when you know the expected vector contents.
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

    /// Asserts that an expression evaluates to a vector with the expected contents.
    ///
    /// The `expected_source` is evaluated to produce the expected vector value.
    pub fn assert_vector_eq(&mut self, source: &str, expected_source: &str, spec_ref: &str) {
        let expected = match self.eval(expected_source) {
            Ok(Value::Vector(vec)) => vec,
            Ok(other) => {
                panic!("{spec_ref}: expected_source must produce a vector, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: expected_source evaluation failed: {err}");
            }
        };

        match self.eval(source) {
            Ok(Value::Vector(actual)) => {
                assert_eq!(
                    actual, expected,
                    "{spec_ref}: vector contents mismatch.\nActual: {actual:?}\nExpected: {expected:?}"
                );
            }
            Ok(other) => {
                panic!("{spec_ref}: expected vector, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a vector with the expected length.
    pub fn assert_vector_len(&mut self, source: &str, expected_len: usize, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Vector(vec)) => {
                let actual_len = vec.len();
                assert_eq!(
                    actual_len, expected_len,
                    "{spec_ref}: vector length mismatch"
                );
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

    /// Asserts that an expression evaluates to a map (type check only).
    ///
    /// Prefer `assert_map_eq` when you know the expected map contents.
    pub fn assert_map(&mut self, source: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Map(_map)) => {
                // Expected - got a map
            }
            Ok(other) => {
                panic!("{spec_ref}: expected map, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a map with the expected contents.
    ///
    /// The `expected_source` is evaluated to produce the expected map value.
    pub fn assert_map_eq(&mut self, source: &str, expected_source: &str, spec_ref: &str) {
        let expected = match self.eval(expected_source) {
            Ok(Value::Map(map)) => map,
            Ok(other) => {
                panic!("{spec_ref}: expected_source must produce a map, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: expected_source evaluation failed: {err}");
            }
        };

        match self.eval(source) {
            Ok(Value::Map(actual)) => {
                assert_eq!(
                    actual, expected,
                    "{spec_ref}: map contents mismatch.\nActual: {actual:?}\nExpected: {expected:?}"
                );
            }
            Ok(other) => {
                panic!("{spec_ref}: expected map, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a map with the expected number of entries.
    pub fn assert_map_len(&mut self, source: &str, expected_len: usize, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Map(map)) => {
                let actual_len = map.len();
                assert_eq!(actual_len, expected_len, "{spec_ref}: map length mismatch");
            }
            Ok(other) => {
                panic!("{spec_ref}: expected map, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a set (type check only).
    ///
    /// Prefer `assert_set_eq` when you know the expected set contents.
    pub fn assert_set(&mut self, source: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Set(_set)) => {
                // Expected - got a set
            }
            Ok(other) => {
                panic!("{spec_ref}: expected set, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a set with the expected number of elements.
    pub fn assert_set_len(&mut self, source: &str, expected_len: usize, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Set(set)) => {
                let actual_len = set.len();
                assert_eq!(actual_len, expected_len, "{spec_ref}: set length mismatch");
            }
            Ok(other) => {
                panic!("{spec_ref}: expected set, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a keyword (type check only).
    pub fn assert_keyword(&mut self, source: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Keyword(_kw)) => {
                // Expected - got a keyword
            }
            Ok(other) => {
                panic!("{spec_ref}: expected keyword, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a keyword with the expected name.
    pub fn assert_keyword_eq(&mut self, source: &str, expected_name: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(Value::Keyword(kw_id)) => {
                let actual_name = self.interner.resolve(kw_id);
                assert_eq!(
                    actual_name, expected_name,
                    "{spec_ref}: keyword name mismatch"
                );
            }
            Ok(other) => {
                panic!("{spec_ref}: expected keyword, got {other:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }

    /// Asserts that an expression evaluates to a binary buffer.
    /// Note: Binary type not yet implemented - placeholder for future use.
    pub fn assert_binary(&mut self, source: &str, spec_ref: &str) {
        match self.eval(source) {
            Ok(value) => {
                // Binary type not yet in Value enum - check when implemented
                // For now, check if it's something that could be binary
                panic!("{spec_ref}: Binary type not yet implemented, got {value:?}");
            }
            Err(err) => {
                panic!("{spec_ref}: evaluation failed: {err}");
            }
        }
    }
}
