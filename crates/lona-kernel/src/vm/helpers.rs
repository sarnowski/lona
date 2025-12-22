// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Helper functions for the VM interpreter.
//!
//! Provides value comparison functions implementing Clojure-style semantic equality.

use lona_core::map::Map;
use lona_core::value::Value;

use super::numeric::integer_to_f64;

/// Compares two sequences element-by-element using semantic equality.
///
/// Returns true if both sequences have the same length and all corresponding
/// elements are semantically equal according to `values_equal`.
fn sequences_equal<'seq>(
    mut left: impl Iterator<Item = &'seq Value>,
    mut right: impl Iterator<Item = &'seq Value>,
) -> bool {
    loop {
        match (left.next(), right.next()) {
            (None, None) => return true,
            (Some(left_val), Some(right_val)) if values_equal(left_val, right_val) => {}
            _ => return false,
        }
    }
}

/// Compares two maps for equality with semantic value comparison.
///
/// Two maps are equal if they have the same keys and all corresponding values
/// are semantically equal according to `values_equal`.
fn maps_equal(left: &Map, right: &Map) -> bool {
    if left.len() != right.len() {
        return false;
    }
    // For each key in left, check it exists in right with semantically equal value
    for (key, left_val) in left.iter() {
        match right.get(key.value()) {
            Some(right_val) if values_equal(left_val, right_val) => {}
            _ => return false,
        }
    }
    true
}

/// Tests if two values are semantically equal.
///
/// Implements Clojure-style equality semantics:
/// - Deep structural equality for collections (elements compared recursively)
/// - Cross-type numeric equality (`1 = 1.0 = 1/1`)
/// - Cross-type sequential equality (lists and vectors with same elements are equal)
/// - NaN is not equal to anything (including itself)
#[expect(
    clippy::float_cmp,
    reason = "[approved] VM equality semantics require exact float comparison"
)]
pub fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        // Primitives
        (&Value::Nil, &Value::Nil) => true,
        (&Value::Bool(left_bool), &Value::Bool(right_bool)) => left_bool == right_bool,
        (&Value::Symbol(ref left_sym), &Value::Symbol(ref right_sym)) => left_sym == right_sym,
        (&Value::Keyword(left_kw), &Value::Keyword(right_kw)) => left_kw == right_kw,
        (&Value::String(ref left_str), &Value::String(ref right_str)) => left_str == right_str,

        // Same-type numeric (NaN != NaN is handled naturally by f64 comparison)
        (&Value::Integer(ref left_int), &Value::Integer(ref right_int)) => left_int == right_int,
        (&Value::Float(left_float), &Value::Float(right_float)) => left_float == right_float,
        (&Value::Ratio(ref left_ratio), &Value::Ratio(ref right_ratio)) => {
            left_ratio == right_ratio
        }

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

        // Sequential equality (recursive + cross-type)
        (&Value::List(ref left_list), &Value::List(ref right_list)) => {
            sequences_equal(left_list.iter(), right_list.iter())
        }
        (&Value::Vector(ref left_vec), &Value::Vector(ref right_vec)) => {
            sequences_equal(left_vec.iter(), right_vec.iter())
        }
        (&Value::List(ref left_list), &Value::Vector(ref right_vec)) => {
            sequences_equal(left_list.iter(), right_vec.iter())
        }
        (&Value::Vector(ref left_vec), &Value::List(ref right_list)) => {
            sequences_equal(left_vec.iter(), right_list.iter())
        }

        // Map equality (recursive value comparison)
        (&Value::Map(ref left_map), &Value::Map(ref right_map)) => maps_equal(left_map, right_map),

        // Set equality (order-independent, semantic value comparison)
        (&Value::Set(ref left_set), &Value::Set(ref right_set)) => sets_equal(left_set, right_set),

        // Function equality (identity-based via PartialEq)
        (&Value::Function(ref left_fn), &Value::Function(ref right_fn)) => left_fn == right_fn,

        // Different types are not equal
        _ => false,
    }
}

/// Compares two sets for equality with semantic value comparison.
///
/// Two sets are equal if they have the same size and all elements in one set
/// are semantically equal to elements in the other set.
fn sets_equal(left: &lona_core::set::Set, right: &lona_core::set::Set) -> bool {
    if left.len() != right.len() {
        return false;
    }
    // For each element in left, check it exists in right
    for item in left.iter() {
        if !right.contains(item.value()) {
            return false;
        }
    }
    true
}
