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

use crate::pvec::PersistentVec;
use crate::value::Value;

/// An immutable, persistent vector.
///
/// Internally uses a 32-way branching trie with tail optimization for efficient
/// operations. Access, update, and push operations are O(log32 n), effectively
/// constant for practical sizes (7 operations for a billion elements).
///
/// Vectors are immutable once created; modification operations return new vectors
/// that share structure with the original.
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
pub struct Vector(PersistentVec<Value>);

impl Vector {
    /// Creates an empty vector.
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self(PersistentVec::new())
    }

    /// Creates a vector from a `Vec<Value>`.
    #[inline]
    #[must_use]
    pub fn from_vec(values: Vec<Value>) -> Self {
        Self(PersistentVec::from_vec(values))
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
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the vector is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a new vector with the element at `index` replaced by `value`.
    ///
    /// Returns `None` if the index is out of bounds.
    /// This operation shares structure with the original vector.
    #[inline]
    #[must_use]
    pub fn assoc(&self, index: usize, value: Value) -> Option<Self> {
        self.0.assoc(index, value).map(Self)
    }

    /// Returns a new vector with the value appended to the end.
    ///
    /// This operation shares structure with the original vector.
    #[inline]
    #[must_use]
    pub fn push(&self, value: Value) -> Self {
        Self(self.0.push(value))
    }

    /// Returns a new vector with the last element removed.
    ///
    /// Returns `None` if the vector is empty.
    /// This operation shares structure with the original vector.
    #[inline]
    #[must_use]
    pub fn pop(&self) -> Option<Self> {
        self.0.pop().map(Self)
    }

    /// Returns an iterator over references to the vector elements.
    #[inline]
    #[must_use]
    pub const fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.0.iter(),
        }
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

    #[test]
    fn pop_empty() {
        let vec = Vector::empty();
        assert!(vec.pop().is_none());
    }

    #[test]
    fn pop_single_element() {
        let vec = Vector::from_vec(alloc::vec![int(42)]);
        let popped = vec.pop().unwrap();

        assert!(popped.is_empty());
        assert_eq!(vec.len(), 1); // Original unchanged
    }

    #[test]
    fn pop_multiple_elements() {
        let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let popped = vec.pop().unwrap();

        assert_eq!(popped.len(), 2);
        assert_eq!(popped.get(0), Some(&int(1)));
        assert_eq!(popped.get(1), Some(&int(2)));
        assert_eq!(vec.len(), 3); // Original unchanged
    }

    #[test]
    fn pop_preserves_original() {
        let v1 = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let v2 = v1.pop().unwrap();

        // v1 unchanged
        assert_eq!(v1.len(), 3);
        assert_eq!(v1.get(2), Some(&int(3)));

        // v2 has one less element
        assert_eq!(v2.len(), 2);
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

        // Both have the same elements
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

    // =========================================================================
    // Structural Sharing Tests
    // =========================================================================

    #[test]
    fn structural_sharing_on_push() {
        let v1 = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let v2 = v1.push(int(4));

        // v1 is unchanged
        assert_eq!(v1.len(), 3);
        assert_eq!(v1.get(0), Some(&int(1)));
        assert_eq!(v1.get(1), Some(&int(2)));
        assert_eq!(v1.get(2), Some(&int(3)));

        // v2 has new element
        assert_eq!(v2.len(), 4);
        assert_eq!(v2.get(3), Some(&int(4)));
    }

    #[test]
    fn structural_sharing_on_assoc() {
        let v1 = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let v2 = v1.assoc(1, int(42)).unwrap();

        // v1 is unchanged
        assert_eq!(v1.get(1), Some(&int(2)));

        // v2 has updated value
        assert_eq!(v2.get(1), Some(&int(42)));

        // Other elements are shared (same value)
        assert_eq!(v1.get(0), v2.get(0));
        assert_eq!(v1.get(2), v2.get(2));
    }

    #[test]
    fn large_vector_structural_sharing() {
        let mut vec = Vector::empty();
        for i in 0..100_i64 {
            vec = vec.push(int(i));
        }

        let original_len = vec.len();
        let modified = vec.assoc(50, int(999)).unwrap();

        // Original unchanged
        assert_eq!(vec.len(), original_len);
        assert_eq!(vec.get(50), Some(&int(50)));

        // Modified has update
        assert_eq!(modified.get(50), Some(&int(999)));
    }

    // =========================================================================
    // Large Vector Tests
    // =========================================================================

    #[test]
    fn large_vector_operations() {
        let mut vec = Vector::empty();
        let count = 1000_i64;

        for i in 0..count {
            vec = vec.push(int(i));
        }

        assert_eq!(vec.len(), count as usize);

        for i in 0..count {
            assert_eq!(vec.get(i as usize), Some(&int(i)));
        }
    }

    // Note: The original test `clone_shares_data` checked Rc::ptr_eq on the internal
    // Rc<[Value]>. With the new PersistentVec implementation, structural sharing
    // happens at the trie node level, not at the top-level container.
    // The test has been replaced with a semantic equivalence test above.
}
