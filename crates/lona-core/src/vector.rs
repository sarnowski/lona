// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Immutable vector type for Lonala.
//!
//! Provides an immutable, reference-counted vector that enables efficient
//! sharing of vector data. Uses `Rc<Vec<Value>>` for heap allocation with
//! reference counting.

use alloc::rc::Rc;
use alloc::vec::Vec;

use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};
use core::slice;

use crate::value::Value;

/// An immutable, reference-counted vector.
///
/// Wraps `Rc<[Value]>` to provide efficient cloning through reference counting.
/// Vectors are immutable once created; modification operations return new vectors.
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
pub struct Vector(Rc<[Value]>);

impl Vector {
    /// Creates an empty vector.
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self(Rc::from([]))
    }

    /// Creates a vector from a `Vec<Value>`.
    #[inline]
    #[must_use]
    pub fn from_vec(values: Vec<Value>) -> Self {
        Self(Rc::from(values))
    }

    /// Returns a reference to the element at the given index, if it exists.
    #[inline]
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.0.get(index)
    }

    /// Returns the number of elements in the vector.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the vector is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a new vector with the element at `index` replaced by `value`.
    ///
    /// Returns `None` if the index is out of bounds.
    /// This is a copy-on-write operation that creates a new vector.
    #[inline]
    #[must_use]
    pub fn assoc(&self, index: usize, value: Value) -> Option<Self> {
        if index >= self.0.len() {
            return None;
        }

        let mut new_vec: Vec<Value> = self.0.iter().cloned().collect();
        if let Some(slot) = new_vec.get_mut(index) {
            *slot = value;
        }
        Some(Self(Rc::from(new_vec)))
    }

    /// Returns a new vector with the value appended to the end.
    ///
    /// This is a copy-on-write operation that creates a new vector.
    #[inline]
    #[must_use]
    pub fn push(&self, value: Value) -> Self {
        let mut new_vec: Vec<Value> = self.0.iter().cloned().collect();
        new_vec.push(value);
        Self(Rc::from(new_vec))
    }

    /// Returns an iterator over references to the vector elements.
    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, Value> {
        self.0.iter()
    }

    /// Returns a reference to the underlying slice.
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[Value] {
        &self.0
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
        for value in self.0.iter() {
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
        for value in self.0.iter() {
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

impl PartialEq for Vector {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Vector {}

impl Hash for Vector {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for value in self.0.iter() {
            value.hash(state);
        }
    }
}

impl<'vec> IntoIterator for &'vec Vector {
    type Item = &'vec Value;
    type IntoIter = slice::Iter<'vec, Value>;

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
mod tests {
    use super::*;
    use crate::integer::Integer;
    use alloc::string::ToString;

    /// Helper to create an integer value.
    fn int(value: i64) -> Value {
        Value::Integer(Integer::from_i64(value))
    }

    // =========================================================================
    // Construction Tests
    // =========================================================================

    #[test]
    fn empty_vector() {
        let vec = Vector::empty();
        assert!(vec.is_empty());
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn from_vec() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        assert!(!vec.is_empty());
        assert_eq!(vec.len(), 3);
    }

    #[test]
    fn from_impl() {
        let vec: Vector = alloc::vec![int(1), int(2)].into();
        assert_eq!(vec.len(), 2);
    }

    // =========================================================================
    // Access Tests
    // =========================================================================

    #[test]
    fn get_valid_index() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        assert_eq!(vec.get(0), Some(&int(1)));
        assert_eq!(vec.get(1), Some(&int(2)));
        assert_eq!(vec.get(2), Some(&int(3)));
    }

    #[test]
    fn get_invalid_index() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2)]);
        assert_eq!(vec.get(5), None);
    }

    #[test]
    fn get_empty() {
        let vec = Vector::empty();
        assert_eq!(vec.get(0), None);
    }

    #[test]
    fn as_slice() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let slice = vec.as_slice();
        assert_eq!(slice.len(), 2);
        assert_eq!(slice.get(0), Some(&int(1)));
    }

    // =========================================================================
    // Mutation Tests (Copy-on-Write)
    // =========================================================================

    #[test]
    fn assoc_valid_index() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let new_vec = vec.assoc(1, int(42)).unwrap();

        // Original unchanged
        assert_eq!(vec.get(1), Some(&int(2)));
        // New vector has updated value
        assert_eq!(new_vec.get(1), Some(&int(42)));
        // Length same
        assert_eq!(new_vec.len(), 3);
    }

    #[test]
    fn assoc_invalid_index() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let result = vec.assoc(10, int(42));
        assert!(result.is_none());
    }

    #[test]
    fn push_element() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let new_vec = vec.push(int(3));

        // Original unchanged
        assert_eq!(vec.len(), 2);
        // New vector has new element
        assert_eq!(new_vec.len(), 3);
        assert_eq!(new_vec.get(2), Some(&int(3)));
    }

    #[test]
    fn push_to_empty() {
        let vec = Vector::empty();
        let new_vec = vec.push(int(1));

        assert!(vec.is_empty());
        assert_eq!(new_vec.len(), 1);
        assert_eq!(new_vec.get(0), Some(&int(1)));
    }

    // =========================================================================
    // Iterator Tests
    // =========================================================================

    #[test]
    fn iter_empty() {
        let vec = Vector::empty();
        let collected: Vec<_> = vec.iter().collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn iter_elements() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let collected: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(collected, alloc::vec![int(1), int(2), int(3)]);
    }

    #[test]
    fn into_iterator() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let mut count = 0_usize;
        for _val in &vec {
            count = count.saturating_add(1);
        }
        assert_eq!(count, 2);
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn display_empty() {
        let vec = Vector::empty();
        assert_eq!(vec.to_string(), "[]");
    }

    #[test]
    fn display_single() {
        let vec = Vector::from_vec(alloc::vec![int(42)]);
        assert_eq!(vec.to_string(), "[42]");
    }

    #[test]
    fn display_multiple() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        assert_eq!(vec.to_string(), "[1 2 3]");
    }

    // =========================================================================
    // Equality Tests
    // =========================================================================

    #[test]
    fn equality_empty() {
        let v1 = Vector::empty();
        let v2 = Vector::empty();
        assert_eq!(v1, v2);
    }

    #[test]
    fn equality_same_elements() {
        let v1 = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let v2 = Vector::from_vec(alloc::vec![int(1), int(2)]);
        assert_eq!(v1, v2);
    }

    #[test]
    fn equality_different_elements() {
        let v1 = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let v2 = Vector::from_vec(alloc::vec![int(1), int(3)]);
        assert_ne!(v1, v2);
    }

    #[test]
    fn equality_different_lengths() {
        let v1 = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let v2 = Vector::from_vec(alloc::vec![int(1)]);
        assert_ne!(v1, v2);
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[test]
    fn clone_shares_data() {
        let v1 = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let v2 = v1.clone();

        // Both point to the same data (Rc)
        assert!(Rc::ptr_eq(&v1.0, &v2.0));
        assert_eq!(v1, v2);
    }

    // =========================================================================
    // Default Test
    // =========================================================================

    #[test]
    fn default_is_empty() {
        let vec: Vector = Vector::default();
        assert!(vec.is_empty());
    }

    // =========================================================================
    // Nested Vector Tests
    // =========================================================================

    #[test]
    fn nested_vectors() {
        let inner = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let outer = Vector::from_vec(alloc::vec![Value::Vector(inner.clone())]);
        assert_eq!(outer.len(), 1);

        if let Some(Value::Vector(inner_vec)) = outer.get(0) {
            assert_eq!(inner_vec.len(), 2);
        } else {
            panic!("Expected Vector value");
        }
    }
}
