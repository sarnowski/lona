// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Macro introspection functions.
//!
//! This module provides native functions for macro introspection:
//! - `native_is_macro` - Check if a symbol names a macro
//! - `native_expand_once` - Expand a macro call one level
//! - `native_expand_fully` - Fully expand a macro call
//!
//! These are registered as normal native functions, using the `NativeContext`
//! to access the macro registry.

use alloc::vec::Vec;

use lona_core::symbol::{self, Interner};
use lona_core::value::Value;
use lonala_compiler::MacroRegistry;

use super::Vm;
use super::collections::lookup_primitives;
use super::natives::{NativeContext, NativeError};

/// Maximum depth for recursive macro expansion in `macroexpand`.
const MAX_EXPANSION_DEPTH: usize = 256;

/// Native implementation of `macro?`.
///
/// Checks if a symbol names a macro in the registry.
///
/// # Arguments
///
/// * `args` - Should contain exactly one argument: a symbol
/// * `ctx` - Native context providing access to macro registry
///
/// # Returns
///
/// `Value::Bool(true)` if the symbol is a registered macro, `false` otherwise.
/// Returns `false` if no macro registry is available.
///
/// # Errors
///
/// Returns an error if the argument count is wrong or the argument is not a symbol.
#[inline]
pub fn native_is_macro(args: &[Value], ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: 1,
            got: args.len(),
        });
    }

    let arg = args.first().ok_or(NativeError::ArityMismatch {
        expected: 1,
        got: 0,
    })?;

    let Value::Symbol(sym_id) = *arg else {
        return Err(NativeError::TypeError {
            expected: "symbol",
            got: value_type_name(arg),
            arg_index: 0,
        });
    };

    // If no macro registry, no macros exist
    let Some(registry) = ctx.macros() else {
        return Ok(Value::Bool(false));
    };

    Ok(Value::Bool(registry.contains(sym_id)))
}

/// Native implementation of `macroexpand-1`.
///
/// Expands a macro call one level. If the form is a list whose first element
/// is a symbol that names a macro, expands the macro once and returns the result.
/// Otherwise returns the form unchanged.
///
/// # Arguments
///
/// * `args` - Should contain exactly one argument: the form to expand
/// * `ctx` - Native context providing access to macro registry and interner
///
/// # Returns
///
/// The expanded form, or the original form if not a macro call.
///
/// # Errors
///
/// Returns an error if the argument count is wrong or expansion fails.
#[inline]
pub fn native_expand_once(args: &[Value], ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: 1,
            got: args.len(),
        });
    }

    let form = args.first().ok_or(NativeError::ArityMismatch {
        expected: 1,
        got: 0,
    })?;

    // If no macro registry, return form unchanged
    let Some(registry) = ctx.macros() else {
        return Ok(form.clone());
    };

    expand_once_internal(form, registry, ctx.interner())
}

/// Internal implementation of macro expansion (one level).
fn expand_once_internal(
    form: &Value,
    registry: &MacroRegistry,
    interner: &Interner,
) -> Result<Value, NativeError> {
    // Must be a list to be a macro call
    let Value::List(ref list) = *form else {
        return Ok(form.clone());
    };

    // Empty list cannot be a macro call
    if list.is_empty() {
        return Ok(form.clone());
    }

    // First element must be a symbol
    let Some(&Value::Symbol(sym_id)) = list.first() else {
        return Ok(form.clone());
    };

    // Look up macro
    let Some(macro_def) = registry.get(sym_id) else {
        return Ok(form.clone()); // Not a macro
    };

    // Extract arguments (skip the macro name)
    let macro_args: Vec<Value> = list.iter().skip(1).cloned().collect();

    // Verify arity
    let expected_arity = usize::from(macro_def.arity());
    if macro_args.len() != expected_arity {
        return Err(NativeError::Error("macro arity mismatch"));
    }

    // Look up collection primitives (must be pre-interned by caller)
    let collection_symbols = lookup_primitives(interner)
        .ok_or(NativeError::Error("collection primitives not interned"))?;

    // Create a fresh VM for macro expansion
    let mut vm = Vm::new(interner);
    register_primitives(&mut vm, &collection_symbols);

    // Execute the macro's chunk with arguments
    let result = vm
        .execute_with_args(macro_def.chunk(), &macro_args)
        .map_err(|_vm_err| NativeError::Error("macro expansion failed"))?;

    Ok(result)
}

