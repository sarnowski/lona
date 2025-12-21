// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! `ValueKey` wrapper for map keys and comparison helpers.
//!
//! Provides an orderable wrapper around `Value` for use as map keys, with
//! consistent ordering for all value types including floats with NaN.

use alloc::vec::Vec;

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use crate::list::List;
use crate::value::Value;
use crate::vector::Vector;

use super::Map;

/// A wrapper around `Value` that implements `Ord` for use as map keys.
///
/// The ordering is defined as:
/// `Nil < Bool < Integer < Float < Ratio < Symbol < Keyword < String < List < Vector < Map < Set < Binary < Function < NativeFunction`
///
/// Within each type, natural ordering is used. Note that Binary should not
/// be used as a map key (its Hash impl panics) since it's a mutable type.
#[derive(Clone, Debug)]
pub struct ValueKey(Value);

impl ValueKey {
    /// Creates a new `ValueKey` from a `Value`.
    #[inline]
    #[must_use]
    pub const fn new(value: Value) -> Self {
        Self(value)
    }

    /// Returns a reference to the wrapped value.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.0
    }

    /// Unwraps the `ValueKey` into its inner `Value`.
    #[inline]
    #[must_use]
    pub fn into_value(self) -> Value {
        self.0
    }

    /// Returns the type discriminant for ordering.
    const fn type_order(&self) -> u8 {
        match self.0 {
            Value::Nil => 0_u8,
            Value::Bool(_) => 1_u8,
            Value::Integer(_) => 2_u8,
            Value::Float(_) => 3_u8,
            Value::Ratio(_) => 4_u8,
            Value::Symbol(_) => 5_u8,
            Value::Keyword(_) => 6_u8,
            Value::String(_) => 7_u8,
            Value::List(_) => 8_u8,
            Value::Vector(_) => 9_u8,
            Value::Map(_) => 10_u8,
            Value::Set(_) => 11_u8,
            Value::Binary(_) => 12_u8,
            Value::Function(_) => 13_u8,
            Value::NativeFunction(_) => 14_u8,
        }
    }
}

impl PartialEq for ValueKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ValueKey {}

impl PartialOrd for ValueKey {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ValueKey {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by type
        let type_cmp = self.type_order().cmp(&other.type_order());
        if type_cmp != Ordering::Equal {
            return type_cmp;
        }

        // Same type, compare values
        match (&self.0, &other.0) {
            (&Value::Bool(ref left), &Value::Bool(ref right)) => left.cmp(right),
            (&Value::Integer(ref left), &Value::Integer(ref right)) => left.cmp(right),
            (&Value::Float(left), &Value::Float(right)) => {
                // Use total ordering for floats to handle NaN consistently
                float_total_order(left, right)
            }
            (&Value::Ratio(ref left), &Value::Ratio(ref right)) => left.cmp(right),
            (&Value::Symbol(left), &Value::Symbol(right))
            | (&Value::Keyword(left), &Value::Keyword(right)) => left.as_u32().cmp(&right.as_u32()),
            (&Value::NativeFunction(left), &Value::NativeFunction(right)) => {
                left.as_u32().cmp(&right.as_u32())
            }
            (&Value::String(ref left), &Value::String(ref right)) => left.cmp(right),
            (&Value::List(ref left), &Value::List(ref right)) => compare_lists(left, right),
            (&Value::Vector(ref left), &Value::Vector(ref right)) => compare_vectors(left, right),
            (&Value::Map(ref left), &Value::Map(ref right)) => compare_maps(left, right),
            (&Value::Set(ref left), &Value::Set(ref right)) => {
                crate::set::compare_sets(left, right)
            }
            // Nil and any other same-type comparisons
            _ => Ordering::Equal,
        }
    }
}

impl Hash for ValueKey {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl From<Value> for ValueKey {
    #[inline]
    fn from(value: Value) -> Self {
        Self::new(value)
    }
}

/// Provides total ordering for floats, handling NaN consistently.
///
/// NaN values are placed at the end of the ordering (greater than all
/// non-NaN values) to ensure consistent, reflexive ordering even for
/// special float values.
fn float_total_order(left: f64, right: f64) -> Ordering {
    // Place NaN at the end for consistent ordering
    match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
    }
}

/// Compares two lists lexicographically.
///
/// Lists are compared element by element. The first differing element
/// determines the ordering. If one list is a prefix of the other, the
/// shorter list is considered less.
fn compare_lists(left: &List, right: &List) -> Ordering {
    let mut left_iter = left.iter();
    let mut right_iter = right.iter();

    loop {
        match (left_iter.next(), right_iter.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_val), Some(right_val)) => {
                let cmp = ValueKey::new(left_val.clone()).cmp(&ValueKey::new(right_val.clone()));
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
        }
    }
}

/// Compares two vectors lexicographically.
///
/// Vectors are compared element by element. The first differing element
/// determines the ordering. If one vector is a prefix of the other, the
/// shorter vector is considered less.
fn compare_vectors(left: &Vector, right: &Vector) -> Ordering {
    let mut left_iter = left.iter();
    let mut right_iter = right.iter();

    loop {
        match (left_iter.next(), right_iter.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_val), Some(right_val)) => {
                let cmp = ValueKey::new(left_val.clone()).cmp(&ValueKey::new(right_val.clone()));
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
        }
    }
}

/// Compares two maps by comparing their sorted entries.
///
/// Maps are first compared by their sorted keys. For equal keys,
/// the corresponding values are compared. This ensures a consistent
/// total ordering regardless of internal iteration order.
pub(super) fn compare_maps(left: &Map, right: &Map) -> Ordering {
    // Collect and sort entries for comparison
    let mut left_entries: Vec<_> = left.iter().collect();
    let mut right_entries: Vec<_> = right.iter().collect();
    left_entries.sort_by(|&(ref k1, _), &(ref k2, _)| k1.cmp(k2));
    right_entries.sort_by(|&(ref k1, _), &(ref k2, _)| k1.cmp(k2));

    let mut left_iter = left_entries.into_iter();
    let mut right_iter = right_entries.into_iter();

    loop {
        match (left_iter.next(), right_iter.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some((left_key, left_val)), Some((right_key, right_val))) => {
                // Compare keys first
                let key_cmp = left_key.cmp(right_key);
                if key_cmp != Ordering::Equal {
                    return key_cmp;
                }
                // Keys are equal, compare values
                let val_cmp =
                    ValueKey::new(left_val.clone()).cmp(&ValueKey::new(right_val.clone()));
                if val_cmp != Ordering::Equal {
                    return val_cmp;
                }
            }
        }
    }
}
