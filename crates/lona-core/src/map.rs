// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Immutable map type for Lonala.
//!
//! Provides an immutable, reference-counted map that enables efficient
//! sharing of map data. Uses `Rc<BTreeMap<ValueKey, Value>>` for heap
//! allocation with reference counting.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use core::cmp::Ordering;
use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};

use crate::list::List;
use crate::value::Value;
use crate::vector::Vector;

/// A wrapper around `Value` that implements `Ord` for use as map keys.
///
/// The ordering is defined as:
/// `Nil < Bool < Integer < Float < Ratio < Symbol < String < List < Vector < Map`
///
/// Within each type, natural ordering is used.
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
            Value::Nil => 0,
            Value::Bool(_) => 1,
            Value::Integer(_) => 2,
            Value::Float(_) => 3,
            Value::Ratio(_) => 4,
            Value::Symbol(_) => 5,
            Value::String(_) => 6,
            Value::List(_) => 7,
            Value::Vector(_) => 8,
            Value::Map(_) => 9,
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
            (&Value::Symbol(left), &Value::Symbol(right)) => left.as_u32().cmp(&right.as_u32()),
            (&Value::String(ref left), &Value::String(ref right)) => left.cmp(right),
            (&Value::List(ref left), &Value::List(ref right)) => compare_lists(left, right),
            (&Value::Vector(ref left), &Value::Vector(ref right)) => compare_vectors(left, right),
            (&Value::Map(ref left), &Value::Map(ref right)) => compare_maps(left, right),
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

/// Total ordering for floats, handling NaN consistently.
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
fn compare_maps(left: &Map, right: &Map) -> Ordering {
    let mut left_iter = left.iter();
    let mut right_iter = right.iter();

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

/// An immutable, reference-counted map.
///
/// Wraps `Rc<BTreeMap<ValueKey, Value>>` to provide efficient cloning through
/// reference counting. Maps are immutable once created; modification operations
/// return new maps.
///
/// # Example
///
/// ```
/// # use lona_core::map::Map;
/// # use lona_core::value::Value;
/// let map = Map::empty()
///     .assoc(Value::from("a"), Value::from(1_i32))
///     .assoc(Value::from("b"), Value::from(2_i32));
/// assert_eq!(map.len(), 2);
/// ```
#[derive(Clone)]
pub struct Map(Rc<BTreeMap<ValueKey, Value>>);

impl Map {
    /// Creates an empty map.
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self(Rc::new(BTreeMap::new()))
    }

    /// Creates a map from an iterator of key-value pairs.
    #[inline]
    #[must_use]
    pub fn from_pairs<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (Value, Value)>,
    {
        let inner: BTreeMap<ValueKey, Value> = pairs
            .into_iter()
            .map(|(key, val)| (ValueKey::new(key), val))
            .collect();
        Self(Rc::new(inner))
    }

    /// Returns a reference to the value associated with the key, if any.
    #[inline]
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.0.get(&ValueKey::new(key.clone()))
    }

    /// Returns the number of entries in the map.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the map is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if the map contains the given key.
    #[inline]
    #[must_use]
    pub fn contains_key(&self, key: &Value) -> bool {
        self.0.contains_key(&ValueKey::new(key.clone()))
    }

    /// Returns a new map with the key-value pair inserted or updated.
    ///
    /// This is a copy-on-write operation that creates a new map.
    #[inline]
    #[must_use]
    pub fn assoc(&self, key: Value, value: Value) -> Self {
        let mut new_map = (*self.0).clone();
        let _old = new_map.insert(ValueKey::new(key), value);
        Self(Rc::new(new_map))
    }

    /// Returns a new map with the key removed.
    ///
    /// This is a copy-on-write operation that creates a new map.
    #[inline]
    #[must_use]
    pub fn dissoc(&self, key: &Value) -> Self {
        let mut new_map = (*self.0).clone();
        let _old = new_map.remove(&ValueKey::new(key.clone()));
        Self(Rc::new(new_map))
    }

    /// Returns an iterator over the keys.
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &Value> {
        self.0.keys().map(ValueKey::value)
    }

    /// Returns an iterator over the values.
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.0.values()
    }

    /// Returns an iterator over key-value pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&ValueKey, &Value)> {
        self.0.iter()
    }
}