/// Native implementation of `macroexpand`.
///
/// Fully expands a macro call by repeatedly expanding until stable.
/// Keeps expanding the form until it no longer changes (i.e., the outer form
/// is no longer a macro call).
///
/// # Arguments
///
/// * `args` - Should contain exactly one argument: the form to expand
/// * `ctx` - Native context providing access to macro registry and interner
///
/// # Returns
///
/// The fully expanded form.
///
/// # Errors
///
/// Returns an error if the argument count is wrong, expansion fails, or
/// the expansion depth limit is exceeded.
#[inline]
pub fn native_expand_fully(args: &[Value], ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: 1,
            got: args.len(),
        });
    }

    let mut form = args
        .first()
        .ok_or(NativeError::ArityMismatch {
            expected: 1,
            got: 0,
        })?
        .clone();

    // If no macro registry, return form unchanged
    let Some(registry) = ctx.macros() else {
        return Ok(form);
    };

    // Keep expanding until the form doesn't change
    for _ in 0_usize..MAX_EXPANSION_DEPTH {
        let expanded = expand_once_internal(&form, registry, ctx.interner())?;

        if values_equal(&expanded, &form) {
            return Ok(form);
        }

        form = expanded;
    }

    Err(NativeError::Error("macro expansion depth exceeded"))
}

/// Returns a type name for error messages.
const fn value_type_name(value: &Value) -> &'static str {
    match *value {
        Value::Nil => "nil",
        Value::Bool(_) => "boolean",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::Ratio(_) => "ratio",
        Value::Symbol(_) => "symbol",
        Value::String(_) => "string",
        Value::List(_) => "list",
        Value::Vector(_) => "vector",
        Value::Map(_) => "map",
        Value::Function(_) => "function",
        _ => "unknown",
    }
}

/// Checks if two values are equal.
fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (&Value::Nil, &Value::Nil) => true,
        (&Value::Bool(lhs), &Value::Bool(rhs)) => lhs == rhs,
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => lhs == rhs,
        (&Value::Float(lhs), &Value::Float(rhs)) => floats_equal(lhs, rhs),
        (&Value::Ratio(ref lhs), &Value::Ratio(ref rhs)) => lhs == rhs,
        (&Value::Symbol(lhs), &Value::Symbol(rhs)) => lhs == rhs,
        (&Value::String(ref lhs), &Value::String(ref rhs)) => lhs.as_str() == rhs.as_str(),
        (&Value::List(ref lhs), &Value::List(ref rhs)) => {
            lhs.len() == rhs.len()
                && lhs
                    .iter()
                    .zip(rhs.iter())
                    .all(|(left_val, right_val)| values_equal(left_val, right_val))
        }
        (&Value::Vector(ref lhs), &Value::Vector(ref rhs)) => {
            lhs.len() == rhs.len()
                && lhs
                    .iter()
                    .zip(rhs.iter())
                    .all(|(left_val, right_val)| values_equal(left_val, right_val))
        }
        _ => false,
    }
}

/// Compares two floats for equality.
///
/// Since this is used for macro expansion comparison (checking if form changed),
/// we use bitwise equality to handle NaN and exact comparisons correctly.
#[inline]
const fn floats_equal(lhs: f64, rhs: f64) -> bool {
    lhs.to_bits() == rhs.to_bits()
}

/// The names of all introspection primitives.
pub const PRIMITIVE_NAMES: &[&str] = &["macro?", "macroexpand-1", "macroexpand"];

