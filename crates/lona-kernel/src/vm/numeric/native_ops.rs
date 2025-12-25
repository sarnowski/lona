// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Frame-free arithmetic operations for native functions.
//!
//! These functions perform arithmetic with type promotion but don't require
//! a `Frame` for error location. They return `NativeError` instead.

use lona_core::error_context::TypeExpectation;
use lona_core::integer::Integer;
use lona_core::ratio::Ratio;
use lona_core::value::Value;
use num_traits::Zero as _;

use super::{get_non_numeric_kind, integer_to_f64};
use crate::vm::natives::NativeError;

/// Performs addition with type promotion (frame-free version for native functions).
///
/// Same promotion rules as [`super::add`], but returns [`NativeError`] instead of
/// requiring a `Frame` for error location.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
#[inline]
pub fn add_values(left: &Value, right: &Value) -> Result<Value, NativeError> {
    match (left, right) {
        // Integer + Integer → Integer
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => Ok(Value::Integer(lhs + rhs)),

        // Float + Float → Float
        (&Value::Float(lhs), &Value::Float(rhs)) => Ok(Value::Float(lhs + rhs)),

        // Integer + Float → Float
        (&Value::Integer(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = integer_to_f64(lhs);
            Ok(Value::Float(lhs_float + rhs))
        }
        (&Value::Float(lhs), &Value::Integer(ref rhs)) => {
            let rhs_float = integer_to_f64(rhs);
            Ok(Value::Float(lhs + rhs_float))
        }

        // Ratio + Ratio → Ratio
        (&Value::Ratio(ref lhs), &Value::Ratio(ref rhs)) => Ok(Value::Ratio(lhs + rhs)),

        // Integer + Ratio → Ratio
        (&Value::Integer(ref lhs), &Value::Ratio(ref rhs)) => {
            let lhs_ratio = Ratio::from_integer(lhs.clone());
            Ok(Value::Ratio(lhs_ratio + rhs.clone()))
        }
        (&Value::Ratio(ref lhs), &Value::Integer(ref rhs)) => {
            let rhs_ratio = Ratio::from_integer(rhs.clone());
            Ok(Value::Ratio(lhs.clone() + rhs_ratio))
        }

        // Ratio + Float → Float
        (&Value::Ratio(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = lhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Float(lhs_float + rhs))
        }
        (&Value::Float(lhs), &Value::Ratio(ref rhs)) => {
            let rhs_float = rhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Float(lhs + rhs_float))
        }

        _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Numeric,
            got: get_non_numeric_kind(if left.kind().is_numeric() {
                right
            } else {
                left
            }),
            arg_index: u8::from(left.kind().is_numeric()),
        }),
    }
}

/// Performs subtraction with type promotion (frame-free version for native functions).
///
/// Same promotion rules as [`super::sub`], but returns [`NativeError`] instead of
/// requiring a `Frame` for error location.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
#[inline]
pub fn sub_values(left: &Value, right: &Value) -> Result<Value, NativeError> {
    match (left, right) {
        // Integer - Integer → Integer
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => Ok(Value::Integer(lhs - rhs)),

        // Float - Float → Float
        (&Value::Float(lhs), &Value::Float(rhs)) => Ok(Value::Float(lhs - rhs)),

        // Integer - Float → Float
        (&Value::Integer(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = integer_to_f64(lhs);
            Ok(Value::Float(lhs_float - rhs))
        }
        (&Value::Float(lhs), &Value::Integer(ref rhs)) => {
            let rhs_float = integer_to_f64(rhs);
            Ok(Value::Float(lhs - rhs_float))
        }

        // Ratio - Ratio → Ratio
        (&Value::Ratio(ref lhs), &Value::Ratio(ref rhs)) => Ok(Value::Ratio(lhs - rhs)),

        // Integer - Ratio → Ratio
        (&Value::Integer(ref lhs), &Value::Ratio(ref rhs)) => {
            let lhs_ratio = Ratio::from_integer(lhs.clone());
            Ok(Value::Ratio(lhs_ratio - rhs.clone()))
        }
        (&Value::Ratio(ref lhs), &Value::Integer(ref rhs)) => {
            let rhs_ratio = Ratio::from_integer(rhs.clone());
            Ok(Value::Ratio(lhs.clone() - rhs_ratio))
        }

        // Ratio - Float → Float
        (&Value::Ratio(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = lhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Float(lhs_float - rhs))
        }
        (&Value::Float(lhs), &Value::Ratio(ref rhs)) => {
            let rhs_float = rhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Float(lhs - rhs_float))
        }

        _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Numeric,
            got: get_non_numeric_kind(if left.kind().is_numeric() {
                right
            } else {
                left
            }),
            arg_index: u8::from(left.kind().is_numeric()),
        }),
    }
}

