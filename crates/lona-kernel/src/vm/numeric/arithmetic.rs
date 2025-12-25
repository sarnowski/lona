// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Arithmetic operations with type promotion.

use lona_core::error_context::TypeExpectation;
use lona_core::ratio::Ratio;
use lona_core::value::Value;
use num_traits::Zero as _;

use super::{first_non_numeric_kind, integer_to_f64};
use crate::vm::error::{Error, Kind as ErrorKind};
use crate::vm::frame::Frame;

// =============================================================================
// Frame-Based Arithmetic (for VM opcodes)
// =============================================================================

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
#[inline]
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
#[inline]
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
#[inline]
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
#[inline]
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
#[inline]
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
