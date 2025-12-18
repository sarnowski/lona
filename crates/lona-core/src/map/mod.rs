// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Immutable map type for Lonala.
//!
//! Provides an immutable, persistent map that enables efficient structural
//! sharing between versions. Uses a Hash Array Mapped Trie (HAMT) internally
//! for O(log32 n) access and update operations, which is effectively constant
//! for practical collection sizes.
//!
//! # Structural Sharing
//!
//! When a map is modified (via [`Map::assoc`] or [`Map::dissoc`]), only the
//! nodes along the path to the modification are copied. All other nodes are
//! shared between the old and new maps, making operations memory-efficient.

use core::cmp::Ordering;
use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};

use crate::fnv::FnvHasher;
use crate::hamt::Hamt;
use crate::symbol::Interner;
use crate::value::Value;

mod value_key;

#[cfg(test)]
mod tests;

pub use value_key::ValueKey;
use value_key::compare_maps;

/// An immutable, persistent map.
///
/// Internally uses a Hash Array Mapped Trie (HAMT) for efficient operations.
/// Lookup, insert, and remove operations are O(log32 n), effectively constant
/// for practical sizes.
///
/// Maps are immutable once created; modification operations return new maps
/// that share structure with the original.
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
pub struct Map(Hamt<ValueKey, Value>);

impl Map {
    /// Creates an empty map.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self(Hamt::new())
    }

    /// Creates a map from an iterator of key-value pairs.
    #[inline]
    #[must_use]
    pub fn from_pairs<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (Value, Value)>,
    {
        let mut hamt = Hamt::new();
        for (key, val) in pairs {
            hamt = hamt.insert(ValueKey::new(key), val);
        }
        Self(hamt)
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
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the map is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
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
    /// This operation shares structure with the original map.
    #[inline]
    #[must_use]
    pub fn assoc(&self, key: Value, value: Value) -> Self {
        Self(self.0.insert(ValueKey::new(key), value))
    }

    /// Returns a new map with the key removed.
    ///
    /// This operation shares structure with the original map.
    #[inline]
    #[must_use]
    pub fn dissoc(&self, key: &Value) -> Self {
        Self(self.0.remove(&ValueKey::new(key.clone())))
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
    ///
    /// Note: Iteration order is based on hash values, not sorted order.
    /// For sorted iteration, collect and sort the results.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&ValueKey, &Value)> {
        self.0.iter()
    }

    /// Creates a wrapper for displaying this map with symbol resolution.
    #[inline]
    #[must_use]
    pub const fn display<'interner>(
        &'interner self,
        interner: &'interner Interner,
    ) -> Displayable<'interner> {
        Displayable {
            map: self,
            interner,
        }
    }
}

/// A wrapper for displaying a [`Map`] with symbol name resolution.
///
/// Created via [`Map::display`].
pub struct Displayable<'interner> {
    map: &'interner Map,
    interner: &'interner Interner,
}

impl Display for Displayable<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (key, value) in self.map.iter() {
            if first {
                first = false;
            } else {
                write!(f, " ")?;
            }
            write!(
                f,
                "{} {}",
                key.value().display(self.interner),
                value.display(self.interner)
            )?;
        }
        write!(f, "}}")
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
        for (key, value) in self.iter() {
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
        for (key, value) in self.iter() {
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

        for (key, value) in self.iter() {
            match other.get(key.value()) {
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
        // For consistent hashing regardless of iteration order,
        // we XOR together the hashes of all entries
        let mut combined: u64 = 0;
        for (key, value) in self.iter() {
            let mut entry_hasher = FnvHasher::default();
            key.hash(&mut entry_hasher);
            value.hash(&mut entry_hasher);
            combined ^= entry_hasher.finish();
        }
        combined.hash(state);
    }
}
