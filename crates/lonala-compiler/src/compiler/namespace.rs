// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Namespace tracking for the compiler.
//!
//! The `NamespaceContext` tracks compile-time namespace state including:
//! - The current namespace being compiled
//! - Namespace aliases (alias -> full namespace name)
//! - Referred symbols (unqualified name -> qualified name)
//!
//! This mirrors the runtime `Namespace` structure but is maintained during
//! compilation for symbol resolution.

use alloc::collections::BTreeMap;

use lona_core::symbol;

/// Compile-time namespace context.
///
/// Tracks the current namespace and its alias/refer mappings for symbol
/// resolution during compilation. Changes to this context affect how
/// subsequent symbols are resolved.
///
/// # Symbol Resolution
///
/// When the compiler encounters a symbol, it uses this context to resolve it:
///
/// 1. **Local/Upvalue**: Not handled here (compiler's local environment)
/// 2. **Qualified (`alias/name`)**: Look up `alias` in aliases map
/// 3. **Referred (`name`)**: Check if symbol is in refers map
/// 4. **Unqualified global**: Qualify with current namespace
#[non_exhaustive]
pub struct Context {
    /// The current namespace name (e.g., `user`, `my.app`).
    current: symbol::Id,
    /// Maps alias symbols to full namespace names.
    ///
    /// For example, after `(:require [some.long.namespace :as short])`,
    /// this would contain `short -> some.long.namespace`.
    aliases: BTreeMap<symbol::Id, symbol::Id>,
    /// Maps unqualified symbol names to qualified symbols.
    ///
    /// For example, after `(:require [some.ns :refer [foo]])`,
    /// this would contain `foo -> some.ns/foo`.
    refers: BTreeMap<symbol::Id, symbol::Id>,
    /// Namespaces marked for `:use` (refer-all).
    ///
    /// These namespaces should have all their public symbols referred
    /// when they are loaded. Actual loading is deferred to Task 1.3.4.
    pending_uses: alloc::vec::Vec<symbol::Id>,
}

impl Context {
    /// Creates a new namespace context with the given default namespace.
    ///
    /// The interner is used to intern the default namespace name.
    #[inline]
    #[must_use]
    pub const fn new(default_ns: symbol::Id) -> Self {
        Self {
            current: default_ns,
            aliases: BTreeMap::new(),
            refers: BTreeMap::new(),
            pending_uses: alloc::vec::Vec::new(),
        }
    }

    /// Returns the current namespace symbol.
    #[inline]
    #[must_use]
    pub const fn current(&self) -> symbol::Id {
        self.current
    }

    /// Sets the current namespace.
    ///
    /// This is called when an `(ns ...)` form is compiled.
    #[inline]
    pub const fn set_current(&mut self, ns: symbol::Id) {
        self.current = ns;
    }

    /// Adds an alias mapping.
    ///
    /// After this call, `alias` will resolve to `namespace` when used
    /// as the namespace part of a qualified symbol (e.g., `alias/name`).
    ///
    /// If an alias with the same name already exists, it is silently
    /// overwritten. This matches Clojure's behavior where re-requiring
    /// with a different alias updates the mapping.
    #[inline]
    pub fn add_alias(&mut self, alias: symbol::Id, namespace: symbol::Id) {
        let _previous = self.aliases.insert(alias, namespace);
    }

    /// Resolves an alias to its full namespace name.
    ///
    /// Returns `Some(namespace)` if the alias was registered,
    /// `None` otherwise.
    #[inline]
    #[must_use]
    pub fn resolve_alias(&self, alias: symbol::Id) -> Option<symbol::Id> {
        self.aliases.get(&alias).copied()
    }

    /// Adds a refer mapping.
    ///
    /// After this call, the unqualified `symbol` will resolve to
    /// `qualified` (e.g., `some.ns/symbol`).
    ///
    /// If a refer for the same symbol already exists, it is silently
    /// overwritten. This matches Clojure's behavior where re-requiring
    /// with different refers updates the mapping.
    #[inline]
    pub fn add_refer(&mut self, symbol: symbol::Id, qualified: symbol::Id) {
        let _previous = self.refers.insert(symbol, qualified);
    }

    /// Resolves a refer to its qualified symbol.
    ///
    /// Returns `Some(qualified)` if the symbol was referred,
    /// `None` otherwise.
    #[inline]
    #[must_use]
    pub fn resolve_refer(&self, symbol: symbol::Id) -> Option<symbol::Id> {
        self.refers.get(&symbol).copied()
    }

