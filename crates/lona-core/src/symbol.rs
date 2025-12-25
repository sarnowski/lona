// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Symbol interning for efficient identifier storage and comparison.
//!
//! Symbols are interned strings that allow O(1) equality comparison using
//! integer IDs instead of string comparison. This is essential for a
//! Lisp-like language where symbols are ubiquitous.

#[cfg(feature = "alloc")]
use alloc::{collections::BTreeMap, format, string::String, vec::Vec};
use core::cell::RefCell;

/// Unique identifier for an interned symbol.
///
/// Provides O(1) equality comparison for symbols. Two symbols with the
/// same name always have the same ID within a given [`Interner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u32);

impl Id {
    /// Creates a new symbol ID from a raw index.
    ///
    /// This is primarily for testing and internal use. Normal code should
    /// use [`Interner::intern`] to create symbol IDs.
    #[inline]
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

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
///
/// Uses interior mutability via `RefCell` to allow interning and gensym
/// through shared references (`&self`). This is safe because the runtime
/// is single-threaded (seL4 root task).
#[cfg(feature = "alloc")]
pub struct Interner {
    /// Maps [`Id`] index to the interned string.
    strings: RefCell<Vec<String>>,
    /// Maps string content to its [`Id`] for deduplication.
    lookup: RefCell<BTreeMap<String, Id>>,
    /// Monotonic counter for gensym.
    gensym_counter: RefCell<u64>,
}

#[cfg(feature = "alloc")]
impl Interner {
    /// Creates a new empty symbol interner.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            strings: RefCell::new(Vec::new()),
            lookup: RefCell::new(BTreeMap::new()),
            gensym_counter: RefCell::new(0),
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
    pub fn intern(&self, name: &str) -> Id {
        if let Some(&id) = self.lookup.borrow().get(name) {
            return id;
        }

        let mut strings = self.strings.borrow_mut();
        let id =
            Id(u32::try_from(strings.len())
                .expect("symbol table overflow: exceeded u32::MAX symbols"));
        strings.push(String::from(name));
        let _previous = self.lookup.borrow_mut().insert(String::from(name), id);
        id
    }

    /// Resolves an [`Id`] to its string representation.
    ///
    /// Returns an owned `String` because the interner uses interior mutability.
    ///
    /// # Panics
    ///
    /// Panics if the ID was not created by this interner.
    #[inline]
    #[must_use]
    #[expect(clippy::expect_used, reason = "invalid ID is a programming error")]
    pub fn resolve(&self, id: Id) -> String {
        self.strings
            .borrow()
            .get(id.as_usize())
            .expect("invalid symbol Id: not from this interner")
            .clone()
    }

    /// Looks up a symbol by name without interning it.
    ///
    /// Returns `Some(id)` if the symbol was previously interned,
    /// `None` otherwise.
    #[inline]
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Id> {
        self.lookup.borrow().get(name).copied()
    }

    /// Returns the number of interned symbols.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.borrow().len()
    }

    /// Returns `true` if no symbols have been interned.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.borrow().is_empty()
    }

    /// Generates a unique symbol with optional prefix.
    ///
    /// # Examples
    ///
    /// - `gensym(None)` returns symbol like `G__123`
    /// - `gensym(Some("temp"))` returns symbol like `temp__123`
    #[inline]
    pub fn gensym(&self, prefix: Option<&str>) -> Id {
        let mut counter = self.gensym_counter.borrow_mut();
        let current = *counter;
        // Use wrapping_add - u64 overflow is practically impossible
        // (would require ~18 quintillion gensyms)
        *counter = counter.wrapping_add(1);

        let sym_prefix = prefix.unwrap_or("G");
        let name = format!("{sym_prefix}__{current}");
        self.intern(&name)
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
        let interner = Interner::new();
        let id1 = interner.intern("foo");
        let id2 = interner.intern("foo");
        assert_eq!(id1, id2);
    }

    #[test]
    fn intern_different_strings_returns_different_ids() {
        let interner = Interner::new();
        let id1 = interner.intern("foo");
        let id2 = interner.intern("bar");
        assert_ne!(id1, id2);
    }

    #[test]
    fn resolve_returns_original_string() {
        let interner = Interner::new();
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
        let interner = Interner::new();
        let id = interner.intern("known");
        assert_eq!(interner.get("known"), Some(id));
    }

    #[test]
    fn len_tracks_unique_symbols() {
        let interner = Interner::new();
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
        let interner = Interner::new();
        let id = interner.intern("test");
        assert_eq!(id.as_u32(), 0);

        let id2 = interner.intern("test2");
        assert_eq!(id2.as_u32(), 1);
    }

    #[test]
    fn interning_special_characters() {
        let interner = Interner::new();

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
        let interner = Interner::new();
        let id = interner.intern("");
        assert_eq!(interner.resolve(id), "");
    }

    #[test]
    fn symbol_id_equality() {
        let interner = Interner::new();
        let id1 = interner.intern("same");
        let id2 = interner.intern("same");
        let id3 = interner.intern("different");

        // Same symbol IDs are equal
        assert!(id1 == id2);
        // Different symbol IDs are not equal
        assert!(id1 != id3);
    }

    #[test]
    fn gensym_generates_unique_symbols() {
        let interner = Interner::new();
        let id1 = interner.gensym(None);
        let id2 = interner.gensym(None);
        assert_ne!(id1, id2);
    }

    #[test]
    fn gensym_uses_default_prefix() {
        let interner = Interner::new();
        let id = interner.gensym(None);
        assert!(interner.resolve(id).starts_with("G__"));
    }

    #[test]
    fn gensym_uses_custom_prefix() {
        let interner = Interner::new();
        let id = interner.gensym(Some("temp"));
        assert!(interner.resolve(id).starts_with("temp__"));
    }

    #[test]
    fn gensym_counter_is_monotonic() {
        let interner = Interner::new();
        let id1 = interner.gensym(None);
        let id2 = interner.gensym(None);
        let name1 = interner.resolve(id1);
        let name2 = interner.resolve(id2);
        // Extract counter values and verify monotonicity
        let n1: u64 = name1.split("__").nth(1).unwrap().parse().unwrap();
        let n2: u64 = name2.split("__").nth(1).unwrap().parse().unwrap();
        assert!(n2 > n1);
    }
}