/// Negates a numeric value (frame-free version for native functions).
///
/// Returns the negation of the value:
/// - Integer → Integer (negated)
/// - Float → Float (negated)
/// - Ratio → Ratio (negated)
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio negation is safe with arbitrary precision"
)]
#[inline]
pub fn negate_value(val: &Value) -> Result<Value, NativeError> {
    match *val {
        Value::Integer(ref int_val) => Ok(Value::Integer(-int_val.clone())),
        Value::Float(float_val) => Ok(Value::Float(-float_val)),
        Value::Ratio(ref ratio_val) => Ok(Value::Ratio(-ratio_val.clone())),
        // Non-numeric types cannot be negated
        Value::Nil
        | Value::Bool(_)
        | Value::Symbol(_)
        | Value::NativeFunction(_)
        | Value::String(_)
        | Value::List(_)
        | Value::Vector(_)
        | Value::Map(_)
        | Value::Function(_)
        | _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Numeric,
            got: val.kind(),
            arg_index: 0_u8,
        }),
    }
}

/// Performs multiplication with type promotion (frame-free version for native functions).
///
/// Promotion rules:
/// - Integer * Integer → Integer
/// - Integer * Float → Float
/// - Integer * Ratio → Ratio
/// - Ratio * Ratio → Ratio
/// - Ratio * Float → Float
/// - Float * Float → Float
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
#[inline]
pub fn mul_values(left: &Value, right: &Value) -> Result<Value, NativeError> {
    match (left, right) {
        // Integer * Integer → Integer
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => Ok(Value::Integer(lhs * rhs)),

        // Float * Float → Float
        (&Value::Float(lhs), &Value::Float(rhs)) => Ok(Value::Float(lhs * rhs)),

        // Integer * Float → Float
        (&Value::Integer(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = integer_to_f64(lhs);
            Ok(Value::Float(lhs_float * rhs))
        }
        (&Value::Float(lhs), &Value::Integer(ref rhs)) => {
            let rhs_float = integer_to_f64(rhs);
            Ok(Value::Float(lhs * rhs_float))
        }

        // Ratio * Ratio → Ratio
        (&Value::Ratio(ref lhs), &Value::Ratio(ref rhs)) => Ok(Value::Ratio(lhs * rhs)),

        // Integer * Ratio → Ratio
        (&Value::Integer(ref lhs), &Value::Ratio(ref rhs)) => {
            let lhs_ratio = Ratio::from_integer(lhs.clone());
            Ok(Value::Ratio(lhs_ratio * rhs.clone()))
        }
        (&Value::Ratio(ref lhs), &Value::Integer(ref rhs)) => {
            let rhs_ratio = Ratio::from_integer(rhs.clone());
            Ok(Value::Ratio(lhs.clone() * rhs_ratio))
        }

        // Ratio * Float → Float
        (&Value::Ratio(ref lhs), &Value::Float(rhs)) => {
            let lhs_float = lhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Float(lhs_float * rhs))
        }
        (&Value::Float(lhs), &Value::Ratio(ref rhs)) => {
            let rhs_float = rhs.to_f64().unwrap_or(f64::NAN);
            Ok(Value::Float(lhs * rhs_float))
        }

        _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Numeric,
            got: get_non_numeric_kind(if left.kind().is_numeric() {
                right
            } else {
                left
            }),
            arg_index: u8::from(left.kind().is_numeric()),
        }),
    }
}

