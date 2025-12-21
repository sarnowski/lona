// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Immutable vector type for Lonala.
//!
//! Provides an immutable, persistent vector that enables efficient structural
//! sharing between versions. Uses a 32-way branching trie internally for
//! O(log32 n) access and update operations, which is effectively constant
//! for practical collection sizes.
//!
//! # Structural Sharing
//!
//! When a vector is modified (via [`Vector::assoc`] or [`Vector::push`]),
//! only the nodes along the path to the modification are copied. All other
//! nodes are shared between the old and new vectors, making operations
//! memory-efficient.

use alloc::vec::Vec;

use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};

use crate::map::Map;
use crate::meta::Meta;
use crate::pvec::PersistentVec;
use crate::symbol::Interner;
use crate::value::Value;

/// An immutable, persistent vector with optional metadata.
///
/// Internally uses a 32-way branching trie with tail optimization for efficient
/// operations. Access, update, and push operations are O(log32 n), effectively
/// constant for practical sizes (7 operations for a billion elements).
///
/// Vectors are immutable once created; modification operations return new vectors
/// that share structure with the original.
///
/// Metadata does not affect equality or hashing, following Clojure semantics.
///
/// # Example
///
/// ```
/// # use lona_core::vector::Vector;
/// # use lona_core::value::Value;
/// let vec = Vector::from_vec(vec![Value::from(1_i32), Value::from(2_i32)]);
/// let extended = vec.push(Value::from(3_i32));
/// // Original vec is unchanged
/// assert_eq!(vec.len(), 2);
/// assert_eq!(extended.len(), 3);
/// ```
#[derive(Clone)]
pub struct Vector {
    /// The underlying persistent vector.
    inner: PersistentVec<Value>,
    /// Optional metadata map.
    meta: Option<Map>,
}

impl Vector {
    /// Creates an empty vector.
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self {
            inner: PersistentVec::new(),
            meta: None,
        }
    }

    /// Creates a vector from a `Vec<Value>`.
    #[inline]
    #[must_use]
    pub fn from_vec(values: Vec<Value>) -> Self {
        Self {
            inner: PersistentVec::from_vec(values),
            meta: None,
        }
    }

    /// Returns a reference to the element at the given index, if it exists.
    #[inline]
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.inner.get(index)
    }

    /// Returns the number of elements in the vector.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the vector is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns a new vector with the element at `index` replaced by `value`.
    ///
    /// Returns `None` if the index is out of bounds.
    /// This operation shares structure with the original vector.
    /// Metadata is preserved.
    #[inline]
    #[must_use]
    pub fn assoc(&self, index: usize, value: Value) -> Option<Self> {
        self.inner.assoc(index, value).map(|inner| Self {
            inner,
            meta: self.meta.clone(),
        })
    }

    /// Returns a new vector with the value appended to the end.
    ///
    /// This operation shares structure with the original vector.
    /// Metadata is preserved.
    #[inline]
    #[must_use]
    pub fn push(&self, value: Value) -> Self {
        Self {
            inner: self.inner.push(value),
            meta: self.meta.clone(),
        }
    }

    /// Returns a new vector with the last element removed.
    ///
    /// Returns `None` if the vector is empty.
    /// This operation shares structure with the original vector.
    /// Metadata is preserved.
    #[inline]
    #[must_use]
    pub fn pop(&self) -> Option<Self> {
        self.inner.pop().map(|inner| Self {
            inner,
            meta: self.meta.clone(),
        })
    }

    /// Returns an iterator over references to the vector elements.
    #[inline]
    #[must_use]
    pub const fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.inner.iter(),
        }
    }

    /// Creates a wrapper for displaying this vector with symbol resolution.
    #[inline]
    #[must_use]
    pub const fn display<'interner>(
        &'interner self,
        interner: &'interner Interner,
    ) -> Displayable<'interner> {
        Displayable {
            vector: self,
            interner,
        }
    }
}

impl Meta for Vector {
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

/// A wrapper for displaying a [`Vector`] with symbol name resolution.
///
/// Created via [`Vector::display`].
pub struct Displayable<'interner> {
    vector: &'interner Vector,
    interner: &'interner Interner,
}

impl Display for Displayable<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut first = true;
        for value in self.vector {
            if first {
                first = false;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{}", value.display(self.interner))?;
        }
        write!(f, "]")
    }
}

impl Default for Vector {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl Display for Vector {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut first = true;
        for value in self {
            if first {
                first = false;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{value}")?;
        }
        write!(f, "]")
    }
}

impl Debug for Vector {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vector(")?;
        let mut first = true;
        for value in self {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{value:?}")?;
        }
        write!(f, ")")
    }
}

/// Equality ignores metadata.
impl PartialEq for Vector {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter()
            .zip(other.iter())
            .all(|(left, right)| left == right)
    }
}

impl Eq for Vector {}

/// Hash ignores metadata.
impl Hash for Vector {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        for value in self {
            value.hash(state);
        }
    }
}

/// Iterator over references to vector elements.
pub struct Iter<'vec> {
    inner: crate::pvec::Iter<'vec, Value>,
}

impl<'vec> Iterator for Iter<'vec> {
    type Item = &'vec Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for Iter<'_> {}
impl core::iter::FusedIterator for Iter<'_> {}

impl<'vec> IntoIterator for &'vec Vector {
    type Item = &'vec Value;
    type IntoIter = Iter<'vec>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl From<Vec<Value>> for Vector {
    #[inline]
    fn from(values: Vec<Value>) -> Self {
        Self::from_vec(values)
    }
}

#[cfg(test)]
#[path = "vector_tests.rs"]
mod tests;