/// Pre-interns all introspection primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `PRIMITIVE_NAMES`.
#[inline]
pub fn intern_primitives(interner: &mut symbol::Interner) -> alloc::vec::Vec<symbol::Id> {
    PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Registers all introspection primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_primitives` with the same interner.
#[inline]
pub fn register_primitives(vm: &mut Vm<'_>, symbols: &[symbol::Id]) {
    let funcs: &[super::natives::NativeFn] =
        &[native_is_macro, native_expand_once, native_expand_fully];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        vm.set_global(*sym, Value::Symbol(*sym));
    }
}

#[cfg(test)]
mod tests {
    use super::super::collections::intern_primitives;
    use super::*;
    use alloc::string::String;
    use alloc::sync::Arc;
    use lona_core::chunk::Chunk;
    use lona_core::integer::Integer;
    use lona_core::list::List;
    use lona_core::opcode::{Opcode, encode_abc};
    use lona_core::span::Span;
    use lonala_compiler::MacroDefinition;

    /// Creates a simple identity macro chunk (returns first argument).
    fn make_identity_chunk() -> Chunk {
        let mut chunk = Chunk::with_name(String::from("test-macro"));
        chunk.set_arity(1_u8);
        // Return R[0] - the first argument
        chunk.emit(
            encode_abc(Opcode::Return, 0_u8, 1_u8, 0_u8),
            Span::new(0_usize, 1_usize),
        );
        chunk.set_max_registers(1_u8);
        chunk
    }

    /// Helper to create a native context with a macro registry.
    fn ctx_with_macros<'a>(
        interner: &'a Interner,
        registry: &'a MacroRegistry,
    ) -> NativeContext<'a> {
        NativeContext::new(interner, Some(registry))
    }

    /// Helper to create a native context without a macro registry.
    fn ctx_without_macros(interner: &Interner) -> NativeContext<'_> {
        NativeContext::new(interner, None)
    }

    #[test]
    fn is_macro_returns_true_for_registered_macro() {
        let mut interner = Interner::new();
        let mut registry = MacroRegistry::new();

        let macro_name = interner.intern("my-macro");
        let chunk = Arc::new(make_identity_chunk());
        let def = MacroDefinition::new(chunk, 1_u8, String::from("my-macro"));
        registry.register(macro_name, def);

        let ctx = ctx_with_macros(&interner, &registry);
        let args = [Value::Symbol(macro_name)];
        let result = native_is_macro(&args, &ctx).unwrap();

        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn is_macro_returns_false_for_unknown_symbol() {
        let mut interner = Interner::new();
        let registry = MacroRegistry::new();

        let unknown_sym = interner.intern("unknown");
        let ctx = ctx_with_macros(&interner, &registry);
        let args = [Value::Symbol(unknown_sym)];
        let result = native_is_macro(&args, &ctx).unwrap();

        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn is_macro_returns_false_without_registry() {
        let mut interner = Interner::new();

        let sym = interner.intern("anything");
        let ctx = ctx_without_macros(&interner);
        let args = [Value::Symbol(sym)];
        let result = native_is_macro(&args, &ctx).unwrap();

        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn is_macro_rejects_non_symbol() {
        let interner = Interner::new();
        let registry = MacroRegistry::new();

        let ctx = ctx_with_macros(&interner, &registry);
        let args = [Value::Integer(Integer::from_i64(42_i64))];
        let result = native_is_macro(&args, &ctx);

        assert!(result.is_err());
    }

    #[test]
    fn is_macro_rejects_wrong_arity() {
        let mut interner = Interner::new();
        let registry = MacroRegistry::new();

        // No arguments
        {
            let ctx = ctx_with_macros(&interner, &registry);
            let result = native_is_macro(&[], &ctx);
            assert!(result.is_err());
        }

        // Two arguments
        let sym1 = interner.intern("foo");
        let sym2 = interner.intern("bar");
        let ctx = ctx_with_macros(&interner, &registry);
        let args = [Value::Symbol(sym1), Value::Symbol(sym2)];
        let result = native_is_macro(&args, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn expand_once_returns_non_list_unchanged() {
        let mut interner = Interner::new();
        let registry = MacroRegistry::new();

        let sym = interner.intern("foo");
        let ctx = ctx_with_macros(&interner, &registry);
        let args = [Value::Symbol(sym)];
        let result = native_expand_once(&args, &ctx).unwrap();

        assert_eq!(result, Value::Symbol(sym));
    }

    #[test]
    fn expand_once_returns_non_macro_list_unchanged() {
        let mut interner = Interner::new();
        let registry = MacroRegistry::new();

        let sym = interner.intern("not-a-macro");
        let list = List::from_vec(alloc::vec![
            Value::Symbol(sym),
            Value::Integer(Integer::from_i64(1_i64)),
        ]);
        let ctx = ctx_with_macros(&interner, &registry);
        let args = [Value::List(list.clone())];
        let result = native_expand_once(&args, &ctx).unwrap();

        assert_eq!(result, Value::List(list));
    }

    #[test]
    fn expand_once_expands_macro_call() {
        let mut interner = Interner::new();
        let mut registry = MacroRegistry::new();

        // Pre-intern collection primitives (required for macro expansion)
        let _primitives = intern_primitives(&mut interner);

        // Register an identity macro
        let macro_name = interner.intern("identity");
        let chunk = Arc::new(make_identity_chunk());
        let def = MacroDefinition::new(chunk, 1_u8, String::from("identity"));
        registry.register(macro_name, def);

        // Create call: (identity 42)
        let list = List::from_vec(alloc::vec![
            Value::Symbol(macro_name),
            Value::Integer(Integer::from_i64(42_i64)),
        ]);
        let ctx = ctx_with_macros(&interner, &registry);
        let args = [Value::List(list)];

        let result = native_expand_once(&args, &ctx).unwrap();

        // Should return the argument (identity returns first arg)
        assert_eq!(result, Value::Integer(Integer::from_i64(42_i64)));
    }

    #[test]
    fn expand_fully_stops_when_stable() {
        let mut interner = Interner::new();
        let registry = MacroRegistry::new();

        // A non-macro form should return immediately
        let sym = interner.intern("foo");
        let ctx = ctx_with_macros(&interner, &registry);
        let args = [Value::Symbol(sym)];
        let result = native_expand_fully(&args, &ctx).unwrap();

        assert_eq!(result, Value::Symbol(sym));
    }

    #[test]
    fn values_equal_handles_lists() {
        let list1 = Value::List(List::from_vec(alloc::vec![
            Value::Integer(Integer::from_i64(1_i64)),
            Value::Integer(Integer::from_i64(2_i64)),
        ]));
        let list2 = Value::List(List::from_vec(alloc::vec![
            Value::Integer(Integer::from_i64(1_i64)),
            Value::Integer(Integer::from_i64(2_i64)),
        ]));
        let list3 = Value::List(List::from_vec(alloc::vec![
            Value::Integer(Integer::from_i64(1_i64)),
            Value::Integer(Integer::from_i64(3_i64)),
        ]));

        assert!(values_equal(&list1, &list2));
        assert!(!values_equal(&list1, &list3));
    }
}
