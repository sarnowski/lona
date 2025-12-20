// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Numeric operations with type promotion for the VM.
//!
//! Handles arithmetic operations across Integer, Float, and Ratio types
//! with automatic type promotion rules.

use lona_core::error_context::TypeExpectation;
use lona_core::integer::Integer;
use lona_core::ratio::Ratio;
use lona_core::value::{self, Value};
use num_traits::{ToPrimitive as _, Zero as _};

use super::error::{Error, Kind as ErrorKind};
use super::frame::Frame;
use super::natives::NativeError;

/// Returns the `value::Kind` for the first non-numeric type in a binary operation.
const fn first_non_numeric_kind(left: &Value, right: &Value) -> lona_core::value::Kind {
    // Check left first, then right
    if matches!(left, Value::Integer(_) | Value::Float(_) | Value::Ratio(_)) {
        right.kind()
    } else {
        left.kind()
    }
}

/// Performs addition with type promotion.
///
/// Promotion rules:
/// - Integer + Integer → Integer
/// - Integer + Float → Float
/// - Integer + Ratio → Ratio
/// - Ratio + Ratio → Ratio
/// - Ratio + Float → Float
/// - Float + Float → Float
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
pub fn add(left: &Value, right: &Value, frame: &Frame<'_>) -> Result<Value, Error> {
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

        _ => Err(Error::new(
            ErrorKind::TypeError {
                operation: "+",
                expected: TypeExpectation::Numeric,
                got: first_non_numeric_kind(left, right),
                operand: None,
            },
            frame.current_location(),
        )),
    }
}

/// Performs subtraction with type promotion.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
pub fn sub(left: &Value, right: &Value, frame: &Frame<'_>) -> Result<Value, Error> {
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

        _ => Err(Error::new(
            ErrorKind::TypeError {
                operation: "-",
                expected: TypeExpectation::Numeric,
                got: first_non_numeric_kind(left, right),
                operand: None,
            },
            frame.current_location(),
        )),
    }
}

/// Performs multiplication with type promotion.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
pub fn mul(left: &Value, right: &Value, frame: &Frame<'_>) -> Result<Value, Error> {
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

        _ => Err(Error::new(
            ErrorKind::TypeError {
                operation: "*",
                expected: TypeExpectation::Numeric,
                got: first_non_numeric_kind(left, right),
                operand: None,
            },
            frame.current_location(),
        )),
    }
}

/// Creates a `DivisionByZero` error.
#[inline]
fn div_zero_err(frame: &Frame<'_>) -> Error {
    Error::new(ErrorKind::DivisionByZero, frame.current_location())
}

/// Performs division with type promotion.
///
/// Note: Integer / Integer creates a Ratio for exact division.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
pub fn div(left: &Value, right: &Value, frame: &Frame<'_>) -> Result<Value, Error> {
    match (left, right) {
        // Integer / Integer → Ratio (exact division)
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                return Err(div_zero_err(frame));
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
                Err(div_zero_err(frame))
            } else {
                Ok(Value::Float(lhs / rhs))
            }
        }
        // Integer / Float → Float
        (&Value::Integer(ref lhs), &Value::Float(rhs)) => {
            if rhs == 0.0 {
                return Err(div_zero_err(frame));
            }
            Ok(Value::Float(integer_to_f64(lhs) / rhs))
        }
        (&Value::Float(lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                return Err(div_zero_err(frame));
            }
            Ok(Value::Float(lhs / integer_to_f64(rhs)))
        }
        // Ratio / Ratio → Ratio
        (&Value::Ratio(ref lhs), &Value::Ratio(ref rhs)) => {
            if rhs.is_zero() {
                Err(div_zero_err(frame))
            } else {
                Ok(Value::Ratio(lhs / rhs))
            }
        }
        // Integer / Ratio → Ratio
        (&Value::Integer(ref lhs), &Value::Ratio(ref rhs)) => {
            if rhs.is_zero() {
                return Err(div_zero_err(frame));
            }
            Ok(Value::Ratio(Ratio::from_integer(lhs.clone()) / rhs.clone()))
        }
        (&Value::Ratio(ref lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                return Err(div_zero_err(frame));
            }
            Ok(Value::Ratio(lhs.clone() / Ratio::from_integer(rhs.clone())))
        }
        // Ratio / Float → Float
        (&Value::Ratio(ref lhs), &Value::Float(rhs)) => {
            if rhs == 0.0 {
                return Err(div_zero_err(frame));
            }
            Ok(Value::Float(lhs.to_f64().unwrap_or(f64::NAN) / rhs))
        }
        (&Value::Float(lhs), &Value::Ratio(ref rhs)) => {
            if rhs.is_zero() {
                return Err(div_zero_err(frame));
            }
            Ok(Value::Float(lhs / rhs.to_f64().unwrap_or(f64::NAN)))
        }
        _ => Err(Error::new(
            ErrorKind::TypeError {
                operation: "/",
                expected: TypeExpectation::Numeric,
                got: first_non_numeric_kind(left, right),
                operand: None,
            },
            frame.current_location(),
        )),
    }
}