    /// Clears aliases, refers, and pending uses when switching to a new namespace.
    ///
    /// Called at the start of an `(ns ...)` form to reset the context
    /// for the new namespace. The current namespace is set separately
    /// via `set_current`.
    #[inline]
    pub fn clear_mappings(&mut self) {
        self.aliases.clear();
        self.refers.clear();
        self.pending_uses.clear();
    }

    /// Adds a namespace to the pending `:use` list.
    ///
    /// Namespaces in this list should have all their public symbols referred
    /// when they are loaded. Actual loading is deferred to Task 1.3.4.
    #[inline]
    pub fn add_pending_use(&mut self, namespace: symbol::Id) {
        self.pending_uses.push(namespace);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create symbol IDs for testing.
    fn intern(interner: &symbol::Interner, name: &str) -> symbol::Id {
        interner.intern(name)
    }

    #[test]
    fn new_context_has_default_namespace() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let ctx = Context::new(user);

        assert_eq!(ctx.current(), user);
    }

    #[test]
    fn set_current_changes_namespace() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let my_app = intern(&interner, "my.app");

        let mut ctx = Context::new(user);
        ctx.set_current(my_app);

        assert_eq!(ctx.current(), my_app);
    }

    #[test]
    fn add_alias_and_resolve() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let short = intern(&interner, "short");
        let full_ns = intern(&interner, "some.long.namespace");

        let mut ctx = Context::new(user);
        ctx.add_alias(short, full_ns);

        assert_eq!(ctx.resolve_alias(short), Some(full_ns));
    }

    #[test]
    fn resolve_unknown_alias_returns_none() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let unknown = intern(&interner, "unknown");

        let ctx = Context::new(user);

        assert_eq!(ctx.resolve_alias(unknown), None);
    }

    #[test]
    fn add_refer_and_resolve() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let foo = intern(&interner, "foo");
        let qualified = intern(&interner, "some.ns/foo");

        let mut ctx = Context::new(user);
        ctx.add_refer(foo, qualified);

        assert_eq!(ctx.resolve_refer(foo), Some(qualified));
    }

    #[test]
    fn resolve_unknown_refer_returns_none() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let unknown = intern(&interner, "unknown");

        let ctx = Context::new(user);

        assert_eq!(ctx.resolve_refer(unknown), None);
    }

    #[test]
    fn clear_mappings_removes_aliases_and_refers() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let alias = intern(&interner, "alias");
        let ns = intern(&interner, "some.ns");
        let sym = intern(&interner, "sym");
        let qualified = intern(&interner, "some.ns/sym");

        let mut ctx = Context::new(user);
        ctx.add_alias(alias, ns);
        ctx.add_refer(sym, qualified);

        // Verify mappings exist
        assert!(ctx.resolve_alias(alias).is_some());
        assert!(ctx.resolve_refer(sym).is_some());

        // Clear and verify they're gone
        ctx.clear_mappings();
        assert!(ctx.resolve_alias(alias).is_none());
        assert!(ctx.resolve_refer(sym).is_none());
    }

    #[test]
    fn clear_mappings_preserves_current_namespace() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let my_app = intern(&interner, "my.app");

        let mut ctx = Context::new(user);
        ctx.set_current(my_app);
        ctx.clear_mappings();

        assert_eq!(ctx.current(), my_app);
    }

    #[test]
    fn multiple_aliases_work_independently() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let a = intern(&interner, "a");
        let b = intern(&interner, "b");
        let ns_a = intern(&interner, "namespace.a");
        let ns_b = intern(&interner, "namespace.b");

        let mut ctx = Context::new(user);
        ctx.add_alias(a, ns_a);
        ctx.add_alias(b, ns_b);

        assert_eq!(ctx.resolve_alias(a), Some(ns_a));
        assert_eq!(ctx.resolve_alias(b), Some(ns_b));
    }

    #[test]
    fn alias_can_be_overwritten() {
        let interner = symbol::Interner::new();
        let user = intern(&interner, "user");
        let alias = intern(&interner, "x");
        let ns1 = intern(&interner, "first.ns");
        let ns2 = intern(&interner, "second.ns");

        let mut ctx = Context::new(user);
        ctx.add_alias(alias, ns1);
        assert_eq!(ctx.resolve_alias(alias), Some(ns1));

        ctx.add_alias(alias, ns2);
        assert_eq!(ctx.resolve_alias(alias), Some(ns2));
    }
}
