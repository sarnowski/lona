// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Collection primitive functions for the Lonala language.
//!
//! Provides native implementations of core collection operations:
//! - `cons` - prepend element to a collection
//! - `first` - get first element of a collection
//! - `rest` - get rest of a collection (tail)
//! - `vector` - create a vector from arguments
//! - `hash-map` - create a map from key-value pairs
//! - `get` - lookup value by key in a map
//! - `list` - create a list from arguments
//! - `concat` - concatenate sequences into a list
//! - `vec` - convert collection to vector
//!
//! # Registration Pattern
//!
//! These primitives use a two-phase registration pattern to avoid borrow conflicts:
//!
//! 1. Call [`intern_primitives`] with `&Interner` to intern symbol names
//! 2. Create the VM with `Vm::new(&interner)` (immutable borrow)
//! 3. Call [`register_primitives`] with the VM and the symbols from step 1
//!
//! This pattern is necessary because the VM holds an immutable reference to the
//! interner, preventing mutable access during registration.

use lona_core::symbol;
use lona_core::value::Value;

use super::interpreter::Vm;

mod list_ops;
mod map_ops;
mod set_ops;
mod vector_ops;

pub use list_ops::{native_concat, native_cons, native_first, native_list, native_rest};
pub use map_ops::{native_get, native_hash_map};
pub use set_ops::{
    native_conj, native_contains_p, native_count, native_disj, native_hash_set, native_set_p,
};
pub use vector_ops::{native_vec, native_vector};

#[cfg(test)]
mod tests;

/// The names of all collection primitives.
pub const PRIMITIVE_NAMES: &[&str] = &[
    "cons",
    "first",
    "rest",
    "vector",
    "hash-map",
    "list",
    "concat",
    "vec",
    "hash-set",
    "disj",
    "set?",
    "conj",
    "contains?",
    "count",
    "get",
];

/// Pre-interns all collection primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `PRIMITIVE_NAMES`.
#[inline]
pub fn intern_primitives(interner: &symbol::Interner) -> alloc::vec::Vec<symbol::Id> {
    PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up collection primitive symbols from an immutable interner.
///
/// This is used when primitives should already be interned (e.g., by the REPL)
/// and we only have an immutable reference to the interner.
///
/// Returns `Some(symbols)` if all primitives are found, `None` otherwise.
#[inline]
#[must_use]
pub fn lookup_primitives(interner: &symbol::Interner) -> Option<alloc::vec::Vec<symbol::Id>> {
    PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all collection primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_primitives` with the same interner.
///
/// Primitives are registered in the `lona.core` namespace and in the native
/// function registry. Call [`vm.namespace_registry_mut().refer_core_to_all()`]
/// after registering all primitives to propagate them to other namespaces.
///
/// [`vm.namespace_registry_mut().refer_core_to_all()`]: crate::namespace::Registry::refer_core_to_all
#[inline]
pub fn register_primitives(vm: &mut Vm<'_>, symbols: &[symbol::Id]) {
    let funcs: &[super::natives::NativeFn] = &[
        native_cons,
        native_first,
        native_rest,
        native_vector,
        native_hash_map,
        native_list,
        native_concat,
        native_vec,
        native_hash_set,
        native_disj,
        native_set_p,
        native_conj,
        native_contains_p,
        native_count,
        native_get,
    ];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        vm.register_core_primitive(*sym, Value::NativeFunction(*sym));
    }
}