/// Performs division with type promotion (frame-free version for native functions).
///
/// Promotion rules:
/// - Integer / Integer → Ratio (or Integer if exact)
/// - Integer / Float → Float
/// - Integer / Ratio → Ratio
/// - Ratio / Ratio → Ratio
/// - Ratio / Float → Float
/// - Float / Float → Float
///
/// Returns `DivisionByZero` error when dividing by zero.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
#[inline]
pub fn div_values(left: &Value, right: &Value) -> Result<Value, NativeError> {
    match (left, right) {
        // Integer / Integer → Ratio (exact division)
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                return Err(NativeError::DivisionByZero);
            }
            // Create a ratio for exact division; return Integer if denominator = 1
            let ratio = Ratio::new(lhs, rhs);
            ratio
                .to_integer()
                .map_or(Ok(Value::Ratio(ratio)), |int_val| {
                    Ok(Value::Integer(int_val))
                })
        }
        // Float / Float → Float
        (&Value::Float(lhs), &Value::Float(rhs)) => {
            if rhs == 0.0 {
                Err(NativeError::DivisionByZero)
            } else {
                Ok(Value::Float(lhs / rhs))
            }
        }
        // Integer / Float → Float
        (&Value::Integer(ref lhs), &Value::Float(rhs)) => {
            if rhs == 0.0 {
                return Err(NativeError::DivisionByZero);
            }
            Ok(Value::Float(integer_to_f64(lhs) / rhs))
        }
        (&Value::Float(lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                return Err(NativeError::DivisionByZero);
            }
            Ok(Value::Float(lhs / integer_to_f64(rhs)))
        }
        // Ratio / Ratio → Ratio
        (&Value::Ratio(ref lhs), &Value::Ratio(ref rhs)) => {
            if rhs.is_zero() {
                Err(NativeError::DivisionByZero)
            } else {
                Ok(Value::Ratio(lhs / rhs))
            }
        }
        // Integer / Ratio → Ratio
        (&Value::Integer(ref lhs), &Value::Ratio(ref rhs)) => {
            if rhs.is_zero() {
                return Err(NativeError::DivisionByZero);
            }
            Ok(Value::Ratio(Ratio::from_integer(lhs.clone()) / rhs.clone()))
        }
        (&Value::Ratio(ref lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                return Err(NativeError::DivisionByZero);
            }
            Ok(Value::Ratio(lhs.clone() / Ratio::from_integer(rhs.clone())))
        }
        // Ratio / Float → Float
        (&Value::Ratio(ref lhs), &Value::Float(rhs)) => {
            if rhs == 0.0 {
                return Err(NativeError::DivisionByZero);
            }
            Ok(Value::Float(lhs.to_f64().unwrap_or(f64::NAN) / rhs))
        }
        (&Value::Float(lhs), &Value::Ratio(ref rhs)) => {
            if rhs.is_zero() {
                return Err(NativeError::DivisionByZero);
            }
            Ok(Value::Float(lhs / rhs.to_f64().unwrap_or(f64::NAN)))
        }
        _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Numeric,
            got: get_non_numeric_kind(if left.kind().is_numeric() {
                right
            } else {
                left
            }),
            arg_index: u8::from(left.kind().is_numeric()),
        }),
    }
}

