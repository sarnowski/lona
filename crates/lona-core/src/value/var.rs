// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Var type for mutable variable bindings with metadata.
//!
//! Vars are the building blocks of namespaces. Unlike regular values which
//! are immutable, Vars are mutable containers that hold a value and optional
//! metadata. They use shared references so that multiple references to the
//! same Var see the same underlying data.
//!
//! Vars do NOT implement the [`Meta`] trait because `with-meta` should
//! return an error for Vars. Instead, Vars have their own metadata methods
//! and use `alter-meta!` (future) for mutation.
//!
//! [`Meta`]: crate::Meta

use alloc::rc::Rc;
use core::cell::RefCell;
use core::hash::{Hash, Hasher};

use crate::map::Map;
use crate::symbol;
use crate::value::Value;

/// Internal data for a Var, stored behind a shared reference.
#[derive(Debug)]
pub struct VarData {
    /// The symbol name of this var (without namespace for now).
    name: symbol::Id,
    /// The current value bound to this var.
    value: Value,
    /// Metadata attached to this var (separate from value metadata).
    meta: Option<Map>,
}

/// A Var is a mutable reference to a value with metadata.
///
/// Vars use shared references so that cloning preserves identity.
/// This is essential for `#'x` support where multiple references
/// to the same var must see the same underlying data.
///
/// Unlike regular values, Vars are mutable containers:
/// - The value can be changed with [`set_value`](Var::set_value)
/// - The metadata can be changed with [`set_meta`](Var::set_meta) or
///   [`merge_meta`](Var::merge_meta)
///
/// Vars do NOT implement `Meta` because `with-meta` should return an error
/// for Vars. Use `alter-meta!` (future) for metadata mutation.
#[derive(Debug, Clone)]
pub struct Var(Rc<RefCell<VarData>>);

impl Var {
    /// Creates a new Var with the given name, value, and optional metadata.
    #[inline]
    #[must_use]
    pub fn new(name: symbol::Id, value: Value, meta: Option<Map>) -> Self {
        Self(Rc::new(RefCell::new(VarData { name, value, meta })))
    }

    /// Returns the var's name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> symbol::Id {
        self.0.borrow().name
    }

    /// Returns the current value (cloned).
    #[inline]
    #[must_use]
    pub fn value(&self) -> Value {
        self.0.borrow().value.clone()
    }

    /// Sets the value.
    #[inline]
    pub fn set_value(&self, value: Value) {
        self.0.borrow_mut().value = value;
    }

    /// Returns the metadata (cloned).
    #[inline]
    #[must_use]
    pub fn meta(&self) -> Option<Map> {
        self.0.borrow().meta.clone()
    }

    /// Sets the metadata (replaces existing).
    #[inline]
    pub fn set_meta(&self, meta: Option<Map>) {
        self.0.borrow_mut().meta = meta;
    }

    /// Merges metadata: adds new keys, overwrites existing keys.
    #[inline]
    pub fn merge_meta(&self, new_meta: Map) {
        let mut data = self.0.borrow_mut();
        if let Some(ref existing) = data.meta {
            // Merge new_meta into existing
            let mut merged = existing.clone();
            for (key, val) in new_meta.iter() {
                merged = merged.assoc(key.value().clone(), val.clone());
            }
            data.meta = Some(merged);
        } else {
            data.meta = Some(new_meta);
        }
    }

    /// Returns true if two Vars are the same object (identity check).
    #[inline]
    #[must_use]
    pub fn is_same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Returns the pointer to the shared `VarData` for identity-based ordering.
    ///
    /// This is used by `ValueKey`'s `Ord` implementation to ensure consistency
    /// with `PartialEq` and `Hash`, which also use the `Rc` pointer.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const () {
        Rc::as_ptr(&self.0).cast()
    }
}

/// Equality compares by identity (same Rc pointer).
impl PartialEq for Var {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.is_same(other)
    }
}

impl Eq for Var {}

