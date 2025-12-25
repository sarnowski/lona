// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Comparison native functions (=, <, >, <=, >=).

use lona_core::symbol;
use lona_core::value::Value;

use super::{NativeContext, NativeError};
use crate::vm::helpers::values_equal;
use crate::vm::numeric;

/// The names of all comparison primitives.
pub const COMPARISON_PRIMITIVE_NAMES: &[&str] = &["=", "<", ">", "<=", ">="];

/// Pre-interns all comparison primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `COMPARISON_PRIMITIVE_NAMES`.
#[inline]
pub fn intern_comparison_primitives(
    interner: &mut symbol::Interner,
) -> alloc::vec::Vec<symbol::Id> {
    COMPARISON_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up comparison primitive symbols from an immutable interner.
///
/// This is used when primitives should already be interned (e.g., by the REPL)
/// and we only have an immutable reference to the interner.
///
/// Returns `Some(symbols)` if all primitives are found, `None` otherwise.
#[inline]
#[must_use]
pub fn lookup_comparison_primitives(
    interner: &symbol::Interner,
) -> Option<alloc::vec::Vec<symbol::Id>> {
    COMPARISON_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all comparison primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_comparison_primitives` with
/// the same interner.
///
/// Each comparison function is registered:
/// - As a native function in the registry (for execution)
/// - As a `NativeFunction` value in globals (for first-class use)
#[inline]
pub fn register_comparison_primitives(
    vm: &mut crate::vm::interpreter::Vm<'_>,
    symbols: &[symbol::Id],
) {
    use super::NativeFn;

    let funcs: &[NativeFn] = &[native_eq, native_lt, native_gt, native_le, native_ge];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        // Use NativeFunction for first-class function support
        vm.set_global(*sym, Value::NativeFunction(*sym));
    }
}

/// Native implementation of `=` (equality).
///
/// Handles all arities:
/// - `(=)` → true (vacuously, but typically error per Clojure spec)
/// - `(= x)` → true (vacuously)
/// - `(= a b ...)` → true if all arguments are semantically equal pairwise
#[inline]
pub fn native_eq(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    // Per Clojure spec: 0 or 1 args returns true (vacuously)
    if args.len() <= 1_usize {
        return Ok(Value::Bool(true));
    }

    // Check all adjacent pairs
    for pair in args.windows(2_usize) {
        let left = pair.first().ok_or(NativeError::Error("missing argument"))?;
        let right = pair
            .get(1_usize)
            .ok_or(NativeError::Error("missing argument"))?;
        if !values_equal(left, right) {
            return Ok(Value::Bool(false));
        }
    }

    Ok(Value::Bool(true))
}

/// Native implementation of `<` (less than).
///
/// Handles arities:
/// - `(<)` → true (vacuously, per Clojure)
/// - `(< x)` → true (vacuously, per Clojure)
/// - `(< a b ...)` → true if args are in strictly increasing order
#[inline]
pub fn native_lt(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    compare_chain(args, |ord| ord == core::cmp::Ordering::Less)
}

/// Native implementation of `>` (greater than).
///
/// Handles arities:
/// - `(>)` → true (vacuously, per Clojure)
/// - `(> x)` → true (vacuously, per Clojure)
/// - `(> a b ...)` → true if args are in strictly decreasing order
#[inline]
pub fn native_gt(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    compare_chain(args, |ord| ord == core::cmp::Ordering::Greater)
}

/// Native implementation of `<=` (less than or equal).
///
/// Handles arities:
/// - `(<=)` → true (vacuously, per Clojure)
/// - `(<= x)` → true (vacuously, per Clojure)
/// - `(<= a b ...)` → true if args are in non-decreasing order
#[inline]
pub fn native_le(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    compare_chain(args, |ord| {
        ord == core::cmp::Ordering::Less || ord == core::cmp::Ordering::Equal
    })
}

/// Native implementation of `>=` (greater than or equal).
///
/// Handles arities:
/// - `(>=)` → true (vacuously, per Clojure)
/// - `(>= x)` → true (vacuously, per Clojure)
/// - `(>= a b ...)` → true if args are in non-increasing order
#[inline]
pub fn native_ge(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    compare_chain(args, |ord| {
        ord == core::cmp::Ordering::Greater || ord == core::cmp::Ordering::Equal
    })
}

/// Performs a chained comparison on numeric values.
///
/// Returns true if all adjacent pairs satisfy the comparison predicate.
fn compare_chain<F>(args: &[Value], pred: F) -> Result<Value, NativeError>
where
    F: Fn(core::cmp::Ordering) -> bool,
{
    // Per Clojure spec: 0 or 1 args returns true (vacuously)
    if args.len() <= 1_usize {
        return Ok(Value::Bool(true));
    }

    // Check all adjacent pairs
    for (idx, pair) in args.windows(2_usize).enumerate() {
        let left = pair.first().ok_or(NativeError::Error("missing argument"))?;
        let right = pair
            .get(1_usize)
            .ok_or(NativeError::Error("missing argument"))?;

        match numeric::compare_values(left, right) {
            Ok(ord) => {
                if !pred(ord) {
                    return Ok(Value::Bool(false));
                }
            }
            Err(err) => {
                // Adjust arg_index: pair[0] is at position idx, pair[1] is at idx+1
                if let NativeError::TypeError {
                    expected,
                    got,
                    arg_index: original_index,
                } = err
                {
                    return Err(NativeError::TypeError {
                        expected,
                        got,
                        arg_index: if original_index == 0 {
                            u8::try_from(idx).unwrap_or(u8::MAX)
                        } else {
                            u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                        },
                    });
                }
                return Err(err);
            }
        }
    }

    Ok(Value::Bool(true))
}
