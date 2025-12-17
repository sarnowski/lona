// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Cons-cell linked list for Lonala.
//!
//! Provides a persistent, immutable linked list with structural sharing.
//! Prepending elements is O(1), and lists can share tails efficiently.

use alloc::rc::Rc;
use alloc::vec::Vec;

use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;

use crate::symbol::Interner;
use crate::value::Value;

/// A cons cell containing a head value and tail list.
struct ConsCell {
    /// The first element of this cons cell.
    head: Value,
    /// The rest of the list.
    tail: List,
}

/// A persistent, immutable linked list.
///
/// Lists are built from cons cells, with structural sharing for efficiency.
/// Operations that prepend elements are O(1), while accessing the tail
/// is also O(1) through reference counting.
///
/// # Example
///
/// ```
/// # use lona_core::list::List;
/// # use lona_core::value::Value;
/// let list = List::empty()
///     .cons(Value::from(3_i32))
///     .cons(Value::from(2_i32))
///     .cons(Value::from(1_i32));
/// // list is now (1 2 3)
/// assert_eq!(list.len(), 3);
/// ```
#[derive(Clone)]
pub struct List(Option<Rc<ConsCell>>);

impl List {
    /// Creates an empty list.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self(None)
    }

    /// Prepends a value to the front of the list.
    ///
    /// This is an O(1) operation that creates a new cons cell
    /// sharing the tail with the original list.
    #[inline]
    #[must_use]
    pub fn cons(&self, head: Value) -> Self {
        Self(Some(Rc::new(ConsCell {
            head,
            tail: self.clone(),
        })))
    }

    /// Creates a list from a vector of values.
    ///
    /// The first element of the vector becomes the first element of the list.
    #[inline]
    #[must_use]
    pub fn from_vec(values: Vec<Value>) -> Self {
        let mut list = Self(None);
        // Build in reverse so first vec element is at the head
        for value in values.into_iter().rev() {
            list = list.cons(value);
        }
        list
    }

    /// Returns a reference to the first element, if any.
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<&Value> {
        self.0.as_ref().map(|cell| &cell.head)
    }

    /// Returns the rest of the list (everything after the first element).
    ///
    /// This is an O(1) operation due to structural sharing.
    #[inline]
    #[must_use]
    pub fn rest(&self) -> Self {
        self.0
            .as_ref()
            .map_or_else(|| Self(None), |cell| cell.tail.clone())
    }

    /// Returns `true` if the list is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// Returns the number of elements in the list.
    ///
    /// This is an O(n) operation that traverses the entire list.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        let mut count: usize = 0;
        let mut current = self;
        while let Some(ref cell) = current.0 {
            count = count.saturating_add(1);
            current = &cell.tail;
        }
        count
    }

    /// Returns an iterator over references to the list elements.
    #[inline]
    #[must_use]
    pub const fn iter(&self) -> Iter<'_> {
        Iter { current: self }
    }

    /// Creates a wrapper for displaying this list with symbol resolution.
    #[inline]
    #[must_use]
    pub const fn display<'interner>(
        &'interner self,
        interner: &'interner Interner,
    ) -> Displayable<'interner> {
        Displayable {
            list: self,
            interner,
        }
    }
}

/// A wrapper for displaying a [`List`] with symbol name resolution.
///
/// Created via [`List::display`].
pub struct Displayable<'interner> {
    list: &'interner List,
    interner: &'interner Interner,
}

impl Display for Displayable<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut first = true;
        for value in self.list {
            if first {
                first = false;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{}", value.display(self.interner))?;
        }
        write!(f, ")")
    }
}

impl Default for List {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl Display for List {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        let mut first = true;
        for value in self {
            if first {
                first = false;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{value}")?;
        }
        write!(f, ")")
    }
}

impl Debug for List {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "List(")?;
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

impl PartialEq for List {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let mut left_iter = self.iter();
        let mut right_iter = other.iter();

        loop {
            match (left_iter.next(), right_iter.next()) {
                (None, None) => return true,
                (Some(left_val), Some(right_val)) if left_val == right_val => {}
                _ => return false,
            }
        }
    }
}

impl Eq for List {}

impl Hash for List {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the length first for differentiation
        self.len().hash(state);
        for value in self {
            value.hash(state);
        }
    }
}

impl<'list> IntoIterator for &'list List {
    type Item = &'list Value;
    type IntoIter = Iter<'list>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over references to list elements.
pub struct Iter<'list> {
    current: &'list List,
}

impl<'list> Iterator for Iter<'list> {
    type Item = &'list Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.current.0 {
            None => None,
            Some(ref cell) => {
                self.current = &cell.tail;
                Some(&cell.head)
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        // We don't know the size without traversing, so provide no upper bound
        (0, None)
    }
}