/// Hash uses the pointer address for identity-based hashing.
impl Hash for Var {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.0).hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integer::Integer;
    use crate::symbol::Interner;

    #[test]
    fn var_identity_preserved_on_clone() {
        let mut interner = Interner::new();
        let name = interner.intern("x");
        let var = Var::new(name, Value::Integer(Integer::from_i64(42)), None);
        let var_clone = var.clone();

        assert!(var.is_same(&var_clone));
    }

    #[test]
    fn var_value_get_set() {
        let mut interner = Interner::new();
        let name = interner.intern("x");
        let var = Var::new(name, Value::Integer(Integer::from_i64(1)), None);

        assert_eq!(var.value(), Value::Integer(Integer::from_i64(1)));

        var.set_value(Value::Integer(Integer::from_i64(2)));
        assert_eq!(var.value(), Value::Integer(Integer::from_i64(2)));

        // Clone sees the new value
        let var_clone = var.clone();
        assert_eq!(var_clone.value(), Value::Integer(Integer::from_i64(2)));
    }

    #[test]
    fn var_metadata_get_set() {
        let mut interner = Interner::new();
        let name = interner.intern("x");
        let var = Var::new(name, Value::Nil, None);

        assert!(var.meta().is_none());

        let meta = Map::empty().assoc(Value::from(1_i32), Value::from(2_i32));
        var.set_meta(Some(meta.clone()));

        assert!(var.meta().is_some());
        assert_eq!(var.meta().unwrap(), meta);
    }

    #[test]
    fn var_metadata_merge() {
        let mut interner = Interner::new();
        let name = interner.intern("x");

        let meta_a = Map::empty().assoc(Value::from(1_i32), Value::from(10_i32));
        let var = Var::new(name, Value::Nil, Some(meta_a));

        let meta_b = Map::empty()
            .assoc(Value::from(2_i32), Value::from(20_i32))
            .assoc(Value::from(1_i32), Value::from(100_i32)); // Overwrites key 1

        var.merge_meta(meta_b);

        let result = var.meta().unwrap();
        // Key 1 should be overwritten to 100
        assert_eq!(result.get(&Value::from(1_i32)), Some(&Value::from(100_i32)));
        // Key 2 should be added
        assert_eq!(result.get(&Value::from(2_i32)), Some(&Value::from(20_i32)));
    }

    #[test]
    fn var_metadata_merge_when_none() {
        let mut interner = Interner::new();
        let name = interner.intern("x");
        let var = Var::new(name, Value::Nil, None);

        let meta = Map::empty().assoc(Value::from(1_i32), Value::from(10_i32));
        var.merge_meta(meta.clone());

        assert_eq!(var.meta(), Some(meta));
    }

    #[test]
    fn var_name() {
        let mut interner = Interner::new();
        let name = interner.intern("my-var");
        let var = Var::new(name, Value::Nil, None);

        assert_eq!(var.name(), name);
    }

    #[test]
    fn var_equality_is_identity() {
        let mut interner = Interner::new();
        let name = interner.intern("x");

        let var1 = Var::new(name, Value::Integer(Integer::from_i64(42)), None);
        let var2 = Var::new(name, Value::Integer(Integer::from_i64(42)), None);

        // Same content, different Vars
        assert!(!var1.is_same(&var2));
        assert_ne!(var1, var2);

        // Clone is the same Var
        let var1_clone = var1.clone();
        assert!(var1.is_same(&var1_clone));
        assert_eq!(var1, var1_clone);
    }

    #[test]
    fn var_hash_is_identity_based() {
        use crate::fnv::FnvHasher;
        use core::hash::Hasher;

        let mut interner = Interner::new();
        let name = interner.intern("x");

        let var1 = Var::new(name, Value::Integer(Integer::from_i64(42)), None);
        let var2 = Var::new(name, Value::Integer(Integer::from_i64(42)), None);

        let mut h1 = FnvHasher::default();
        let mut h2 = FnvHasher::default();
        var1.hash(&mut h1);
        var2.hash(&mut h2);

        // Different Vars have different hashes
        assert_ne!(h1.finish(), h2.finish());

        // Clone has same hash
        let var1_clone = var1.clone();
        let mut h3 = FnvHasher::default();
        var1_clone.hash(&mut h3);

        let mut h1_again = FnvHasher::default();
        var1.hash(&mut h1_again);

        assert_eq!(h1_again.finish(), h3.finish());
    }
}
