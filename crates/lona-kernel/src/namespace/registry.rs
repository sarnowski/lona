// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Namespace registry for managing all namespaces in the runtime.
//!
//! The registry tracks all namespaces and maintains the "current" namespace
//! context used for symbol resolution during evaluation.
//!
//! # Bootstrap
//!
//! When created, the registry automatically bootstraps with:
//! - `lona.core` - The core library namespace (will contain builtins)
//! - `user` - The default namespace for REPL interaction
//!
//! The `user` namespace is set as the current namespace by default.

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;

use lona_core::symbol::{self, Interner};

use super::Namespace;

/// A registry of all namespaces in the runtime.
///
/// The registry is the central authority for namespace management. It tracks
/// all namespaces by name and maintains the "current" namespace context.
///
/// # Auto-Refer `lona.core`
///
/// Like Clojure, all namespaces automatically refer `lona.core`. This means
/// that core functions like `first`, `rest`, `+`, etc. are available without
/// qualification in all namespaces. When a new namespace is created via
/// [`get_or_create`](Self::get_or_create), all public vars from `lona.core`
/// are automatically referred into it.
#[cfg(feature = "alloc")]
#[derive(Clone)]
#[non_exhaustive]
pub struct Registry {
    /// All namespaces (namespace name → Namespace).
    namespaces: BTreeMap<symbol::Id, Namespace>,

    /// Current namespace (the context for `def` and unqualified lookups).
    current: symbol::Id,

    /// The `lona.core` namespace name, cached for auto-refer.
    core_name: symbol::Id,
}

#[cfg(feature = "alloc")]
impl Registry {
    /// Creates a new registry with bootstrapped namespaces.
    ///
    /// This creates:
    /// - `lona.core` namespace (empty, will be populated with builtins)
    /// - `user` namespace (set as current, with auto-refer of `lona.core`)
    ///
    /// The `user` namespace automatically refers all public vars from
    /// `lona.core`, making core functions available without qualification.
    /// New namespaces created via [`get_or_create`](Self::get_or_create) also
    /// get this auto-refer behavior.
    #[inline]
    #[must_use]
    pub fn new(interner: &Interner) -> Self {
        let core_name = interner.intern("lona.core");
        let user_name = interner.intern("user");

        let core_ns = Namespace::new(core_name);
        let user_ns = Namespace::new(user_name);

        let mut namespaces = BTreeMap::new();
        let _prev_core = namespaces.insert(core_name, core_ns);
        let _prev_user = namespaces.insert(user_name, user_ns);

        Self {
            namespaces,
            current: user_name,
            core_name,
        }
    }

    /// Returns the `lona.core` namespace name.
    #[inline]
    #[must_use]
    pub const fn core_name(&self) -> symbol::Id {
        self.core_name
    }

    /// Returns a reference to the current namespace.
    ///
    /// Returns `None` only if the registry is in an invalid state (which
    /// should not happen in normal operation since the registry is always
    /// bootstrapped with valid namespaces).
    #[inline]
    #[must_use]
    pub fn current(&self) -> Option<&Namespace> {
        self.namespaces.get(&self.current)
    }

    /// Returns a mutable reference to the current namespace.
    ///
    /// Returns `None` only if the registry is in an invalid state (which
    /// should not happen in normal operation since the registry is always
    /// bootstrapped with valid namespaces).
    #[inline]
    #[must_use]
    pub fn current_mut(&mut self) -> Option<&mut Namespace> {
        self.namespaces.get_mut(&self.current)
    }

    /// Returns the name of the current namespace.
    #[inline]
    #[must_use]
    pub const fn current_name(&self) -> symbol::Id {
        self.current
    }

    /// Gets a reference to a namespace by name, if it exists.
    #[inline]
    #[must_use]
    pub fn get(&self, name: symbol::Id) -> Option<&Namespace> {
        self.namespaces.get(&name)
    }

