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

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::list::List;
use lona_core::symbol::{self, Interner};
use lona_core::value::Value;
use lonala_compiler::MacroRegistry;

use super::Vm;
use super::collections::{
    lookup_primitives, register_primitives as register_collection_primitives,
};
use super::natives::{NativeContext, NativeError};

/// Maximum depth for recursive macro expansion in `macroexpand`.
const MAX_EXPANSION_DEPTH: usize = 256;

/// Builds the effective macro arguments, handling rest parameters.
///
/// Fixed args: `raw_args[0..arity]`
/// Rest arg (if `has_rest`): `raw_args[arity..]` collected into a list
fn build_effective_macro_args(raw_args: &[Value], arity: u8, has_rest: bool) -> Vec<Value> {
    if has_rest {
        let arity_usize = usize::from(arity);
        let mut effective = Vec::with_capacity(arity_usize.saturating_add(1));
        for arg in raw_args.iter().take(arity_usize) {
            effective.push(arg.clone());
        }
        let rest_elements: Vec<Value> = raw_args.iter().skip(arity_usize).cloned().collect();
        effective.push(Value::List(List::from_vec(rest_elements)));
        effective
    } else {
        raw_args.to_vec()
    }
}

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
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let arg = args.first().ok_or(NativeError::ArityMismatch {
        expected: ArityExpectation::Exact(1_u8),
        got: 0_u8,
    })?;

    let Value::Symbol(ref sym) = *arg else {
        return Err(NativeError::TypeError {
            expected: TypeExpectation::Symbol,
            got: arg.kind(),
            arg_index: 0_u8,
        });
    };

    // If no macro registry, no macros exist
    let Some(registry) = ctx.macros() else {
        return Ok(Value::Bool(false));
    };

    Ok(Value::Bool(registry.contains(sym.id())))
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
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let form = args.first().ok_or(NativeError::ArityMismatch {
        expected: ArityExpectation::Exact(1_u8),
        got: 0_u8,
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
    let Some(&Value::Symbol(ref sym)) = list.first() else {
        return Ok(form.clone());
    };

    // Look up macro
    let Some(macro_def) = registry.get(sym.id()) else {
        return Ok(form.clone()); // Not a macro
    };

    // Extract arguments (skip the macro name)
    let macro_args: Vec<Value> = list.iter().skip(1).cloned().collect();

    // Find matching arity body
    let body = macro_def
        .find_body(macro_args.len())
        .ok_or(NativeError::Error("macro arity mismatch"))?;

    // Look up collection primitives (must be pre-interned by caller)
    let collection_symbols = lookup_primitives(interner)
        .ok_or(NativeError::Error("collection primitives not interned"))?;

    // Create a fresh VM for macro expansion
    let mut vm = Vm::new(interner);
    register_collection_primitives(&mut vm, &collection_symbols);

    // Build effective arguments (handling rest parameters)
    let effective_args = build_effective_macro_args(&macro_args, body.arity, body.has_rest);

    // Execute the macro's chunk with arguments
    let result = vm
        .execute_with_args(&body.chunk, &effective_args)
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
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let mut form = args
        .first()
        .ok_or(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: 0_u8,
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

/// Checks if two values are equal.
///
/// Used for macro expansion to detect when the form has stopped changing.
fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (&Value::Nil, &Value::Nil) => true,
        (&Value::Bool(lhs), &Value::Bool(rhs)) => lhs == rhs,
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => lhs == rhs,
        (&Value::Float(lhs), &Value::Float(rhs)) => floats_equal(lhs, rhs),
        (&Value::Ratio(ref lhs), &Value::Ratio(ref rhs)) => lhs == rhs,
        (&Value::Symbol(ref lhs), &Value::Symbol(ref rhs)) => lhs == rhs,
        (&Value::Keyword(lhs), &Value::Keyword(rhs)) => lhs == rhs,
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
        (&Value::Map(ref lhs), &Value::Map(ref rhs)) => {
            lhs.len() == rhs.len()
                && lhs.iter().all(|(key, val)| {
                    rhs.get(key.value())
                        .is_some_and(|rhs_val| values_equal(val, rhs_val))
                })
        }
        (&Value::Set(ref lhs), &Value::Set(ref rhs)) => lhs == rhs,
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
pub fn intern_primitives(interner: &symbol::Interner) -> alloc::vec::Vec<symbol::Id> {
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
        vm.set_global(*sym, Value::NativeFunction(*sym));
    }
}

#[cfg(test)]
#[path = "introspection_tests.rs"]
mod tests;
