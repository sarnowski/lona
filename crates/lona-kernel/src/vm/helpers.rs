// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Helper functions for the VM interpreter.
//!
//! Provides type name utilities and value comparison functions.

use lona_core::value::Value;
use lonala_compiler::Constant;

use super::numeric::integer_to_f64;

/// Returns the type name of a value.
pub const fn value_type_name(value: &Value) -> &'static str {
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
        // Value is non-exhaustive, handle future variants
        _ => "unknown",
    }
}

/// Returns the type name of a constant.
pub const fn constant_type_name(constant: &Constant) -> &'static str {
    match *constant {
        Constant::Nil => "nil",
        Constant::Bool(_) => "boolean",
        Constant::Integer(_) => "integer",
        Constant::Float(_) => "float",
        Constant::String(_) => "string",
        Constant::Symbol(_) => "symbol",
        Constant::List(_) => "list",
        Constant::Vector(_) => "vector",
        // Constant is non-exhaustive, handle future variants
        _ => "unknown",
    }
}

/// Returns a description of the types in a binary operation.
pub const fn binary_type_description(left: &Value, right: &Value) -> &'static str {
    match (left, right) {
        (&Value::Nil, _) | (_, &Value::Nil) => "nil",
        (&Value::Bool(_), _) | (_, &Value::Bool(_)) => "boolean",
        (&Value::Symbol(_), _) | (_, &Value::Symbol(_)) => "symbol",
        (&Value::String(_), _) | (_, &Value::String(_)) => "string",
        (&Value::List(_), _) | (_, &Value::List(_)) => "list",
        (&Value::Vector(_), _) | (_, &Value::Vector(_)) => "vector",
        (&Value::Map(_), _) | (_, &Value::Map(_)) => "map",
        _ => "non-number",
    }
}

/// Tests if two values are equal.
#[expect(
    clippy::float_cmp,
    reason = "[approved] VM equality semantics require exact float comparison"
)]
pub fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (&Value::Nil, &Value::Nil) => true,
        (&Value::Bool(left_bool), &Value::Bool(right_bool)) => left_bool == right_bool,
        (&Value::Integer(ref left_int), &Value::Integer(ref right_int)) => left_int == right_int,
        (&Value::Float(left_float), &Value::Float(right_float)) => left_float == right_float,
        (&Value::Ratio(ref left_ratio), &Value::Ratio(ref right_ratio)) => {
            left_ratio == right_ratio
        }
        (&Value::Symbol(left_sym), &Value::Symbol(right_sym)) => left_sym == right_sym,
        (&Value::String(ref left_str), &Value::String(ref right_str)) => left_str == right_str,
        (&Value::List(ref left_list), &Value::List(ref right_list)) => left_list == right_list,
        (&Value::Vector(ref left_vec), &Value::Vector(ref right_vec)) => left_vec == right_vec,
        (&Value::Map(ref left_map), &Value::Map(ref right_map)) => left_map == right_map,
        // Cross-type numeric comparison: Integer <=> Float
        (&Value::Integer(ref left_int), &Value::Float(right_float)) => {
            let left_as_float = integer_to_f64(left_int);
            left_as_float == right_float
        }
        (&Value::Float(left_float), &Value::Integer(ref right_int)) => {
            let right_as_float = integer_to_f64(right_int);
            left_float == right_as_float
        }
        // Cross-type numeric comparison: Integer <=> Ratio
        (&Value::Integer(ref left_int), &Value::Ratio(ref right_ratio)) => right_ratio
            .to_integer()
            .is_some_and(|right_int| left_int == &right_int),
        (&Value::Ratio(ref left_ratio), &Value::Integer(ref right_int)) => left_ratio
            .to_integer()
            .is_some_and(|left_int| &left_int == right_int),
        // Cross-type numeric comparison: Float <=> Ratio
        (&Value::Float(left_float), &Value::Ratio(ref right_ratio)) => {
            let right_float = right_ratio.to_f64().unwrap_or(f64::NAN);
            left_float == right_float
        }
        (&Value::Ratio(ref left_ratio), &Value::Float(right_float)) => {
            let left_float = left_ratio.to_f64().unwrap_or(f64::NAN);
            left_float == right_float
        }
        _ => false,
    }
}
