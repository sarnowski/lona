// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Type predicate native functions (keyword?, etc.).

use lona_core::error_context::ArityExpectation;
use lona_core::symbol;
use lona_core::value::Value;

use super::{NativeContext, NativeError};

/// The names of all type predicate primitives.
pub const TYPE_PREDICATE_NAMES: &[&str] = &["keyword?"];

/// Pre-interns all type predicate primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `TYPE_PREDICATE_NAMES`.
#[inline]
pub fn intern_type_predicates(interner: &mut symbol::Interner) -> alloc::vec::Vec<symbol::Id> {
    TYPE_PREDICATE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up type predicate primitive symbols from an immutable interner.
///
/// This is used when primitives should already be interned (e.g., by the REPL)
/// and we only have an immutable reference to the interner.
///
/// Returns `Some(symbols)` if all primitives are found, `None` otherwise.
#[inline]
#[must_use]
pub fn lookup_type_predicates(interner: &symbol::Interner) -> Option<alloc::vec::Vec<symbol::Id>> {
    TYPE_PREDICATE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all type predicate primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_type_predicates` with
/// the same interner.
///
/// Each type predicate function is registered:
/// - As a native function in the registry (for execution)
/// - As a `NativeFunction` value in globals (for first-class use)
#[inline]
pub fn register_type_predicates(vm: &mut crate::vm::interpreter::Vm<'_>, symbols: &[symbol::Id]) {
    use super::NativeFn;

    let funcs: &[NativeFn] = &[native_keyword_p];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        // Use NativeFunction for first-class function support
        vm.set_global(*sym, Value::NativeFunction(*sym));
    }
}

/// Native implementation of `keyword?` (type predicate).
///
/// Returns true if the argument is a keyword.
#[inline]
pub fn native_keyword_p(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
    Ok(Value::Bool(arg.is_keyword()))
}