/// Computes the reciprocal (1/x) of a numeric value.
///
/// Returns:
/// - `(/ 1)` → 1 (exact integer)
/// - `(/ -1)` → -1 (exact integer)
/// - `(/ 2)` → 1/2 (ratio)
/// - `(/ 0.5)` → 2.0 (float)
///
/// Returns `DivisionByZero` error when the value is zero.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
#[inline]
pub fn inverse_value(val: &Value) -> Result<Value, NativeError> {
    match *val {
        Value::Integer(ref int_val) => {
            if int_val.is_zero() {
                return Err(NativeError::DivisionByZero);
            }
            // 1/n as a ratio; simplify to integer if denominator is 1
            let one = Integer::from_i64(1);
            let ratio = Ratio::new(&one, int_val);
            ratio
                .to_integer()
                .map_or(Ok(Value::Ratio(ratio)), |int_result| {
                    Ok(Value::Integer(int_result))
                })
        }
        Value::Float(float_val) => {
            if float_val == 0.0 {
                Err(NativeError::DivisionByZero)
            } else {
                Ok(Value::Float(1.0 / float_val))
            }
        }
        Value::Ratio(ref ratio_val) => {
            if ratio_val.is_zero() {
                Err(NativeError::DivisionByZero)
            } else {
                // Reciprocal of a/b is b/a
                let one = Ratio::from_integer(Integer::from_i64(1));
                Ok(Value::Ratio(one / ratio_val.clone()))
            }
        }
        // Non-numeric types cannot have a reciprocal
        Value::Nil
        | Value::Bool(_)
        | Value::Symbol(_)
        | Value::NativeFunction(_)
        | Value::String(_)
        | Value::List(_)
        | Value::Vector(_)
        | Value::Map(_)
        | Value::Function(_)
        | _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Numeric,
            got: val.kind(),
            arg_index: 0_u8,
        }),
    }
}

/// Performs modulo with type promotion (frame-free version for native functions).
///
/// Modulo is only supported for Integer and Float types (not Ratio).
///
/// Returns `DivisionByZero` error when the divisor is zero.
/// Returns `TypeError` for Ratio types.
#[expect(
    clippy::modulo_arithmetic,
    reason = "[approved] Standard IEEE 754 float modulo for language runtime"
)]
#[inline]
pub fn modulo_values(left: &Value, right: &Value) -> Result<Value, NativeError> {
    match (left, right) {
        // Integer % Integer → Integer
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                Err(NativeError::DivisionByZero)
            } else {
                lhs.checked_rem(rhs).map_or_else(
                    || Err(NativeError::DivisionByZero),
                    |result| Ok(Value::Integer(result)),
                )
            }
        }

        // Float % Float → Float
        (&Value::Float(lhs), &Value::Float(rhs)) => {
            if rhs == 0.0 {
                Err(NativeError::DivisionByZero)
            } else {
                Ok(Value::Float(lhs % rhs))
            }
        }

        // Integer % Float → Float
        (&Value::Integer(ref lhs), &Value::Float(rhs)) => {
            if rhs == 0.0 {
                Err(NativeError::DivisionByZero)
            } else {
                let lhs_float = integer_to_f64(lhs);
                Ok(Value::Float(lhs_float % rhs))
            }
        }
        (&Value::Float(lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                Err(NativeError::DivisionByZero)
            } else {
                let rhs_float = integer_to_f64(rhs);
                Ok(Value::Float(lhs % rhs_float))
            }
        }

        // Ratio types are not supported for modulo
        (&Value::Ratio(_), _) | (_, &Value::Ratio(_)) => Err(NativeError::TypeError {
            expected: TypeExpectation::IntegerOrFloat,
            got: get_non_numeric_kind(if matches!(left, Value::Ratio(_)) {
                left
            } else {
                right
            }),
            arg_index: u8::from(!matches!(left, Value::Ratio(_))),
        }),

        _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Numeric,
            got: get_non_numeric_kind(if left.kind().is_numeric() {
                right
            } else {
                left
            }),
            arg_index: u8::from(left.kind().is_numeric()),
        }),
    }
}