/// Performs modulo with type promotion.
///
/// Note: Modulo only works with Integer and Float types.
#[expect(
    clippy::modulo_arithmetic,
    reason = "[approved] Standard IEEE 754 float modulo for language runtime"
)]
pub fn modulo(left: &Value, right: &Value, frame: &Frame<'_>) -> Result<Value, Error> {
    match (left, right) {
        // Integer % Integer → Integer
        (&Value::Integer(ref lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                Err(Error::new(
                    ErrorKind::DivisionByZero,
                    frame.current_location(),
                ))
            } else {
                lhs.checked_rem(rhs).map_or_else(
                    || {
                        Err(Error::new(
                            ErrorKind::DivisionByZero,
                            frame.current_location(),
                        ))
                    },
                    |result| Ok(Value::Integer(result)),
                )
            }
        }

        // Float % Float → Float
        (&Value::Float(lhs), &Value::Float(rhs)) => {
            if rhs == 0.0 {
                Err(Error::new(
                    ErrorKind::DivisionByZero,
                    frame.current_location(),
                ))
            } else {
                Ok(Value::Float(lhs % rhs))
            }
        }

        // Integer % Float → Float
        (&Value::Integer(ref lhs), &Value::Float(rhs)) => {
            if rhs == 0.0 {
                Err(Error::new(
                    ErrorKind::DivisionByZero,
                    frame.current_location(),
                ))
            } else {
                let lhs_float = integer_to_f64(lhs);
                Ok(Value::Float(lhs_float % rhs))
            }
        }
        (&Value::Float(lhs), &Value::Integer(ref rhs)) => {
            if rhs.is_zero() {
                Err(Error::new(
                    ErrorKind::DivisionByZero,
                    frame.current_location(),
                ))
            } else {
                let rhs_float = integer_to_f64(rhs);
                Ok(Value::Float(lhs % rhs_float))
            }
        }

        _ => Err(Error::new(
            ErrorKind::TypeError {
                operation: "%",
                expected: TypeExpectation::Numeric,
                got: first_non_numeric_kind(left, right),
                operand: None,
            },
            frame.current_location(),
        )),
    }
}

/// Performs a numeric comparison operation.
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

        _ => Err(Error::new(
            ErrorKind::TypeError {
                operation: "compare",
                expected: TypeExpectation::Numeric,
                got: first_non_numeric_kind(left, right),
                operand: None,
            },
            frame.current_location(),
        )),
    }
}

/// Converts an Integer to f64 for mixed-type arithmetic.
pub fn integer_to_f64(int_val: &Integer) -> f64 {
    // Use BigInt's ToPrimitive implementation
    int_val.to_bigint().to_f64().unwrap_or(f64::NAN)
}

/// Compares two integers using the given float comparison function.
///
/// For integers, we first try to compare using the Integer type's Ord implementation,
/// then map the result to match the float comparison semantics.
fn integer_compare<F>(lhs: &Integer, rhs: &Integer, float_cmp: &F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    // Convert to f64 and use float comparison
    // This ensures consistent semantics with mixed-type comparisons
    let lhs_float = integer_to_f64(lhs);
    let rhs_float = integer_to_f64(rhs);
    float_cmp(lhs_float, rhs_float)
}

// =============================================================================
// Frame-Free Numeric Helpers for Native Functions
// =============================================================================

/// Returns the `value::Kind` for type error reporting.
const fn get_non_numeric_kind(val: &Value) -> value::Kind {
    val.kind()
}

/// Performs addition with type promotion (frame-free version for native functions).
///
/// Same promotion rules as [`add`], but returns [`NativeError`] instead of
/// requiring a `Frame` for error location.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
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
/// Same promotion rules as [`sub`], but returns [`NativeError`] instead of
/// requiring a `Frame` for error location.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "[approved] Integer/Ratio arithmetic is safe with arbitrary precision"
)]
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
