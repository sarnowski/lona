// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Global variable storage for the virtual machine.

use alloc::collections::BTreeMap;

use lona_core::symbol;
use lona_core::value::Value;

/// Global variable storage mapping symbols to values.
///
/// Uses `BTreeMap` for `no_std` compatibility (no random seed required).
#[derive(Debug, Clone, Default)]
pub struct Globals {
    /// Symbol ID to value mapping.
    values: BTreeMap<symbol::Id, Value>,
}

impl Globals {
    /// Creates a new empty global variable store.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    /// Gets the value of a global variable.
    ///
    /// Returns `None` if the global is not defined.
    #[inline]
    #[must_use]
    pub fn get(&self, symbol: symbol::Id) -> Option<Value> {
        self.values.get(&symbol).cloned()
    }

    /// Sets the value of a global variable.
    ///
    /// Creates the global if it doesn't exist, or updates it if it does.
    #[inline]
    pub fn set(&mut self, symbol: symbol::Id, value: Value) {
        let _previous = self.values.insert(symbol, value);
    }

    /// Returns `true` if the global variable is defined.
    #[inline]
    #[must_use]
    pub fn contains(&self, symbol: symbol::Id) -> bool {
        self.values.contains_key(&symbol)
    }

    /// Returns the number of defined globals.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns `true` if no globals are defined.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::symbol::Interner;

    #[test]
    fn new_globals_is_empty() {
        let globals = Globals::new();
        assert!(globals.is_empty());
        assert_eq!(globals.len(), 0);
    }

    #[test]
    fn set_and_get_global() {
        let mut interner = Interner::new();
        let sym = interner.intern("x");

        let mut globals = Globals::new();
        globals.set(sym, Value::Integer(42));

        assert_eq!(globals.get(sym), Some(Value::Integer(42)));
        assert!(globals.contains(sym));
        assert_eq!(globals.len(), 1);
    }

    #[test]
    fn get_undefined_global_returns_none() {
        let mut interner = Interner::new();
        let sym = interner.intern("undefined");

        let globals = Globals::new();
        assert_eq!(globals.get(sym), None);
        assert!(!globals.contains(sym));
    }

    #[test]
    fn set_overwrites_existing_global() {
        let mut interner = Interner::new();
        let sym = interner.intern("x");

        let mut globals = Globals::new();
        globals.set(sym, Value::Integer(1));
        globals.set(sym, Value::Integer(2));

        assert_eq!(globals.get(sym), Some(Value::Integer(2)));
        assert_eq!(globals.len(), 1); // Still just one global
    }

    #[test]
    fn multiple_globals() {
        let mut interner = Interner::new();
        let x = interner.intern("x");
        let y = interner.intern("y");
        let z = interner.intern("z");

        let mut globals = Globals::new();
        globals.set(x, Value::Integer(1));
        globals.set(y, Value::Integer(2));
        globals.set(z, Value::Integer(3));

        assert_eq!(globals.get(x), Some(Value::Integer(1)));
        assert_eq!(globals.get(y), Some(Value::Integer(2)));
        assert_eq!(globals.get(z), Some(Value::Integer(3)));
        assert_eq!(globals.len(), 3);
    }

    #[test]
    fn globals_with_different_value_types() {
        let mut interner = Interner::new();
        let a = interner.intern("a");
        let b = interner.intern("b");
        let c = interner.intern("c");
        let d = interner.intern("d");

        let mut globals = Globals::new();
        globals.set(a, Value::Nil);
        globals.set(b, Value::Bool(true));
        globals.set(c, Value::Integer(42));
        globals.set(d, Value::Float(3.14));

        assert_eq!(globals.get(a), Some(Value::Nil));
        assert_eq!(globals.get(b), Some(Value::Bool(true)));
        assert_eq!(globals.get(c), Some(Value::Integer(42)));
        assert_eq!(globals.get(d), Some(Value::Float(3.14)));
    }

    #[test]
    fn default_creates_empty_globals() {
        let globals = Globals::default();
        assert!(globals.is_empty());
    }
}
