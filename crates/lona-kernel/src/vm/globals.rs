// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Global variable storage for the virtual machine.

use alloc::collections::BTreeMap;

use lona_core::map::Map;
use lona_core::symbol;
use lona_core::value::{Value, Var};

/// Global variable storage mapping symbols to Vars.
///
/// Uses `BTreeMap` for `no_std` compatibility (no random seed required).
/// Each global is stored as a [`Var`], which holds both the value and
/// metadata (docstrings, source locations, etc.).
#[derive(Debug, Clone, Default)]
pub struct Globals {
    /// Symbol ID to Var mapping.
    vars: BTreeMap<symbol::Id, Var>,
}

impl Globals {
    /// Creates a new empty global variable store.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            vars: BTreeMap::new(),
        }
    }

    /// Gets the value of a global variable (auto-deref).
    ///
    /// Returns `None` if the global is not defined.
    /// For backward compatibility, this returns the value directly,
    /// not the Var. Use [`get_var`](Self::get_var) to get the Var itself.
    #[inline]
    #[must_use]
    pub fn get(&self, symbol: symbol::Id) -> Option<Value> {
        self.vars.get(&symbol).map(Var::value)
    }

    /// Gets the Var itself for a global variable.
    ///
    /// Returns `None` if the global is not defined.
    /// Use this to access var metadata or to pass the var to other code.
    #[inline]
    #[must_use]
    pub fn get_var(&self, symbol: symbol::Id) -> Option<&Var> {
        self.vars.get(&symbol)
    }

    /// Sets the value of a global variable.
    ///
    /// Creates a new Var if the global doesn't exist.
    /// If the global exists, updates only the value (preserves metadata).
    #[inline]
    pub fn set(&mut self, symbol: symbol::Id, value: Value) {
        if let Some(var) = self.vars.get(&symbol) {
            var.set_value(value);
        } else {
            let _previous = self.vars.insert(symbol, Var::new(symbol, value, None));
        }
    }

    /// Sets value and metadata atomically.
    ///
    /// Creates a new Var if the global doesn't exist.
    /// If the global exists, updates the value and merges the metadata.
    #[inline]
    pub fn set_with_meta(&mut self, symbol: symbol::Id, value: Value, meta: Option<Map>) {
        if let Some(var) = self.vars.get(&symbol) {
            var.set_value(value);
            if let Some(new_meta) = meta {
                var.merge_meta(new_meta);
            }
        } else {
            let _previous = self.vars.insert(symbol, Var::new(symbol, value, meta));
        }
    }

    /// Merges metadata into an existing var.
    ///
    /// Does nothing if the global is not defined.
    #[inline]
    pub fn merge_meta(&mut self, symbol: symbol::Id, meta: Map) {
        if let Some(var) = self.vars.get(&symbol) {
            var.merge_meta(meta);
        }
    }

    /// Returns `true` if the global variable is defined.
    #[inline]
    #[must_use]
    pub fn contains(&self, symbol: symbol::Id) -> bool {
        self.vars.contains_key(&symbol)
    }

    /// Returns the number of defined globals.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.vars.len()
    }

    /// Returns `true` if no globals are defined.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::integer::Integer;
    use lona_core::symbol::Interner;

    #[test]
    fn new_globals_is_empty() {
        let globals = Globals::new();
        assert!(globals.is_empty());
        assert_eq!(globals.len(), 0);
    }

    #[test]
    fn set_and_get_global() {
        let interner = Interner::new();
        let sym = interner.intern("x");

        let mut globals = Globals::new();
        globals.set(sym, Value::Integer(Integer::from_i64(42)));

        assert_eq!(
            globals.get(sym),
            Some(Value::Integer(Integer::from_i64(42)))
        );
        assert!(globals.contains(sym));
        assert_eq!(globals.len(), 1);
    }

    #[test]
    fn get_undefined_global_returns_none() {
        let interner = Interner::new();
        let sym = interner.intern("undefined");

        let globals = Globals::new();
        assert_eq!(globals.get(sym), None);
        assert!(!globals.contains(sym));
    }

    #[test]
    fn set_overwrites_existing_global() {
        let interner = Interner::new();
        let sym = interner.intern("x");

        let mut globals = Globals::new();
        globals.set(sym, Value::Integer(Integer::from_i64(1)));
        globals.set(sym, Value::Integer(Integer::from_i64(2)));

        assert_eq!(globals.get(sym), Some(Value::Integer(Integer::from_i64(2))));
        assert_eq!(globals.len(), 1); // Still just one global
    }

    #[test]
    fn multiple_globals() {
        let interner = Interner::new();
        let x = interner.intern("x");
        let y = interner.intern("y");
        let z = interner.intern("z");

        let mut globals = Globals::new();
        globals.set(x, Value::Integer(Integer::from_i64(1)));
        globals.set(y, Value::Integer(Integer::from_i64(2)));
        globals.set(z, Value::Integer(Integer::from_i64(3)));

        assert_eq!(globals.get(x), Some(Value::Integer(Integer::from_i64(1))));
        assert_eq!(globals.get(y), Some(Value::Integer(Integer::from_i64(2))));
        assert_eq!(globals.get(z), Some(Value::Integer(Integer::from_i64(3))));
        assert_eq!(globals.len(), 3);
    }

    #[test]
    fn globals_with_different_value_types() {
        let interner = Interner::new();
        let a = interner.intern("a");
        let b = interner.intern("b");
        let c = interner.intern("c");
        let d = interner.intern("d");

        let mut globals = Globals::new();
        globals.set(a, Value::Nil);
        globals.set(b, Value::Bool(true));
        globals.set(c, Value::Integer(Integer::from_i64(42)));
        globals.set(d, Value::Float(3.14));

        assert_eq!(globals.get(a), Some(Value::Nil));
        assert_eq!(globals.get(b), Some(Value::Bool(true)));
        assert_eq!(globals.get(c), Some(Value::Integer(Integer::from_i64(42))));
        assert_eq!(globals.get(d), Some(Value::Float(3.14)));
    }

    #[test]
    fn default_creates_empty_globals() {
        let globals = Globals::default();
        assert!(globals.is_empty());
    }

    #[test]
    fn get_var_returns_var() {
        let interner = Interner::new();
        let sym = interner.intern("x");

        let mut globals = Globals::new();
        globals.set(sym, Value::Integer(Integer::from_i64(42)));

        let var = globals.get_var(sym);
        assert!(var.is_some());

        let var = var.unwrap();
        assert_eq!(var.name(), sym);
        assert_eq!(var.value(), Value::Integer(Integer::from_i64(42)));
        assert!(var.meta().is_none());
    }

    #[test]
    fn get_var_undefined_returns_none() {
        let interner = Interner::new();
        let sym = interner.intern("undefined");

        let globals = Globals::new();
        assert!(globals.get_var(sym).is_none());
    }

    #[test]
    fn set_preserves_metadata_on_redefine() {
        let interner = Interner::new();
        let sym = interner.intern("x");

        let meta = Map::empty().assoc(Value::from(1_i32), Value::from(10_i32));

        let mut globals = Globals::new();
        globals.set_with_meta(
            sym,
            Value::Integer(Integer::from_i64(1)),
            Some(meta.clone()),
        );

        // Redefine with set() - should preserve metadata
        globals.set(sym, Value::Integer(Integer::from_i64(2)));

        assert_eq!(globals.get(sym), Some(Value::Integer(Integer::from_i64(2))));

        let var = globals.get_var(sym).unwrap();
        assert_eq!(var.meta(), Some(meta));
    }

    #[test]
    fn set_with_meta_creates_new_var() {
        let interner = Interner::new();
        let sym = interner.intern("x");

        let meta = Map::empty().assoc(Value::from(1_i32), Value::from(10_i32));

        let mut globals = Globals::new();
        globals.set_with_meta(
            sym,
            Value::Integer(Integer::from_i64(42)),
            Some(meta.clone()),
        );

        let var = globals.get_var(sym).unwrap();
        assert_eq!(var.value(), Value::Integer(Integer::from_i64(42)));
        assert_eq!(var.meta(), Some(meta));
    }

    #[test]
    fn set_with_meta_merges_on_existing() {
        let interner = Interner::new();
        let sym = interner.intern("x");

        let meta_a = Map::empty().assoc(Value::from(1_i32), Value::from(10_i32));
        let meta_b = Map::empty()
            .assoc(Value::from(2_i32), Value::from(20_i32))
            .assoc(Value::from(1_i32), Value::from(100_i32)); // Overwrites key 1

        let mut globals = Globals::new();
        globals.set_with_meta(sym, Value::Integer(Integer::from_i64(1)), Some(meta_a));
        globals.set_with_meta(sym, Value::Integer(Integer::from_i64(2)), Some(meta_b));

        let var = globals.get_var(sym).unwrap();
        assert_eq!(var.value(), Value::Integer(Integer::from_i64(2)));

        let result = var.meta().unwrap();
        assert_eq!(result.get(&Value::from(1_i32)), Some(&Value::from(100_i32)));
        assert_eq!(result.get(&Value::from(2_i32)), Some(&Value::from(20_i32)));
    }

    #[test]
    fn set_with_meta_none_does_not_clear() {
        let interner = Interner::new();
        let sym = interner.intern("x");

        let meta = Map::empty().assoc(Value::from(1_i32), Value::from(10_i32));

        let mut globals = Globals::new();
        globals.set_with_meta(
            sym,
            Value::Integer(Integer::from_i64(1)),
            Some(meta.clone()),
        );

        // Set with None meta should NOT clear existing metadata
        globals.set_with_meta(sym, Value::Integer(Integer::from_i64(2)), None);

        let var = globals.get_var(sym).unwrap();
        assert_eq!(var.meta(), Some(meta));
    }

    #[test]
    fn merge_meta_on_existing() {
        let interner = Interner::new();
        let sym = interner.intern("x");

        let meta_a = Map::empty().assoc(Value::from(1_i32), Value::from(10_i32));
        let meta_b = Map::empty().assoc(Value::from(2_i32), Value::from(20_i32));

        let mut globals = Globals::new();
        globals.set_with_meta(sym, Value::Integer(Integer::from_i64(42)), Some(meta_a));
        globals.merge_meta(sym, meta_b);

        let var = globals.get_var(sym).unwrap();
        let result = var.meta().unwrap();
        assert_eq!(result.get(&Value::from(1_i32)), Some(&Value::from(10_i32)));
        assert_eq!(result.get(&Value::from(2_i32)), Some(&Value::from(20_i32)));
    }

    #[test]
    fn merge_meta_on_undefined_does_nothing() {
        let interner = Interner::new();
        let sym = interner.intern("undefined");

        let meta = Map::empty().assoc(Value::from(1_i32), Value::from(10_i32));

        let mut globals = Globals::new();
        globals.merge_meta(sym, meta);

        // Should not create a new global
        assert!(!globals.contains(sym));
    }
}
