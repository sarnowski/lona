// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Namespace system for organizing code into named modules.
//!
//! Namespaces are the primary mechanism for code organization in Lonala.
//! Every [`Var`] belongs to exactly one namespace, and the runtime tracks
//! a "current" namespace for symbol resolution.
//!
//! # Structure
//!
//! Each namespace contains:
//! - **Mappings**: Vars defined directly in this namespace via `def`
//! - **Refers**: Vars imported from other namespaces via `refer`
//! - **Aliases**: Shorthand names for other namespaces
//!
//! # Lookup Order
//!
//! When resolving an unqualified symbol:
//! 1. Check mappings (vars defined in this namespace)
//! 2. Check refers (vars imported from other namespaces)
//!
//! Mappings take precedence over refers, allowing local definitions
//! to shadow imported names.

#[cfg(feature = "alloc")]
mod registry;

#[cfg(feature = "alloc")]
pub use registry::Registry;

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;

use lona_core::map::Map;
use lona_core::symbol;
use lona_core::value::{Value, Var};

/// A namespace is a named container for Vars.
///
/// Namespaces provide code organization and prevent name collisions.
/// Each namespace has a unique name (e.g., `lona.core`, `user`) and
/// contains a set of Vars that can be looked up by symbol.
#[cfg(feature = "alloc")]
#[non_exhaustive]
pub struct Namespace {
    /// Namespace name (e.g., symbol for "lona.core").
    name: symbol::Id,

    /// Vars defined in this namespace (symbol → Var).
    mappings: BTreeMap<symbol::Id, Var>,

    /// Aliases to other namespaces (alias → namespace name).
    aliases: BTreeMap<symbol::Id, symbol::Id>,

    /// Referred vars from other namespaces (symbol → Var).
    refers: BTreeMap<symbol::Id, Var>,

    /// Namespace metadata (docstring, etc.).
    meta: Option<Map>,
}

#[cfg(feature = "alloc")]
impl Namespace {
    /// Creates a new empty namespace with the given name.
    #[inline]
    #[must_use]
    pub const fn new(name: symbol::Id) -> Self {
        Self {
            name,
            mappings: BTreeMap::new(),
            aliases: BTreeMap::new(),
            refers: BTreeMap::new(),
            meta: None,
        }
    }