impl Default for Map {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl Display for Map {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (key, value) in self.0.iter() {
            if first {
                first = false;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{} {}", key.value(), value)?;
        }
        write!(f, "}}")
    }
}

impl Debug for Map {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Map({{")?;
        let mut first = true;
        for (key, value) in self.0.iter() {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{:?}: {:?}", key.value(), value)?;
        }
        write!(f, "}})")
    }
}

impl PartialEq for Map {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }

        for (key, value) in self.0.iter() {
            match other.0.get(key) {
                Some(other_value) if value == other_value => {}
                _ => return false,
            }
        }
        true
    }
}

impl Eq for Map {}

impl PartialOrd for Map {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Map {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        compare_maps(self, other)
    }
}

impl Hash for Map {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for (key, value) in self.0.iter() {
            key.hash(state);
            value.hash(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integer::Integer;
    use crate::string::HeapStr;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;

    /// Helper to create an integer value.
    fn int(value: i64) -> Value {
        Value::Integer(Integer::from_i64(value))
    }

    /// Helper to create a string value.
    fn string(text: &str) -> Value {
        Value::String(HeapStr::new(text))
    }

    // =========================================================================
    // Construction Tests
    // =========================================================================

    #[test]
    fn empty_map() {
        let map = Map::empty();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn from_pairs() {
        let map = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
        assert!(!map.is_empty());
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn from_pairs_empty() {
        let map = Map::from_pairs(Vec::<(Value, Value)>::new());
        assert!(map.is_empty());
    }

    // =========================================================================
    // Access Tests
    // =========================================================================

    #[test]
    fn get_existing_key() {
        let map = Map::from_pairs(vec![(string("key"), int(42))]);
        assert_eq!(map.get(&string("key")), Some(&int(42)));
    }

    #[test]
    fn get_missing_key() {
        let map = Map::from_pairs(vec![(string("key"), int(42))]);
        assert_eq!(map.get(&string("other")), None);
    }

    #[test]
    fn get_empty() {
        let map = Map::empty();
        assert_eq!(map.get(&string("key")), None);
    }

    #[test]
    fn contains_key() {
        let map = Map::from_pairs(vec![(string("a"), int(1))]);
        assert!(map.contains_key(&string("a")));
        assert!(!map.contains_key(&string("b")));
    }

    // =========================================================================
    // Mutation Tests (Copy-on-Write)
    // =========================================================================

    #[test]
    fn assoc_new_key() {
        let map = Map::empty();
        let new_map = map.assoc(string("key"), int(42));

        assert!(map.is_empty());
        assert_eq!(new_map.len(), 1);
        assert_eq!(new_map.get(&string("key")), Some(&int(42)));
    }

    #[test]
    fn assoc_existing_key() {
        let map = Map::from_pairs(vec![(string("key"), int(1))]);
        let new_map = map.assoc(string("key"), int(2));

        // Original unchanged
        assert_eq!(map.get(&string("key")), Some(&int(1)));
        // New map has updated value
        assert_eq!(new_map.get(&string("key")), Some(&int(2)));
    }

    #[test]
    fn dissoc_existing_key() {
        let map = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
        let new_map = map.dissoc(&string("a"));

        // Original unchanged
        assert_eq!(map.len(), 2);
        // New map has key removed
        assert_eq!(new_map.len(), 1);
        assert!(!new_map.contains_key(&string("a")));
        assert!(new_map.contains_key(&string("b")));
    }

    #[test]
    fn dissoc_missing_key() {
        let map = Map::from_pairs(vec![(string("a"), int(1))]);
        let new_map = map.dissoc(&string("missing"));

        // Same length, nothing removed
        assert_eq!(new_map.len(), 1);
    }

    // =========================================================================
    // Iterator Tests
    // =========================================================================

    #[test]
    fn keys_iterator() {
        let map = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
        let keys: Vec<_> = map.keys().cloned().collect();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn values_iterator() {
        let map = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
        let values: Vec<_> = map.values().cloned().collect();
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn iter() {
        let map = Map::from_pairs(vec![(string("a"), int(1))]);
        let entries: Vec<_> = map.iter().collect();
        assert_eq!(entries.len(), 1);
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn display_empty() {
        let map = Map::empty();
        assert_eq!(map.to_string(), "{}");
    }

    #[test]
    fn display_single() {
        let map = Map::from_pairs(vec![(int(1), int(2))]);
        assert_eq!(map.to_string(), "{1 2}");
    }

    // =========================================================================
    // Equality Tests
    // =========================================================================

    #[test]
    fn equality_empty() {
        let m1 = Map::empty();
        let m2 = Map::empty();
        assert_eq!(m1, m2);
    }

    #[test]
    fn equality_same_entries() {
        let m1 = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
        let m2 = Map::from_pairs(vec![(string("b"), int(2)), (string("a"), int(1))]);
        assert_eq!(m1, m2);
    }

    #[test]
    fn equality_different_values() {
        let m1 = Map::from_pairs(vec![(string("a"), int(1))]);
        let m2 = Map::from_pairs(vec![(string("a"), int(2))]);
        assert_ne!(m1, m2);
    }

    #[test]
    fn equality_different_keys() {
        let m1 = Map::from_pairs(vec![(string("a"), int(1))]);
        let m2 = Map::from_pairs(vec![(string("b"), int(1))]);
        assert_ne!(m1, m2);
    }

    #[test]
    fn equality_different_sizes() {
        let m1 = Map::from_pairs(vec![(string("a"), int(1))]);
        let m2 = Map::from_pairs(vec![(string("a"), int(1)), (string("b"), int(2))]);
        assert_ne!(m1, m2);
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[test]
    fn clone_shares_data() {
        let m1 = Map::from_pairs(vec![(string("a"), int(1))]);
        let m2 = m1.clone();

        // Both point to the same data (Rc)
        assert!(Rc::ptr_eq(&m1.0, &m2.0));
        assert_eq!(m1, m2);
    }

    // =========================================================================
    // Default Test
    // =========================================================================

    #[test]
    fn default_is_empty() {
        let map: Map = Map::default();
        assert!(map.is_empty());
    }

    // =========================================================================
    // ValueKey Ordering Tests
    // =========================================================================

    #[test]
    fn value_key_nil_ordering() {
        let nil1 = ValueKey::new(Value::Nil);
        let nil2 = ValueKey::new(Value::Nil);
        assert_eq!(nil1.cmp(&nil2), Ordering::Equal);
    }

    #[test]
    fn value_key_bool_ordering() {
        let false_key = ValueKey::new(Value::Bool(false));
        let true_key = ValueKey::new(Value::Bool(true));
        assert_eq!(false_key.cmp(&true_key), Ordering::Less);
    }

    #[test]
    fn value_key_integer_ordering() {
        let k1 = ValueKey::new(int(1));
        let k2 = ValueKey::new(int(2));
        assert_eq!(k1.cmp(&k2), Ordering::Less);
    }

    #[test]
    fn value_key_cross_type_ordering() {
        let nil_key = ValueKey::new(Value::Nil);
        let bool_key = ValueKey::new(Value::Bool(true));
        let int_key = ValueKey::new(int(1));
        let string_key = ValueKey::new(string("a"));

        // Nil < Bool < Integer < ... < String
        assert_eq!(nil_key.cmp(&bool_key), Ordering::Less);
        assert_eq!(bool_key.cmp(&int_key), Ordering::Less);
        assert_eq!(int_key.cmp(&string_key), Ordering::Less);
    }

    #[test]
    fn value_key_float_nan_ordering() {
        let nan = ValueKey::new(Value::Float(f64::NAN));
        let number = ValueKey::new(Value::Float(1.0));

        // NaN should be greater than any number for consistent ordering
        assert_eq!(nan.cmp(&number), Ordering::Greater);
        assert_eq!(number.cmp(&nan), Ordering::Less);
    }

    // =========================================================================
    // Integer and Symbol Key Tests
    // =========================================================================

    #[test]
    fn integer_keys() {
        let map = Map::from_pairs(vec![(int(1), string("one")), (int(2), string("two"))]);
        assert_eq!(map.get(&int(1)), Some(&string("one")));
        assert_eq!(map.get(&int(2)), Some(&string("two")));
    }

    #[test]
    fn mixed_type_keys() {
        let map = Map::from_pairs(vec![
            (Value::Nil, int(0)),
            (Value::Bool(true), int(1)),
            (int(42), int(2)),
        ]);
        assert_eq!(map.get(&Value::Nil), Some(&int(0)));
        assert_eq!(map.get(&Value::Bool(true)), Some(&int(1)));
        assert_eq!(map.get(&int(42)), Some(&int(2)));
    }

    // =========================================================================
    // Nested Map Tests
    // =========================================================================

    #[test]
    fn nested_maps() {
        let inner = Map::from_pairs(vec![(string("x"), int(1))]);
        let outer = Map::from_pairs(vec![(string("inner"), Value::Map(inner.clone()))]);

        if let Some(Value::Map(inner_map)) = outer.get(&string("inner")) {
            assert_eq!(inner_map.get(&string("x")), Some(&int(1)));
        } else {
            panic!("Expected Map value");
        }
    }
}
