// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Arithmetic native functions (+, -, *, /, mod).

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::integer::Integer;
use lona_core::symbol;
use lona_core::value::Value;

use super::{NativeContext, NativeError};
use crate::vm::numeric;

/// The names of all arithmetic primitives.
pub const ARITHMETIC_PRIMITIVE_NAMES: &[&str] = &["+", "-", "*", "/", "mod"];

/// Pre-interns all arithmetic primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `ARITHMETIC_PRIMITIVE_NAMES`.
#[inline]
pub fn intern_arithmetic_primitives(interner: &symbol::Interner) -> alloc::vec::Vec<symbol::Id> {
    ARITHMETIC_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up arithmetic primitive symbols from an immutable interner.
///
/// This is used when primitives should already be interned (e.g., by the REPL)
/// and we only have an immutable reference to the interner.
///
/// Returns `Some(symbols)` if all primitives are found, `None` otherwise.
#[inline]
#[must_use]
pub fn lookup_arithmetic_primitives(
    interner: &symbol::Interner,
) -> Option<alloc::vec::Vec<symbol::Id>> {
    ARITHMETIC_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all arithmetic primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_arithmetic_primitives` with
/// the same interner.
///
/// Each arithmetic function is registered:
/// - As a native function in the registry (for execution)
/// - As a `NativeFunction` value in globals (for first-class use)
#[inline]
pub fn register_arithmetic_primitives(
    vm: &mut crate::vm::interpreter::Vm<'_>,
    symbols: &[symbol::Id],
) {
    use super::NativeFn;

    let funcs: &[NativeFn] = &[native_add, native_sub, native_mul, native_div, native_mod];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        // Use NativeFunction for first-class function support
        vm.set_global(*sym, Value::NativeFunction(*sym));
    }
}

/// Native implementation of `+` (addition).
///
/// Handles all arities:
/// - `(+)` → 0 (identity)
/// - `(+ x)` → x
/// - `(+ a b ...)` → sum of all arguments
#[inline]
pub fn native_add(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    match args.len() {
        0 => Ok(Value::Integer(Integer::from_i64(0))),
        1 => {
            // Validate that the single argument is numeric
            let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            if !arg.kind().is_numeric() {
                return Err(NativeError::TypeError {
                    expected: TypeExpectation::Numeric,
                    got: arg.kind(),
                    arg_index: 0_u8,
                });
            }
            Ok(arg.clone())
        }
        _ => {
            let first_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            let mut acc = first_arg.clone();
            for (idx, arg) in args.iter().skip(1).enumerate() {
                acc = numeric::add_values(&acc, arg).map_err(|err| {
                    // Adjust arg_index for the actual position in the original args.
                    // If error.arg_index is 0, the left operand (accumulator) was wrong.
                    // On first iteration, this means the first argument is wrong (index 0).
                    // If error.arg_index is 1, the right operand (current arg) was wrong.
                    if let NativeError::TypeError {
                        expected,
                        got,
                        arg_index: original_index,
                    } = err
                    {
                        NativeError::TypeError {
                            expected,
                            got,
                            arg_index: if original_index == 0 {
                                0 // Error in first argument
                            } else {
                                u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                            },
                        }
                    } else {
                        err
                    }
                })?;
            }
            Ok(acc)
        }
    }
}

/// Native implementation of `-` (subtraction).
///
/// Handles arities:
/// - `(-)` → Error (requires at least one argument)
/// - `(- x)` → -x (negation)
/// - `(- a b ...)` → a - b - ...
#[inline]
pub fn native_sub(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    match args.len() {
        0 => Err(NativeError::ArityMismatch {
            expected: ArityExpectation::AtLeast(1_u8),
            got: 0_u8,
        }),
        1 => {
            let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            numeric::negate_value(arg)
        }
        _ => {
            let first_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            let mut acc = first_arg.clone();
            for (idx, arg) in args.iter().skip(1).enumerate() {
                acc = numeric::sub_values(&acc, arg).map_err(|err| {
                    // Adjust arg_index for the actual position in the original args.
                    // If error.arg_index is 0, the left operand (accumulator) was wrong.
                    // On first iteration, this means the first argument is wrong (index 0).
                    // If error.arg_index is 1, the right operand (current arg) was wrong.
                    if let NativeError::TypeError {
                        expected,
                        got,
                        arg_index: original_index,
                    } = err
                    {
                        NativeError::TypeError {
                            expected,
                            got,
                            arg_index: if original_index == 0 {
                                0 // Error in first argument
                            } else {
                                u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                            },
                        }
                    } else {
                        err
                    }
                })?;
            }
            Ok(acc)
        }
    }
}

/// Native implementation of `*` (multiplication).
///
/// Handles all arities:
/// - `(*)` → 1 (identity for multiplication)
/// - `(* x)` → x (validates numeric type)
/// - `(* a b ...)` → product of all arguments
#[inline]
pub fn native_mul(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    match args.len() {
        0 => Ok(Value::Integer(Integer::from_i64(1))),
        1 => {
            // Validate that the single argument is numeric
            let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            if !arg.kind().is_numeric() {
                return Err(NativeError::TypeError {
                    expected: TypeExpectation::Numeric,
                    got: arg.kind(),
                    arg_index: 0_u8,
                });
            }
            Ok(arg.clone())
        }
        _ => {
            let first_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            let mut acc = first_arg.clone();
            for (idx, arg) in args.iter().skip(1).enumerate() {
                acc = numeric::mul_values(&acc, arg).map_err(|err| {
                    // Adjust arg_index for the actual position in the original args.
                    // If error.arg_index is 0, the left operand (accumulator) was wrong.
                    // On first iteration, this means the first argument is wrong (index 0).
                    // If error.arg_index is 1, the right operand (current arg) was wrong.
                    if let NativeError::TypeError {
                        expected,
                        got,
                        arg_index: original_index,
                    } = err
                    {
                        NativeError::TypeError {
                            expected,
                            got,
                            arg_index: if original_index == 0 {
                                0 // Error in first argument
                            } else {
                                u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                            },
                        }
                    } else {
                        err
                    }
                })?;
            }
            Ok(acc)
        }
    }
}

/// Native implementation of `/` (division).
///
/// Handles arities:
/// - `(/)` → Error (requires at least one argument)
/// - `(/ x)` → 1/x (reciprocal)
/// - `(/ a b ...)` → a / b / ...
#[inline]
pub fn native_div(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    match args.len() {
        0 => Err(NativeError::ArityMismatch {
            expected: ArityExpectation::AtLeast(1_u8),
            got: 0_u8,
        }),
        1 => {
            let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            numeric::inverse_value(arg)
        }
        _ => {
            let first_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            let mut acc = first_arg.clone();
            for (idx, arg) in args.iter().skip(1).enumerate() {
                acc = numeric::div_values(&acc, arg).map_err(|err| {
                    // Adjust arg_index for the actual position in the original args.
                    // If error.arg_index is 0, the left operand (accumulator) was wrong.
                    // On first iteration, this means the first argument is wrong (index 0).
                    // If error.arg_index is 1, the right operand (current arg) was wrong.
                    if let NativeError::TypeError {
                        expected,
                        got,
                        arg_index: original_index,
                    } = err
                    {
                        NativeError::TypeError {
                            expected,
                            got,
                            arg_index: if original_index == 0 {
                                0 // Error in first argument
                            } else {
                                u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                            },
                        }
                    } else {
                        err
                    }
                })?;
            }
            Ok(acc)
        }
    }
}

/// Native implementation of `mod` (modulo).
///
/// Requires exactly 2 arguments:
/// - `(mod a b)` → a % b
#[inline]
pub fn native_mod(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 2_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(2_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let left = args.first().ok_or(NativeError::Error("missing argument"))?;
    let right = args
        .get(1_usize)
        .ok_or(NativeError::Error("missing argument"))?;

    numeric::modulo_values(left, right)
}