    /// Returns the namespace's name.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> symbol::Id {
        self.name
    }

    /// Interns a symbol in this namespace, creating or updating its Var.
    ///
    /// If the symbol already exists in mappings, updates the Var's value
    /// and returns the existing Var (preserving identity). If not,
    /// creates a new Var with this namespace's name and inserts it.
    #[inline]
    pub fn intern(&mut self, sym: symbol::Id, value: Value) -> Var {
        if let Some(var) = self.mappings.get(&sym) {
            var.set_value(value);
            return var.clone();
        }

        let var = Var::new(sym, Some(self.name), value, None);
        let _previous = self.mappings.insert(sym, var.clone());
        var
    }

    /// Looks up a symbol's value in this namespace.
    ///
    /// Checks mappings first, then refers. Returns `None` if the
    /// symbol is not found in either.
    #[inline]
    #[must_use]
    pub fn lookup(&self, sym: symbol::Id) -> Option<Value> {
        if let Some(var) = self.mappings.get(&sym) {
            return Some(var.value());
        }
        if let Some(var) = self.refers.get(&sym) {
            return Some(var.value());
        }
        None
    }

    /// Gets a reference to a Var by symbol, if it exists in mappings.
    ///
    /// This is for introspection; it returns the Var itself rather than
    /// its value. Only checks mappings, not refers.
    #[inline]
    #[must_use]
    pub fn get_var(&self, sym: symbol::Id) -> Option<&Var> {
        self.mappings.get(&sym)
    }

    /// Adds an alias for another namespace.
    ///
    /// After calling `add_alias(alias, ns_name)`, the alias can be used
    /// to refer to the namespace `ns_name` in qualified symbols.
    #[inline]
    pub fn add_alias(&mut self, alias: symbol::Id, ns_name: symbol::Id) {
        let _previous = self.aliases.insert(alias, ns_name);
    }

    /// Gets the namespace name for an alias, if it exists.
    #[inline]
    #[must_use]
    pub fn get_alias(&self, alias: symbol::Id) -> Option<symbol::Id> {
        self.aliases.get(&alias).copied()
    }

    /// Adds a referred Var from another namespace.
    ///
    /// After calling `add_refer(sym, var)`, the symbol `sym` will resolve
    /// to the given Var during lookup (if not shadowed by a mapping).
    #[inline]
    pub fn add_refer(&mut self, sym: symbol::Id, var: Var) {
        let _previous = self.refers.insert(sym, var);
    }

    /// Returns an iterator over all mappings (symbol → Var).
    #[inline]
    pub fn mappings(&self) -> impl Iterator<Item = (&symbol::Id, &Var)> {
        self.mappings.iter()
    }

    /// Returns an iterator over all aliases (alias → namespace name).
    #[inline]
    pub fn aliases(&self) -> impl Iterator<Item = (&symbol::Id, &symbol::Id)> {
        self.aliases.iter()
    }

    /// Returns an iterator over all refers (symbol → Var).
    #[inline]
    pub fn refers(&self) -> impl Iterator<Item = (&symbol::Id, &Var)> {
        self.refers.iter()
    }

    /// Returns the number of vars defined in this namespace (mappings only).
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// Returns true if no vars are defined in this namespace.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Returns the namespace's metadata.
    #[inline]
    #[must_use]
    pub const fn meta(&self) -> Option<&Map> {
        self.meta.as_ref()
    }

    /// Sets the namespace's metadata.
    #[inline]
    pub fn set_meta(&mut self, meta: Option<Map>) {
        self.meta = meta;
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;
    use lona_core::integer::Integer;
    use lona_core::symbol::Interner;

    #[test]
    fn test_namespace_creation() {
        let interner = Interner::new();
        let name = interner.intern("user");
        let ns = Namespace::new(name);

        assert_eq!(ns.name(), name);
        assert!(ns.is_empty());
        assert_eq!(ns.len(), 0);
    }

    #[test]
    fn test_intern_creates_var() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let sym = interner.intern("x");
        let value = Value::Integer(Integer::from_i64(42));

        let var = ns.intern(sym, value.clone());

        assert_eq!(var.value(), value);
        assert_eq!(var.name(), sym);
        assert_eq!(ns.len(), 1);
    }

    #[test]
    fn test_intern_twice_returns_same_var() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let sym = interner.intern("x");
        let value1 = Value::Integer(Integer::from_i64(1));
        let value2 = Value::Integer(Integer::from_i64(2));

        let var1 = ns.intern(sym, value1);
        let var2 = ns.intern(sym, value2.clone());

        // Same identity
        assert!(var1.is_same(&var2));

        // Value was updated
        assert_eq!(var1.value(), value2);
        assert_eq!(var2.value(), value2);

        // Only one mapping
        assert_eq!(ns.len(), 1);
    }

    #[test]
    fn test_lookup_finds_interned_var() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let sym = interner.intern("x");
        let value = Value::Integer(Integer::from_i64(42));
        let _var = ns.intern(sym, value.clone());

        assert_eq!(ns.lookup(sym), Some(value));
    }

    #[test]
    fn test_lookup_finds_referred_var() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let sym = interner.intern("x");
        let value = Value::Integer(Integer::from_i64(42));
        let var = Var::new(sym, None, value.clone(), None);
        ns.add_refer(sym, var);

        assert_eq!(ns.lookup(sym), Some(value));
    }

    #[test]
    fn test_lookup_prefers_mapping_over_refer() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let sym = interner.intern("x");
        let refer_value = Value::Integer(Integer::from_i64(1));
        let mapping_value = Value::Integer(Integer::from_i64(2));

        // Add refer first
        let refer_var = Var::new(sym, None, refer_value, None);
        ns.add_refer(sym, refer_var);

        // Then add mapping
        let _mapping_var = ns.intern(sym, mapping_value.clone());

        // Lookup should return the mapping value
        assert_eq!(ns.lookup(sym), Some(mapping_value));
    }

    #[test]
    fn test_lookup_returns_none_for_unknown() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let ns = Namespace::new(ns_name);

        let unknown = interner.intern("unknown");
        assert_eq!(ns.lookup(unknown), None);
    }

    #[test]
    fn test_get_var_returns_var() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let sym = interner.intern("x");
        let value = Value::Integer(Integer::from_i64(42));
        let var = ns.intern(sym, value);

        let Some(retrieved) = ns.get_var(sym) else {
            panic!("get_var should return Some for interned symbol");
        };
        assert!(retrieved.is_same(&var));
    }

    #[test]
    fn test_get_var_ignores_refers() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let sym = interner.intern("x");
        let value = Value::Integer(Integer::from_i64(42));
        let var = Var::new(sym, None, value, None);
        ns.add_refer(sym, var);

        // get_var only checks mappings, not refers
        assert!(ns.get_var(sym).is_none());
    }

    #[test]
    fn test_add_alias() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let alias = interner.intern("str");
        let target = interner.intern("lona.string");

        ns.add_alias(alias, target);

        assert_eq!(ns.get_alias(alias), Some(target));
    }

    #[test]
    fn test_alias_not_found() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let ns = Namespace::new(ns_name);

        let unknown = interner.intern("unknown");
        assert_eq!(ns.get_alias(unknown), None);
    }

    #[test]
    fn test_mappings_iterator() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let sym_a = interner.intern("a");
        let sym_b = interner.intern("b");
        let _var_a = ns.intern(sym_a, Value::Integer(Integer::from_i64(1)));
        let _var_b = ns.intern(sym_b, Value::Integer(Integer::from_i64(2)));

        let mappings: Vec<_> = ns.mappings().collect();
        assert_eq!(mappings.len(), 2);
    }

    #[test]
    fn test_namespace_metadata() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        assert!(ns.meta().is_none());

        let meta = Map::empty().assoc(Value::from(1_i32), Value::from(2_i32));
        ns.set_meta(Some(meta.clone()));

        assert_eq!(ns.meta(), Some(&meta));
    }

    #[test]
    fn test_aliases_iterator() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let alias_a = interner.intern("str");
        let target_a = interner.intern("lona.string");
        let alias_b = interner.intern("io");
        let target_b = interner.intern("lona.io");

        ns.add_alias(alias_a, target_a);
        ns.add_alias(alias_b, target_b);

        let aliases: Vec<_> = ns.aliases().collect();
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn test_refers_iterator() {
        let interner = Interner::new();
        let ns_name = interner.intern("user");
        let mut ns = Namespace::new(ns_name);

        let sym_a = interner.intern("map");
        let sym_b = interner.intern("filter");
        let var_a = Var::new(sym_a, None, Value::Integer(Integer::from_i64(1)), None);
        let var_b = Var::new(sym_b, None, Value::Integer(Integer::from_i64(2)), None);

        ns.add_refer(sym_a, var_a);
        ns.add_refer(sym_b, var_b);

        let refers: Vec<_> = ns.refers().collect();
        assert_eq!(refers.len(), 2);
    }

    #[test]
    fn test_intern_sets_namespace_on_var() {
        let interner = Interner::new();
        let ns_name = interner.intern("my.namespace");
        let mut ns = Namespace::new(ns_name);

        let sym = interner.intern("x");
        let value = Value::Integer(Integer::from_i64(42));
        let var = ns.intern(sym, value);

        // Var should have the namespace set
        assert_eq!(var.namespace(), Some(ns_name));
        assert_eq!(var.name(), sym);
    }
}
