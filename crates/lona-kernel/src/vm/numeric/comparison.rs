// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Comparison operations with type promotion.

use lona_core::error_context::TypeExpectation;
use lona_core::ratio::Ratio;
use lona_core::value::Value;

use super::integer_to_f64;
use crate::vm::error::{Error, Kind as ErrorKind};
use crate::vm::frame::Frame;
use crate::vm::natives::NativeError;

/// Performs a numeric comparison operation.
#[inline]
pub fn compare<F>(
    left: &Value,
    right: &Value,
    frame: &Frame<'_>,
    float_cmp: F,
) -> Result<Value, Error>
where
    F: Fn(f64, f64) -> bool,
{
    match (left, right) {
        // Integer <=> Integer
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => {
            Ok(Value::Bool(integer_compare(lhs, rhs, &float_cmp)))
        }

        // Float <=> Float
        (&Value::Float(lhs), &Value::Float(rhs)) => Ok(Value::Bool(float_cmp(lhs, rhs))),

        // Integer <=> Float
        (&Value::Integer(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = integer_to_f64(lhs);
            Ok(Value::Bool(float_cmp(lhs_float, rhs)))
        }
        (&Value::Float(lhs), &Value::Integer(ref rhs)) => {
            let rhs_float = integer_to_f64(rhs);
            Ok(Value::Bool(float_cmp(lhs, rhs_float)))
        }

        // Ratio <=> Ratio
        (&Value::Ratio(ref lhs), &Value::Ratio(ref rhs)) => {
            let lhs_float = lhs.to_f64().unwrap_or(f64::NAN);
            let rhs_float = rhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Bool(float_cmp(lhs_float, rhs_float)))
        }

        // Integer <=> Ratio
        (&Value::Integer(ref lhs), &Value::Ratio(ref rhs)) => {
            let lhs_float = integer_to_f64(lhs);
            let rhs_float = rhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Bool(float_cmp(lhs_float, rhs_float)))
        }
        (&Value::Ratio(ref lhs), &Value::Integer(ref rhs)) => {
            let lhs_float = lhs.to_f64().unwrap_or(f64::NAN);
            let rhs_float = integer_to_f64(rhs);
            Ok(Value::Bool(float_cmp(lhs_float, rhs_float)))
        }

        // Float <=> Ratio
        (&Value::Float(lhs), &Value::Ratio(ref rhs)) => {
            let rhs_float = rhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Bool(float_cmp(lhs, rhs_float)))
        }
        (&Value::Ratio(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = lhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Bool(float_cmp(lhs_float, rhs)))
        }

        // String <=> String (lexicographic)
        (&Value::String(ref lhs), &Value::String(ref rhs)) => {
            // Map string ordering to f64 values that work with float comparison closure:
            // Less → (0.0, 1.0), Equal → (0.0, 0.0), Greater → (1.0, 0.0)
            let (lhs_val, rhs_val) = match lhs.cmp(rhs) {
                core::cmp::Ordering::Less => (0.0, 1.0),
                core::cmp::Ordering::Equal => (0.0, 0.0),
                core::cmp::Ordering::Greater => (1.0, 0.0),
            };
            Ok(Value::Bool(float_cmp(lhs_val, rhs_val)))
        }

        // Non-comparable types or mixed types (number vs string)
        _ => Err(Error::new(
            ErrorKind::TypeError {
                operation: "compare",
                expected: TypeExpectation::Comparable,
                got: if left.kind().is_numeric() || matches!(left, Value::String(_)) {
                    right.kind()
                } else {
                    left.kind()
                },
                operand: None,
            },
            frame.current_location(),
        )),
    }
}

/// Compares two numeric values and returns their ordering.
///
/// Returns `Ordering::Equal`, `Ordering::Less`, or `Ordering::Greater`
/// for comparable numeric values. Returns a `TypeError` for non-numeric types.
///
/// Note: NaN comparisons follow IEEE 754 semantics where NaN is unordered
/// with respect to all values, but this function will return an ordering
/// based on f64's `partial_cmp` behavior.
#[inline]
pub fn compare_values(left: &Value, right: &Value) -> Result<core::cmp::Ordering, NativeError> {
    // Error for NaN comparisons
    const NAN_ERROR: NativeError = NativeError::Error("cannot compare NaN values");

    match (left, right) {
        // Integer <=> Integer
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => Ok(lhs.cmp(rhs)),

        // Float <=> Float
        (&Value::Float(lhs), &Value::Float(rhs)) => lhs.partial_cmp(&rhs).ok_or(NAN_ERROR),

        // Integer <=> Float
        (&Value::Integer(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = integer_to_f64(lhs);
            lhs_float.partial_cmp(&rhs).ok_or(NAN_ERROR)
        }
        (&Value::Float(lhs), &Value::Integer(ref rhs)) => {
            let rhs_float = integer_to_f64(rhs);
            lhs.partial_cmp(&rhs_float).ok_or(NAN_ERROR)
        }

        // Ratio <=> Ratio
        (&Value::Ratio(ref lhs), &Value::Ratio(ref rhs)) => Ok(lhs.cmp(rhs)),

        // Integer <=> Ratio
        (&Value::Integer(ref lhs), &Value::Ratio(ref rhs)) => {
            let lhs_ratio = Ratio::from_integer(lhs.clone());
            Ok(lhs_ratio.cmp(rhs))
        }
        (&Value::Ratio(ref lhs), &Value::Integer(ref rhs)) => {
            let rhs_ratio = Ratio::from_integer(rhs.clone());
            Ok(lhs.cmp(&rhs_ratio))
        }

        // Float <=> Ratio
        (&Value::Float(lhs), &Value::Ratio(ref rhs)) => {
            let rhs_float = rhs.to_f64().unwrap_or(f64::NAN);
            lhs.partial_cmp(&rhs_float).ok_or(NAN_ERROR)
        }
        (&Value::Ratio(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = lhs.to_f64().unwrap_or(f64::NAN);
            lhs_float.partial_cmp(&rhs).ok_or(NAN_ERROR)
        }

        // String <=> String (lexicographic)
        (&Value::String(ref lhs), &Value::String(ref rhs)) => Ok(lhs.cmp(rhs)),

        // Non-comparable types or mixed types (number vs string)
        _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Comparable,
            got: if left.kind().is_numeric() || matches!(left, Value::String(_)) {
                right.kind()
            } else {
                left.kind()
            },
            arg_index: u8::from(left.kind().is_numeric() || matches!(left, Value::String(_))),
        }),
    }
}

/// Compares two integers using the given float comparison function.
///
/// For integers, we first try to compare using the Integer type's Ord implementation,
/// then map the result to match the float comparison semantics.
fn integer_compare<F>(
    lhs: &lona_core::integer::Integer,
    rhs: &lona_core::integer::Integer,
    float_cmp: &F,
) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    // Convert to f64 and use float comparison
    // This ensures consistent semantics with mixed-type comparisons
    let lhs_float = integer_to_f64(lhs);
    let rhs_float = integer_to_f64(rhs);
    float_cmp(lhs_float, rhs_float)
}