impl FusedIterator for Iter<'_> {}

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
    fn empty_list() {
        let list = List::empty();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert!(list.first().is_none());
    }

    #[test]
    fn cons_single_element() {
        let list = List::empty().cons(int(1));
        assert!(!list.is_empty());
        assert_eq!(list.len(), 1);
        assert_eq!(list.first(), Some(&int(1)));
    }

    #[test]
    fn cons_multiple_elements() {
        let list = List::empty().cons(int(3)).cons(int(2)).cons(int(1));
        assert_eq!(list.len(), 3);
        assert_eq!(list.first(), Some(&int(1)));
    }

    #[test]
    fn from_vec_empty() {
        let list = List::from_vec(Vec::new());
        assert!(list.is_empty());
    }

    #[test]
    fn from_vec_elements() {
        let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
        assert_eq!(list.len(), 3);
        assert_eq!(list.first(), Some(&int(1)));
    }

    // =========================================================================
    // Access Tests
    // =========================================================================

    #[test]
    fn first_empty() {
        let list = List::empty();
        assert!(list.first().is_none());
    }

    #[test]
    fn first_non_empty() {
        let list = List::empty().cons(int(42));
        assert_eq!(list.first(), Some(&int(42)));
    }

    #[test]
    fn rest_empty() {
        let list = List::empty();
        let rest = list.rest();
        assert!(rest.is_empty());
    }

    #[test]
    fn rest_single_element() {
        let list = List::empty().cons(int(1));
        let rest = list.rest();
        assert!(rest.is_empty());
    }

    #[test]
    fn rest_multiple_elements() {
        let list = List::empty().cons(int(3)).cons(int(2)).cons(int(1));
        let rest = list.rest();
        assert_eq!(rest.len(), 2);
        assert_eq!(rest.first(), Some(&int(2)));
    }

    // =========================================================================
    // Iterator Tests
    // =========================================================================

    #[test]
    fn iter_empty() {
        let list = List::empty();
        let collected: Vec<_> = list.iter().collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn iter_elements() {
        let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let collected: Vec<_> = list.iter().cloned().collect();
        assert_eq!(collected, alloc::vec![int(1), int(2), int(3)]);
    }

    #[test]
    fn into_iterator() {
        let list = List::from_vec(alloc::vec![int(1), int(2)]);
        let mut count = 0_usize;
        for _val in &list {
            count = count.saturating_add(1);
        }
        assert_eq!(count, 2);
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn display_empty() {
        let list = List::empty();
        assert_eq!(list.to_string(), "()");
    }

    #[test]
    fn display_single() {
        let list = List::empty().cons(int(42));
        assert_eq!(list.to_string(), "(42)");
    }

    #[test]
    fn display_multiple() {
        let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
        assert_eq!(list.to_string(), "(1 2 3)");
    }

    // =========================================================================
    // Equality Tests
    // =========================================================================

    #[test]
    fn equality_empty() {
        let l1 = List::empty();
        let l2 = List::empty();
        assert_eq!(l1, l2);
    }

    #[test]
    fn equality_same_elements() {
        let l1 = List::from_vec(alloc::vec![int(1), int(2)]);
        let l2 = List::from_vec(alloc::vec![int(1), int(2)]);
        assert_eq!(l1, l2);
    }

    #[test]
    fn equality_different_elements() {
        let l1 = List::from_vec(alloc::vec![int(1), int(2)]);
        let l2 = List::from_vec(alloc::vec![int(1), int(3)]);
        assert_ne!(l1, l2);
    }

    #[test]
    fn equality_different_lengths() {
        let l1 = List::from_vec(alloc::vec![int(1), int(2)]);
        let l2 = List::from_vec(alloc::vec![int(1)]);
        assert_ne!(l1, l2);
    }

    // =========================================================================
    // Clone/Sharing Tests
    // =========================================================================

    #[test]
    fn clone_shares_tail() {
        let original = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let cloned = original.clone();

        // Both should have the same elements
        assert_eq!(original, cloned);
        assert_eq!(original.len(), cloned.len());
    }

    #[test]
    fn structural_sharing() {
        let base = List::from_vec(alloc::vec![int(2), int(3)]);
        let extended = base.cons(int(1));

        // The base list is unaffected
        assert_eq!(base.len(), 2);
        assert_eq!(extended.len(), 3);

        // The tail of extended should equal base
        assert_eq!(extended.rest(), base);
    }

    // =========================================================================
    // Default Test
    // =========================================================================

    #[test]
    fn default_is_empty() {
        let list: List = List::default();
        assert!(list.is_empty());
    }

    // =========================================================================
    // Nested List Tests
    // =========================================================================

    #[test]
    fn nested_lists() {
        let inner = List::from_vec(alloc::vec![int(1), int(2)]);
        let outer = List::empty().cons(Value::List(inner.clone()));
        assert_eq!(outer.len(), 1);

        if let Some(Value::List(inner_list)) = outer.first() {
            assert_eq!(inner_list.len(), 2);
        } else {
            panic!("Expected List value");
        }
    }
}
