// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Native function support for the Lonala virtual machine.
//!
//! Provides infrastructure for registering and calling Rust functions
//! from Lonala code. Native functions are used for primitives like
//! `print`, arithmetic operations, and I/O.
//!
//! # Error Handling
//!
//! Native functions use [`NativeError`] for structured error reporting.
//! These errors use typed context (like `value::Kind`) instead of strings,
//! and formatting is centralized in `lonala-human`.

mod arithmetic;
mod comparison;
mod metadata;
mod predicates;

pub use arithmetic::{
    ARITHMETIC_PRIMITIVE_NAMES, intern_arithmetic_primitives, lookup_arithmetic_primitives,
    native_add, native_div, native_mod, native_mul, native_sub, register_arithmetic_primitives,
};
pub use comparison::{
    COMPARISON_PRIMITIVE_NAMES, intern_comparison_primitives, lookup_comparison_primitives,
    native_eq, native_ge, native_gt, native_le, native_lt, register_comparison_primitives,
};
pub use metadata::{
    METADATA_PRIMITIVE_NAMES, intern_metadata_primitives, lookup_metadata_primitives, native_meta,
    native_with_meta, register_metadata_primitives,
};
pub use predicates::{
    TYPE_PREDICATE_NAMES, intern_type_predicates, lookup_type_predicates, native_keyword_p,
    register_type_predicates,
};

use alloc::collections::BTreeMap;

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::symbol::{self, Interner};
use lona_core::value::{self, Value};
use lonala_compiler::MacroRegistry;

/// Context passed to native functions during execution.
///
/// Provides access to VM state that native functions may need, such as
/// the symbol interner for resolving names and the macro registry for
/// introspection functions.
#[non_exhaustive]
pub struct NativeContext<'vm> {
    /// Symbol interner for resolving symbol names.
    interner: &'vm Interner,
    /// Optional macro registry for introspection functions.
    macros: Option<&'vm MacroRegistry>,
}

impl<'vm> NativeContext<'vm> {
    /// Creates a new native context.
    #[inline]
    #[must_use]
    pub const fn new(interner: &'vm Interner, macros: Option<&'vm MacroRegistry>) -> Self {
        Self { interner, macros }
    }

    /// Returns a reference to the symbol interner.
    #[inline]
    #[must_use]
    pub const fn interner(&self) -> &'vm Interner {
        self.interner
    }

    /// Returns a reference to the macro registry, if available.
    #[inline]
    #[must_use]
    pub const fn macros(&self) -> Option<&'vm MacroRegistry> {
        self.macros
    }
}

/// Signature for native functions.
///
/// Native functions receive their arguments as a slice along with
/// a context providing access to VM state. Returns either a value
/// or an error.
///
/// # Arguments
///
/// * `args` - Slice of argument values passed to the function
/// * `ctx` - Context providing access to interner, macro registry, etc.
///
/// # Returns
///
/// The function result or an error.
pub type NativeFn = fn(args: &[Value], ctx: &NativeContext<'_>) -> Result<Value, NativeError>;

/// Errors that can occur in native functions.
///
/// Uses structured types instead of strings for consistent formatting
/// via `lonala-human`. This type does NOT implement `Display`; all
/// formatting is centralized.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeError {
    /// Wrong number of arguments.
    ArityMismatch {
        /// Expected number of arguments.
        expected: ArityExpectation,
        /// Actual number of arguments provided.
        got: u8,
    },
    /// Type error in argument.
    TypeError {
        /// Expected type(s).
        expected: TypeExpectation,
        /// Actual type encountered.
        got: value::Kind,
        /// Zero-based argument index.
        arg_index: u8,
    },
    /// Division or modulo by zero.
    DivisionByZero,
    /// Generic error with message (for cases where structured data isn't available).
    Error(&'static str),
}

impl NativeError {
    /// Returns the variant name for error identification.
    #[inline]
    #[must_use]
    pub const fn variant_name(&self) -> &'static str {
        match *self {
            Self::ArityMismatch { .. } => "ArityMismatch",
            Self::TypeError { .. } => "TypeError",
            Self::DivisionByZero => "DivisionByZero",
            Self::Error(_) => "Error",
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
    fn test_add(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
        if args.len() != 2_usize {
            return Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(2_u8),
                got: u8::try_from(args.len()).unwrap_or(u8::MAX),
            });
        }

        let a_val = args.first().ok_or(NativeError::Error("missing argument"))?;
        let left = a_val.as_integer().ok_or(NativeError::TypeError {
            expected: TypeExpectation::Single(value::Kind::Integer),
            got: a_val.kind(),
            arg_index: 0_u8,
        })?;

        let b_val = args
            .get(1_usize)
            .ok_or(NativeError::Error("missing argument"))?;
        let right = b_val.as_integer().ok_or(NativeError::TypeError {
            expected: TypeExpectation::Single(value::Kind::Integer),
            got: b_val.kind(),
            arg_index: 1_u8,
        })?;

        Ok(Value::Integer(left + right))
    }

    /// Native function that returns nil.
    fn test_nil(_args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
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
        let ctx = NativeContext::new(&interner, None);
        let result = native_fn(&args, &ctx).unwrap();

        assert_eq!(result, Value::Integer(Integer::from_i64(3)));
    }

    #[test]
    fn native_arity_error() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);
        let result = test_add(&[Value::Integer(Integer::from_i64(1))], &ctx);
        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(2_u8),
                got: 1_u8
            })
        ));
    }

    #[test]
    fn native_type_error() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);
        let result = test_add(
            &[Value::Bool(true), Value::Integer(Integer::from_i64(2))],
            &ctx,
        );
        match result {
            Err(NativeError::TypeError {
                expected,
                got,
                arg_index,
            }) => {
                assert_eq!(expected, TypeExpectation::Single(value::Kind::Integer));
                assert_eq!(got, value::Kind::Bool);
                assert_eq!(arg_index, 0_u8);
            }
            _ => panic!("Expected TypeError"),
        }
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
        let ctx = NativeContext::new(&interner, None);
        let result = native_fn(&[], &ctx).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn native_error_variant_name() {
        assert_eq!(
            NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(2_u8),
                got: 3_u8
            }
            .variant_name(),
            "ArityMismatch"
        );
        assert_eq!(
            NativeError::TypeError {
                expected: TypeExpectation::Numeric,
                got: value::Kind::Bool,
                arg_index: 0_u8
            }
            .variant_name(),
            "TypeError"
        );
        assert_eq!(NativeError::DivisionByZero.variant_name(), "DivisionByZero");
        assert_eq!(NativeError::Error("test").variant_name(), "Error");
    }
}
