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
#[path = "list_tests.rs"]
mod tests;