    /// Gets a mutable reference to a namespace by name, if it exists.
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, name: symbol::Id) -> Option<&mut Namespace> {
        self.namespaces.get_mut(&name)
    }

    /// Gets a mutable reference to a namespace, creating it if it doesn't exist.
    ///
    /// When a new namespace is created, all public vars from `lona.core` are
    /// automatically referred into it (unless the namespace is `lona.core` itself).
    /// This makes core functions available without qualification.
    #[inline]
    pub fn get_or_create(&mut self, name: symbol::Id) -> &mut Namespace {
        // First, collect vars to refer if we're creating a new namespace (not lona.core)
        let vars_to_refer: alloc::vec::Vec<_> =
            if !self.namespaces.contains_key(&name) && name != self.core_name {
                // Collect vars from lona.core before modifying the map
                self.namespaces
                    .get(&self.core_name)
                    .map(|core_ns| {
                        core_ns
                            .mappings()
                            .map(|(sym, var)| (*sym, var.clone()))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                alloc::vec::Vec::new()
            };

        // Use entry API to get or create the namespace
        let ns = self
            .namespaces
            .entry(name)
            .or_insert_with(|| Namespace::new(name));

        // Add refers if we collected any (this happens for newly created namespaces)
        for (sym, var) in vars_to_refer {
            ns.add_refer(sym, var);
        }

        ns
    }

    /// Refers all vars from `lona.core` into the given namespace.
    ///
    /// This is called automatically when namespaces are created, but can also
    /// be called manually to refresh refers after `lona.core` is populated
    /// with new vars (e.g., after registering primitives).
    ///
    /// Note: All core primitives are public, so we don't check `is_private`.
    /// Private var support can be added later when needed.
    #[inline]
    pub fn refer_core_to(&mut self, target_name: symbol::Id) {
        // Don't refer lona.core into itself
        if target_name == self.core_name {
            return;
        }

        // Collect the vars to refer (to avoid borrow conflicts)
        let mut vars_to_refer = alloc::vec::Vec::new();
        if let Some(core_ns) = self.namespaces.get(&self.core_name) {
            for (sym, var) in core_ns.mappings() {
                vars_to_refer.push((*sym, var.clone()));
            }
        }

        // Add the refers to the target namespace
        if let Some(target_ns) = self.namespaces.get_mut(&target_name) {
            for (sym, var) in vars_to_refer {
                target_ns.add_refer(sym, var);
            }
        }
    }

    /// Refers all public vars from `lona.core` into all existing namespaces.
    ///
    /// Call this after populating `lona.core` with primitives to make them
    /// available in all namespaces (including those created before registration).
    #[inline]
    pub fn refer_core_to_all(&mut self) {
        // Collect namespace names first to avoid borrow conflicts
        let ns_names: alloc::vec::Vec<_> = self
            .namespaces
            .keys()
            .filter(|name| **name != self.core_name)
            .copied()
            .collect();

        for ns_name in ns_names {
            self.refer_core_to(ns_name);
        }
    }

    /// Switches the current namespace to the given name.
    ///
    /// If the namespace doesn't exist, it is created first. This ensures
    /// `current` always refers to a valid namespace in the registry.
    #[inline]
    pub fn switch_to(&mut self, name: symbol::Id) {
        // Ensure the namespace exists before updating `current`. We call
        // `get_or_create` rather than just `insert` to avoid overwriting
        // an existing namespace with an empty one.
        let _ns = self.get_or_create(name);
        self.current = name;
    }

    /// Returns an iterator over all namespaces (name → Namespace).
    #[inline]
    pub fn all_namespaces(&self) -> impl Iterator<Item = (&symbol::Id, &Namespace)> {
        self.namespaces.iter()
    }

    /// Returns the number of namespaces in the registry.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.namespaces.len()
    }

    /// Returns true if the registry contains no namespaces.
    ///
    /// Note: This will always return false after construction because
    /// the registry is bootstrapped with `lona.core` and `user`.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.namespaces.is_empty()
    }

    /// Returns true if the registry contains a namespace with the given name.
    #[inline]
    #[must_use]
    pub fn contains(&self, name: symbol::Id) -> bool {
        self.namespaces.contains_key(&name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_bootstrap_creates_core() {
        let interner = Interner::new();
        let registry = Registry::new(&interner);

        let core_name = interner.intern("lona.core");
        assert!(registry.contains(core_name));
        assert!(registry.get(core_name).is_some());
    }

    #[test]
    fn test_registry_bootstrap_creates_user() {
        let interner = Interner::new();
        let registry = Registry::new(&interner);

        let user_name = interner.intern("user");
        assert!(registry.contains(user_name));
        assert!(registry.get(user_name).is_some());
    }

    #[test]
    fn test_registry_current_is_user() {
        let interner = Interner::new();
        let registry = Registry::new(&interner);

        let user_name = interner.intern("user");
        assert_eq!(registry.current_name(), user_name);

        let current = registry.current();
        assert!(current.is_some());
        assert_eq!(current.map(|ns| ns.name()), Some(user_name));
    }

    #[test]
    fn test_registry_switch_to_existing() {
        let interner = Interner::new();
        let mut registry = Registry::new(&interner);

        let core_name = interner.intern("lona.core");
        registry.switch_to(core_name);

        assert_eq!(registry.current_name(), core_name);
        assert_eq!(registry.current().map(|ns| ns.name()), Some(core_name));
    }

    #[test]
    fn test_registry_switch_to_creates_new() {
        let interner = Interner::new();
        let mut registry = Registry::new(&interner);

        let new_ns = interner.intern("my.namespace");
        assert!(!registry.contains(new_ns));

        registry.switch_to(new_ns);

        assert!(registry.contains(new_ns));
        assert_eq!(registry.current_name(), new_ns);
        assert_eq!(registry.current().map(|ns| ns.name()), Some(new_ns));
    }

    #[test]
    fn test_registry_get_or_create_existing() {
        let interner = Interner::new();
        let mut registry = Registry::new(&interner);

        let user_name = interner.intern("user");
        let initial_len = registry.len();

        let ns = registry.get_or_create(user_name);
        assert_eq!(ns.name(), user_name);
        assert_eq!(registry.len(), initial_len);
    }

    #[test]
    fn test_registry_get_or_create_new() {
        let interner = Interner::new();
        let mut registry = Registry::new(&interner);

        let new_ns = interner.intern("new.namespace");
        let initial_len = registry.len();

        let ns = registry.get_or_create(new_ns);
        assert_eq!(ns.name(), new_ns);
        assert_eq!(registry.len(), initial_len.checked_add(1).unwrap());
    }

    #[test]
    fn test_registry_len() {
        let interner = Interner::new();
        let registry = Registry::new(&interner);

        // Bootstrap creates lona.core and user
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_registry_is_not_empty() {
        let interner = Interner::new();
        let registry = Registry::new(&interner);

        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_all_namespaces() {
        let interner = Interner::new();
        let registry = Registry::new(&interner);

        let all: alloc::vec::Vec<_> = registry.all_namespaces().collect();
        assert_eq!(all.len(), 2);

        let core_name = interner.intern("lona.core");
        let user_name = interner.intern("user");

        // Collect owned symbol::Id values for comparison
        let names: alloc::vec::Vec<_> = all.iter().map(|(name, _)| **name).collect();
        assert!(names.contains(&core_name));
        assert!(names.contains(&user_name));
    }

    #[test]
    fn test_registry_get_mut() {
        use lona_core::integer::Integer;
        use lona_core::value::Value;

        let interner = Interner::new();
        let mut registry = Registry::new(&interner);

        let user_name = interner.intern("user");
        let sym = interner.intern("x");
        let value = Value::Integer(Integer::from_i64(42));

        {
            let Some(ns) = registry.get_mut(user_name) else {
                panic!("user namespace should exist after bootstrap");
            };
            let _var = ns.intern(sym, value.clone());
        }

        let lookup_result = registry.get(user_name).and_then(|ns| ns.lookup(sym));
        assert_eq!(lookup_result, Some(value));
    }

    #[test]
    fn test_registry_current_mut() {
        use lona_core::integer::Integer;
        use lona_core::value::Value;

        let interner = Interner::new();
        let mut registry = Registry::new(&interner);

        let sym = interner.intern("y");
        let value = Value::Integer(Integer::from_i64(100));

        {
            let Some(ns) = registry.current_mut() else {
                panic!("current namespace should exist after bootstrap");
            };
            let _var = ns.intern(sym, value.clone());
        }

        let lookup_result = registry.current().and_then(|ns| ns.lookup(sym));
        assert_eq!(lookup_result, Some(value));
    }
}
