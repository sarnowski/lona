// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Numeric operations with type promotion for the VM.
//!
//! Handles arithmetic operations across Integer, Float, and Ratio types
//! with automatic type promotion rules.

mod arithmetic;
mod comparison;
mod native_ops;

pub use arithmetic::{add, div, modulo, mul, sub};
pub use comparison::{compare, compare_values};
pub use native_ops::{
    add_values, div_values, inverse_value, modulo_values, mul_values, negate_value, sub_values,
};

use lona_core::integer::Integer;
use lona_core::value::Value;
use num_traits::ToPrimitive as _;

/// Returns the `value::Kind` for the first non-numeric type in a binary operation.
pub(super) const fn first_non_numeric_kind(left: &Value, right: &Value) -> lona_core::value::Kind {
    // Check left first, then right
    if matches!(left, Value::Integer(_) | Value::Float(_) | Value::Ratio(_)) {
        right.kind()
    } else {
        left.kind()
    }
}

/// Returns the `value::Kind` for type error reporting.
pub(super) const fn get_non_numeric_kind(val: &Value) -> lona_core::value::Kind {
    val.kind()
}

/// Converts an Integer to f64 for mixed-type arithmetic.
#[inline]
pub fn integer_to_f64(int_val: &Integer) -> f64 {
    // Use BigInt's ToPrimitive implementation
    int_val.to_bigint().to_f64().unwrap_or(f64::NAN)
}
