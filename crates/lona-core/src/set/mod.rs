// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Immutable set type for Lonala.
//!
//! Provides an immutable, persistent set that enables efficient structural
//! sharing between versions. Uses a Hash Array Mapped Trie (HAMT) internally
//! for O(log32 n) access and update operations, which is effectively constant
//! for practical collection sizes.
//!
//! # Structural Sharing
//!
//! When a set is modified (via [`Set::insert`] or [`Set::remove`]), only the
//! nodes along the path to the modification are copied. All other nodes are
//! shared between the old and new sets, making operations memory-efficient.

use core::cmp::Ordering;
use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};

use alloc::vec::Vec;

use crate::fnv::FnvHasher;
use crate::hamt::Hamt;
use crate::map::{Map, ValueKey};
use crate::meta::Meta;
use crate::symbol::Interner;
use crate::value::Value;

#[cfg(test)]
mod tests;

/// An immutable, persistent set with optional metadata.
///
/// Internally uses a Hash Array Mapped Trie (HAMT) for efficient operations.
/// Lookup, insert, and remove operations are O(log32 n), effectively constant
/// for practical sizes.
///
/// Sets are immutable once created; modification operations return new sets
/// that share structure with the original.
///
/// Metadata does not affect equality or hashing, following Clojure semantics.
///
/// # Example
///
/// ```
/// # use lona_core::set::Set;
/// # use lona_core::value::Value;
/// let set = Set::empty()
///     .insert(Value::from(1_i32))
///     .insert(Value::from(2_i32));
/// assert_eq!(set.len(), 2);
/// ```
#[derive(Clone)]
pub struct Set {
    /// The underlying HAMT structure.
    inner: Hamt<ValueKey, ()>,
    /// Optional metadata map.
    meta: Option<Map>,
}

impl Set {
    /// Creates an empty set.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            inner: Hamt::new(),
            meta: None,
        }
    }

    /// Creates a set from an iterator of values.
    #[inline]
    #[must_use]
    pub fn from_values<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
    {
        let mut hamt = Hamt::new();
        for value in values {
            hamt = hamt.insert(ValueKey::new(value), ());
        }
        Self {
            inner: hamt,
            meta: None,
        }
    }

    /// Returns the number of elements in the set.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the set is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if the set contains the given value.
    #[inline]
    #[must_use]
    pub fn contains(&self, value: &Value) -> bool {
        self.inner.contains_key(&ValueKey::new(value.clone()))
    }

    /// Returns a new set with the value inserted.
    ///
    /// If the value is already present, returns a set equal to the original.
    /// This operation shares structure with the original set.
    /// Metadata is preserved.
    #[inline]
    #[must_use]
    pub fn insert(&self, value: Value) -> Self {
        Self {
            inner: self.inner.insert(ValueKey::new(value), ()),
            meta: self.meta.clone(),
        }
    }

    /// Returns a new set with the value removed.
    ///
    /// If the value is not present, returns a set equal to the original.
    /// This operation shares structure with the original set.
    /// Metadata is preserved.
    #[inline]
    #[must_use]
    pub fn remove(&self, value: &Value) -> Self {
        Self {
            inner: self.inner.remove(&ValueKey::new(value.clone())),
            meta: self.meta.clone(),
        }
    }

    /// Returns an iterator over the elements.
    ///
    /// Note: Iteration order is based on hash values, not sorted order.
    /// For sorted iteration, collect and sort the results.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &ValueKey> {
        self.inner.keys()
    }

    /// Creates a wrapper for displaying this set with symbol resolution.
    #[inline]
    #[must_use]
    pub const fn display<'interner>(
        &'interner self,
        interner: &'interner Interner,
    ) -> Displayable<'interner> {
        Displayable {
            set: self,
            interner,
        }
    }
}

impl Meta for Set {
    #[inline]
    fn meta(&self) -> Option<&Map> {
        self.meta.as_ref()
    }

    #[inline]
    fn with_meta(self, meta: Option<Map>) -> Self {
        Self {
            inner: self.inner,
            meta,
        }
    }
}

/// A wrapper for displaying a [`Set`] with symbol name resolution.
///
/// Created via [`Set::display`].
pub struct Displayable<'interner> {
    set: &'interner Set,
    interner: &'interner Interner,
}

impl Display for Displayable<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{{")?;
        let mut first = true;
        for key in self.set.iter() {
            if first {
                first = false;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{}", key.value().display(self.interner))?;
        }
        write!(f, "}}")
    }
}

impl Default for Set {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl Display for Set {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{{")?;
        let mut first = true;
        for key in self.iter() {
            if first {
                first = false;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{}", key.value())?;
        }
        write!(f, "}}")
    }
}

impl Debug for Set {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Set(#{{")?;
        let mut first = true;
        for key in self.iter() {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", key.value())?;
        }
        write!(f, "}})")
    }
}

/// Equality ignores metadata.
impl PartialEq for Set {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.inner.len() != other.inner.len() {
            return false;
        }

        for key in self.iter() {
            if !other.contains(key.value()) {
                return false;
            }
        }
        true
    }
}

impl Eq for Set {}

impl PartialOrd for Set {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Set {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        compare_sets(self, other)
    }
}

/// Hash ignores metadata.
impl Hash for Set {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.len().hash(state);
        // For consistent hashing regardless of iteration order,
        // we XOR together the hashes of all elements
        let mut combined: u64 = 0;
        for key in self.iter() {
            let mut entry_hasher = FnvHasher::default();
            key.hash(&mut entry_hasher);
            combined ^= entry_hasher.finish();
        }
        combined.hash(state);
    }
}

/// Compares two sets by comparing their sorted elements.
///
/// Sets are first compared by size. For equal sizes, elements are
/// sorted and compared lexicographically. This ensures a consistent
/// total ordering regardless of internal iteration order.
pub(crate) fn compare_sets(left: &Set, right: &Set) -> Ordering {
    // First compare by size
    let size_cmp = left.len().cmp(&right.len());
    if size_cmp != Ordering::Equal {
        return size_cmp;
    }

    // Same size, compare sorted elements
    let mut left_elements: Vec<_> = left.iter().collect();
    let mut right_elements: Vec<_> = right.iter().collect();
    left_elements.sort();
    right_elements.sort();

    let mut left_iter = left_elements.into_iter();
    let mut right_iter = right_elements.into_iter();

    loop {
        match (left_iter.next(), right_iter.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_key), Some(right_key)) => {
                let cmp = left_key.cmp(right_key);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
        }
    }
}
