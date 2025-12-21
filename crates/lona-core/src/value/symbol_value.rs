// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Symbol type with metadata support.
//!
//! Wraps an interned symbol ID with optional metadata, enabling symbols
//! to carry information while still using efficient ID-based comparison.

use core::hash::{Hash, Hasher};

use crate::map::Map;
use crate::meta::Meta;
use crate::symbol;

/// An interned symbol with optional metadata.
///
/// Symbols are identifiers used to name things. The name is interned
/// for O(1) equality comparison, while metadata can vary between
/// symbol instances with the same name.
///
/// Two symbols with the same ID but different metadata compare equal,
/// following Clojure's metadata semantics.
#[derive(Clone, Debug)]
pub struct Symbol {
    /// The interned symbol ID.
    id: symbol::Id,
    /// Optional metadata map.
    meta: Option<Map>,
}

impl Symbol {
    /// Creates a new symbol with the given ID and no metadata.
    #[inline]
    #[must_use]
    pub const fn new(id: symbol::Id) -> Self {
        Self { id, meta: None }
    }

    /// Creates a new symbol with the given ID and metadata.
    #[inline]
    #[must_use]
    pub const fn with_id_and_meta(id: symbol::Id, meta: Option<Map>) -> Self {
        Self { id, meta }
    }

    /// Returns the interned symbol ID.
    #[inline]
    #[must_use]
    pub const fn id(&self) -> symbol::Id {
        self.id
    }
}

impl Meta for Symbol {
    #[inline]
    fn meta(&self) -> Option<&Map> {
        self.meta.as_ref()
    }

    #[inline]
    fn with_meta(self, meta: Option<Map>) -> Self {
        Self { id: self.id, meta }
    }
}

/// Equality ignores metadata.
impl PartialEq for Symbol {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Symbol {}

/// Ordering ignores metadata.
impl PartialOrd for Symbol {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Symbol {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

/// Hash ignores metadata.
impl Hash for Symbol {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl From<symbol::Id> for Symbol {
    #[inline]
    fn from(id: symbol::Id) -> Self {
        Self::new(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn symbol_equality_ignores_metadata() {
        let sym_id = symbol::Id::new(42);
        let sym1 = Symbol::new(sym_id);
        let sym2 = Symbol::with_id_and_meta(sym_id, Some(Map::empty()));

        assert_eq!(sym1, sym2);
    }

    #[test]
    fn symbol_hash_ignores_metadata() {
        use crate::fnv::FnvHasher;
        use core::hash::Hasher;

        let sym_id = symbol::Id::new(42);
        let sym1 = Symbol::new(sym_id);
        let sym2 = Symbol::with_id_and_meta(
            sym_id,
            Some(Map::empty().assoc(Value::from(1_i32), Value::from(2_i32))),
        );

        let mut h1 = FnvHasher::default();
        let mut h2 = FnvHasher::default();
        sym1.hash(&mut h1);
        sym2.hash(&mut h2);

        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn symbol_with_meta_returns_new_symbol_with_metadata() {
        let sym_id = symbol::Id::new(42);
        let sym = Symbol::new(sym_id);
        let meta = Map::empty().assoc(Value::from(1_i32), Value::from(2_i32));

        let sym_with_meta = sym.with_meta(Some(meta.clone()));

        assert_eq!(sym_with_meta.id(), sym_id);
        assert_eq!(sym_with_meta.meta(), Some(&meta));
    }

    #[test]
    fn symbol_meta_returns_none_for_new_symbol() {
        let sym = Symbol::new(symbol::Id::new(42));
        assert!(sym.meta().is_none());
    }

    #[test]
    fn symbol_with_meta_none_clears_metadata() {
        let sym_id = symbol::Id::new(42);
        let meta = Map::empty().assoc(Value::from(1_i32), Value::from(2_i32));
        let sym = Symbol::with_id_and_meta(sym_id, Some(meta));

        let cleared = sym.with_meta(None);
        assert!(cleared.meta().is_none());
    }
}
