// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Symbol interning for efficient identifier storage and comparison.
//!
//! Symbols are interned strings that allow O(1) equality comparison using
//! integer IDs instead of string comparison. This is essential for a
//! Lisp-like language where symbols are ubiquitous.

#[cfg(feature = "alloc")]
use alloc::{collections::BTreeMap, string::String, vec::Vec};

/// Unique identifier for an interned symbol.
///
/// Provides O(1) equality comparison for symbols. Two symbols with the
/// same name always have the same ID within a given [`Interner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u32);

impl Id {
    /// Returns the raw numeric identifier.
    #[inline]
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the raw numeric identifier as usize for indexing.
    #[inline]
    #[must_use]
    #[expect(clippy::as_conversions, reason = "u32 to usize is always safe")]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// Symbol interner that deduplicates symbol strings.
///
/// Stores each unique symbol string once and assigns it an [`Id`].
/// Interning the same string multiple times returns the same ID.
#[cfg(feature = "alloc")]
pub struct Interner {
    /// Maps [`Id`] index to the interned string.
    strings: Vec<String>,
    /// Maps string content to its [`Id`] for deduplication.
    lookup: BTreeMap<String, Id>,
}

#[cfg(feature = "alloc")]
impl Interner {
    /// Creates a new empty symbol interner.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            strings: Vec::new(),
            lookup: BTreeMap::new(),
        }
    }

    /// Interns a symbol string, returning its unique [`Id`].
    ///
    /// If the string was previously interned, returns the existing ID.
    /// Otherwise, allocates a new ID and stores the string.
    ///
    /// # Panics
    ///
    /// Panics if more than `u32::MAX` symbols are interned.
    #[inline]
    #[expect(clippy::expect_used, reason = "overflow is unrecoverable")]
    pub fn intern(&mut self, name: &str) -> Id {
        if let Some(&id) = self.lookup.get(name) {
            return id;
        }

        let id = Id(u32::try_from(self.strings.len())
            .expect("symbol table overflow: exceeded u32::MAX symbols"));
        self.strings.push(String::from(name));
        let _previous = self.lookup.insert(String::from(name), id);
        id
    }

    /// Resolves an [`Id`] to its string representation.
    ///
    /// # Panics
    ///
    /// Panics if the ID was not created by this interner.
    #[inline]
    #[must_use]
    #[expect(clippy::expect_used, reason = "invalid ID is a programming error")]
    pub fn resolve(&self, id: Id) -> &str {
        self.strings
            .get(id.as_usize())
            .expect("invalid symbol Id: not from this interner")
    }

    /// Looks up a symbol by name without interning it.
    ///
    /// Returns `Some(id)` if the symbol was previously interned,
    /// `None` otherwise.
    #[inline]
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Id> {
        self.lookup.get(name).copied()
    }

    /// Returns the number of interned symbols.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.strings.len()
    }

    /// Returns `true` if no symbols have been interned.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

#[cfg(feature = "alloc")]
impl Default for Interner {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_same_string_returns_same_id() {
        let mut interner = Interner::new();
        let id1 = interner.intern("foo");
        let id2 = interner.intern("foo");
        assert_eq!(id1, id2);
    }

    #[test]
    fn intern_different_strings_returns_different_ids() {
        let mut interner = Interner::new();
        let id1 = interner.intern("foo");
        let id2 = interner.intern("bar");
        assert_ne!(id1, id2);
    }

    #[test]
    fn resolve_returns_original_string() {
        let mut interner = Interner::new();
        let id = interner.intern("hello-world");
        assert_eq!(interner.resolve(id), "hello-world");
    }

    #[test]
    fn get_returns_none_for_unknown_symbol() {
        let interner = Interner::new();
        assert_eq!(interner.get("unknown"), None);
    }

    #[test]
    fn get_returns_some_for_known_symbol() {
        let mut interner = Interner::new();
        let id = interner.intern("known");
        assert_eq!(interner.get("known"), Some(id));
    }

    #[test]
    fn len_tracks_unique_symbols() {
        let mut interner = Interner::new();
        assert_eq!(interner.len(), 0);
        assert!(interner.is_empty());

        let _id1 = interner.intern("first");
        assert_eq!(interner.len(), 1);

        let _id2 = interner.intern("second");
        assert_eq!(interner.len(), 2);

        // Re-interning doesn't increase count
        let _id3 = interner.intern("first");
        assert_eq!(interner.len(), 2);
        assert!(!interner.is_empty());
    }

    #[test]
    fn symbol_id_as_u32() {
        let mut interner = Interner::new();
        let id = interner.intern("test");
        assert_eq!(id.as_u32(), 0);

        let id2 = interner.intern("test2");
        assert_eq!(id2.as_u32(), 1);
    }

    #[test]
    fn interning_special_characters() {
        let mut interner = Interner::new();

        // Lisp-style symbols with special characters
        let id_plus = interner.intern("+");
        let id_minus = interner.intern("-");
        let id_arrow = interner.intern("->");
        let id_bang = interner.intern("update!");
        let id_pred = interner.intern("empty?");

        assert_eq!(interner.resolve(id_plus), "+");
        assert_eq!(interner.resolve(id_minus), "-");
        assert_eq!(interner.resolve(id_arrow), "->");
        assert_eq!(interner.resolve(id_bang), "update!");
        assert_eq!(interner.resolve(id_pred), "empty?");
    }

    #[test]
    fn interning_empty_string() {
        let mut interner = Interner::new();
        let id = interner.intern("");
        assert_eq!(interner.resolve(id), "");
    }

    #[test]
    fn symbol_id_equality() {
        let mut interner = Interner::new();
        let id1 = interner.intern("same");
        let id2 = interner.intern("same");
        let id3 = interner.intern("different");

        // Same symbol IDs are equal
        assert!(id1 == id2);
        // Different symbol IDs are not equal
        assert!(id1 != id3);
    }
}
