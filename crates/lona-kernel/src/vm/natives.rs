// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Native function support for the Lonala virtual machine.
//!
//! Provides infrastructure for registering and calling Rust functions
//! from Lonala code. Native functions are used for primitives like
//! `print`, arithmetic operations, and I/O.

use alloc::collections::BTreeMap;

use core::fmt::{self, Display};

use lona_core::symbol::{self, Interner};
use lona_core::value::Value;

/// Signature for native functions.
///
/// Native functions receive their arguments as a slice along with
/// the symbol interner for resolving symbol names. Returns either
/// a value or an error.
///
/// # Arguments
///
/// * `args` - Slice of argument values passed to the function
/// * `interner` - Symbol interner for resolving symbol names
///
/// # Returns
///
/// The function result or an error.
pub type NativeFn = fn(args: &[Value], interner: &Interner) -> Result<Value, NativeError>;

/// Errors that can occur in native functions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeError {
    /// Wrong number of arguments.
    ArityMismatch {
        /// Expected number of arguments.
        expected: usize,
        /// Actual number of arguments provided.
        got: usize,
    },
    /// Type error in argument.
    TypeError {
        /// Expected type name.
        expected: &'static str,
        /// Actual type name.
        got: &'static str,
        /// Zero-based argument index.
        arg_index: usize,
    },
    /// Generic error with message.
    Error(&'static str),
}

impl Display for NativeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::ArityMismatch { expected, got } => {
                write!(f, "expected {expected} arguments, got {got}")
            }
            Self::TypeError {
                expected,
                got,
                arg_index,
            } => {
                write!(
                    f,
                    "argument {}: expected {expected}, got {got}",
                    arg_index.saturating_add(1)
                )
            }
            Self::Error(msg) => write!(f, "{msg}"),
        }
    }
}

/// Registry mapping symbol IDs to native function implementations.
///
/// Used by the VM to look up native functions when executing `Call`
/// instructions on global function symbols.
#[non_exhaustive]
pub struct Registry {
    /// Map from symbol ID to native function.
    functions: BTreeMap<symbol::Id, NativeFn>,
}

impl Registry {
    /// Creates a new empty native function registry.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            functions: BTreeMap::new(),
        }
    }

    /// Registers a native function for a symbol.
    ///
    /// If a function was already registered for this symbol, it is replaced.
    #[inline]
    pub fn register(&mut self, symbol: symbol::Id, func: NativeFn) {
        let _previous = self.functions.insert(symbol, func);
    }

    /// Looks up a native function by symbol ID.
    ///
    /// Returns `Some(func)` if a native function is registered for the symbol,
    /// `None` otherwise.
    #[inline]
    #[must_use]
    pub fn get(&self, symbol: symbol::Id) -> Option<NativeFn> {
        self.functions.get(&symbol).copied()
    }

    /// Returns the number of registered native functions.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Returns `true` if no native functions are registered.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

impl Default for Registry {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::integer::Integer;
    use lona_core::symbol::Interner;

    /// Simple native function for testing.
    fn test_add(args: &[Value], _interner: &Interner) -> Result<Value, NativeError> {
        if args.len() != 2_usize {
            return Err(NativeError::ArityMismatch {
                expected: 2,
                got: args.len(),
            });
        }

        let a = args
            .first()
            .and_then(Value::as_integer)
            .ok_or(NativeError::TypeError {
                expected: "integer",
                got: "non-integer",
                arg_index: 0,
            })?;

        let b = args
            .get(1_usize)
            .and_then(Value::as_integer)
            .ok_or(NativeError::TypeError {
                expected: "integer",
                got: "non-integer",
                arg_index: 1,
            })?;

        Ok(Value::Integer(a + b))
    }

    /// Native function that returns nil.
    fn test_nil(_args: &[Value], _interner: &Interner) -> Result<Value, NativeError> {
        Ok(Value::Nil)
    }

    #[test]
    fn register_and_get_native() {
        let mut interner = Interner::new();
        let mut registry = Registry::new();

        let add_sym = interner.intern("add");
        registry.register(add_sym, test_add);

        assert!(registry.get(add_sym).is_some());
    }

    #[test]
    fn get_unknown_symbol_returns_none() {
        let mut interner = Interner::new();
        let registry = Registry::new();

        let unknown_sym = interner.intern("unknown");
        assert!(registry.get(unknown_sym).is_none());
    }

    #[test]
    fn call_registered_native() {
        let mut interner = Interner::new();
        let mut registry = Registry::new();

        let add_sym = interner.intern("add");
        registry.register(add_sym, test_add);

        let native_fn = registry.get(add_sym).unwrap();
        let args = [
            Value::Integer(Integer::from_i64(1)),
            Value::Integer(Integer::from_i64(2)),
        ];
        let result = native_fn(&args, &interner).unwrap();

        assert_eq!(result, Value::Integer(Integer::from_i64(3)));
    }

    #[test]
    fn native_arity_error() {
        let interner = Interner::new();
        let result = test_add(&[Value::Integer(Integer::from_i64(1))], &interner);
        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: 2,
                got: 1
            })
        ));
    }

    #[test]
    fn native_type_error() {
        let interner = Interner::new();
        let result = test_add(
            &[Value::Bool(true), Value::Integer(Integer::from_i64(2))],
            &interner,
        );
        assert!(matches!(
            result,
            Err(NativeError::TypeError {
                expected: "integer",
                arg_index: 0,
                ..
            })
        ));
    }

    #[test]
    fn registry_len_and_is_empty() {
        let mut interner = Interner::new();
        let mut registry = Registry::new();

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        let sym = interner.intern("test");
        registry.register(sym, test_nil);

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn register_replaces_existing() {
        let mut interner = Interner::new();
        let mut registry = Registry::new();

        let sym = interner.intern("test");
        registry.register(sym, test_add);
        registry.register(sym, test_nil);

        // Should have replaced, not added
        assert_eq!(registry.len(), 1);

        // Should use the new function
        let native_fn = registry.get(sym).unwrap();
        let result = native_fn(&[], &interner).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn native_error_display() {
        let err = NativeError::ArityMismatch {
            expected: 2,
            got: 3,
        };
        let msg = alloc::format!("{err}");
        assert!(msg.contains("expected 2"));
        assert!(msg.contains("got 3"));

        let err = NativeError::TypeError {
            expected: "integer",
            got: "boolean",
            arg_index: 0,
        };
        let msg = alloc::format!("{err}");
        assert!(msg.contains("argument 1"));
        assert!(msg.contains("integer"));
        assert!(msg.contains("boolean"));

        let err = NativeError::Error("custom error");
        let msg = alloc::format!("{err}");
        assert!(msg.contains("custom error"));
    }
}
