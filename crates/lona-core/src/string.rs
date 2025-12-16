// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! String representation for the Lonala language.
//!
//! Provides an immutable, reference-counted string type that enables
//! efficient sharing of string data. Uses `Rc<str>` for heap allocation
//! with reference counting.

use alloc::rc::Rc;
use alloc::string::String;

use core::cmp::Ordering;
use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};

/// An immutable, reference-counted string for Lonala values.
///
/// Wraps `Rc<str>` to provide efficient cloning through reference counting.
/// Strings are immutable once created, enabling safe sharing across values.
///
/// # Example
///
/// ```ignore
/// let s1 = HeapStr::new("hello");
/// let s2 = s1.clone();  // Cheap: increments reference count
/// assert_eq!(s1.as_str(), "hello");
/// ```
#[derive(Clone)]
pub struct HeapStr(Rc<str>);

impl HeapStr {
    /// Creates a new `HeapStr` from a string slice.
    #[inline]
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(Rc::from(text))
    }

    /// Returns the string contents as a slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the length of the string in bytes.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the string is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for HeapStr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for HeapStr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl PartialEq for HeapStr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for HeapStr {}

impl PartialOrd for HeapStr {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapStr {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Hash for HeapStr {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl From<&str> for HeapStr {
    #[inline]
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for HeapStr {
    #[inline]
    fn from(text: String) -> Self {
        Self(Rc::from(text))
    }
}

impl AsRef<str> for HeapStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeSet;
    use alloc::string::ToString;

    #[test]
    fn new_creates_string() {
        let string = HeapStr::new("hello");
        assert_eq!(string.as_str(), "hello");
    }

    #[test]
    fn len_returns_byte_count() {
        let string = HeapStr::new("hello");
        assert_eq!(string.len(), 5);

        let unicode = HeapStr::new("héllo");
        assert_eq!(unicode.len(), 6); // é is 2 bytes in UTF-8
    }

    #[test]
    fn is_empty_for_empty_string() {
        let empty = HeapStr::new("");
        assert!(empty.is_empty());

        let non_empty = HeapStr::new("x");
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn clone_is_cheap() {
        let s1 = HeapStr::new("test");
        let s2 = s1.clone();
        // Both point to the same data (Rc)
        assert_eq!(s1.as_str(), s2.as_str());
        assert!(Rc::ptr_eq(&s1.0, &s2.0));
    }

    #[test]
    fn display_shows_content() {
        let string = HeapStr::new("hello world");
        assert_eq!(string.to_string(), "hello world");
    }

    #[test]
    fn debug_shows_quoted() {
        let string = HeapStr::new("hello");
        let debug_output = alloc::format!("{string:?}");
        assert_eq!(debug_output, "\"hello\"");
    }

    #[test]
    fn equality_by_content() {
        let s1 = HeapStr::new("test");
        let s2 = HeapStr::new("test");
        let s3 = HeapStr::new("other");

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn ordering_lexicographic() {
        let apple = HeapStr::new("apple");
        let banana = HeapStr::new("banana");
        let cherry = HeapStr::new("cherry");

        assert!(apple < banana);
        assert!(banana < cherry);
        assert!(apple < cherry);
    }

    #[test]
    fn hash_consistent() {
        let s1 = HeapStr::new("test");
        let s2 = HeapStr::new("test");

        // Can be used in hash-based collections
        let mut set = BTreeSet::new();
        set.insert(s1.clone());
        assert!(set.contains(&s2));
    }

    #[test]
    fn from_str_slice() {
        let string: HeapStr = "hello".into();
        assert_eq!(string.as_str(), "hello");
    }

    #[test]
    fn from_string() {
        let owned = String::from("hello");
        let string: HeapStr = owned.into();
        assert_eq!(string.as_str(), "hello");
    }

    #[test]
    fn as_ref_str() {
        let string = HeapStr::new("test");
        let slice: &str = string.as_ref();
        assert_eq!(slice, "test");
    }

    #[test]
    fn empty_string() {
        let empty = HeapStr::new("");
        assert_eq!(empty.as_str(), "");
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn unicode_content() {
        let unicode = HeapStr::new("日本語");
        assert_eq!(unicode.as_str(), "日本語");
        assert_eq!(unicode.len(), 9); // 3 characters × 3 bytes each
    }

    #[test]
    fn special_characters() {
        let special = HeapStr::new("hello\nworld\ttab");
        assert_eq!(special.as_str(), "hello\nworld\ttab");
    }
}
