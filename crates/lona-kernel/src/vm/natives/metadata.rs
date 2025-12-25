// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Metadata native functions (meta, with-meta).

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::meta::Meta as _;
use lona_core::symbol;
use lona_core::value::{self, Value};

use super::{NativeContext, NativeError};

/// The names of all metadata primitives.
pub const METADATA_PRIMITIVE_NAMES: &[&str] = &["meta", "with-meta"];

/// Pre-interns all metadata primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `METADATA_PRIMITIVE_NAMES`.
#[inline]
pub fn intern_metadata_primitives(interner: &symbol::Interner) -> alloc::vec::Vec<symbol::Id> {
    METADATA_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up metadata primitive symbols from an immutable interner.
///
/// This is used when primitives should already be interned (e.g., by the REPL)
/// and we only have an immutable reference to the interner.
///
/// Returns `Some(symbols)` if all primitives are found, `None` otherwise.
#[inline]
#[must_use]
pub fn lookup_metadata_primitives(
    interner: &symbol::Interner,
) -> Option<alloc::vec::Vec<symbol::Id>> {
    METADATA_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all metadata primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_metadata_primitives` with
/// the same interner.
///
/// Each metadata function is registered:
/// - As a native function in the registry (for execution)
/// - As a `NativeFunction` value in globals (for first-class use)
#[inline]
pub fn register_metadata_primitives(
    vm: &mut crate::vm::interpreter::Vm<'_>,
    symbols: &[symbol::Id],
) {
    use super::NativeFn;

    let funcs: &[NativeFn] = &[native_meta, native_with_meta];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        // Use NativeFunction for first-class function support
        vm.set_global(*sym, Value::NativeFunction(*sym));
    }
}

/// Native implementation of `meta` (get metadata).
///
/// Returns the metadata map attached to a value, or nil if no metadata.
///
/// - `(meta obj)` → metadata map or nil
#[inline]
pub fn native_meta(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let arg = args.first().ok_or(NativeError::Error("missing argument"))?;

    // Return metadata as a map, or nil if no metadata
    let metadata = match *arg {
        Value::List(ref list) => list.meta().cloned(),
        Value::Vector(ref vec) => vec.meta().cloned(),
        Value::Map(ref map) => map.meta().cloned(),
        Value::Set(ref set) => set.meta().cloned(),
        Value::Symbol(ref sym) => sym.meta().cloned(),
        Value::Var(ref var) => var.meta(),
        // Types that don't support metadata return nil (wildcard covers future variants)
        Value::Nil
        | Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Keyword(_)
        | Value::String(_)
        | Value::Binary(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | _ => None,
    };

    Ok(metadata.map_or(Value::Nil, Value::Map))
}

/// Native implementation of `with-meta` (attach metadata).
///
/// Returns a new object with the given metadata attached.
/// The second argument must be a map or nil.
///
/// - `(with-meta obj map)` → obj with metadata replaced
/// - `(with-meta obj nil)` → obj with metadata cleared
#[inline]
pub fn native_with_meta(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 2_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(2_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let obj = args.first().ok_or(NativeError::Error("missing argument"))?;
    let meta_arg = args
        .get(1_usize)
        .ok_or(NativeError::Error("missing argument"))?;

    // Parse the metadata argument
    let meta = match *meta_arg {
        Value::Nil => None,
        Value::Map(ref map) => Some(map.clone()),
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::Keyword(_)
        | Value::String(_)
        | Value::List(_)
        | Value::Vector(_)
        | Value::Set(_)
        | Value::Binary(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | _ => {
            return Err(NativeError::TypeError {
                expected: TypeExpectation::Either(value::Kind::Map, value::Kind::Nil),
                got: meta_arg.kind(),
                arg_index: 1_u8,
            });
        }
    };

    // Apply metadata to the object
    match obj.clone() {
        Value::List(list) => Ok(Value::List(list.with_meta(meta))),
        Value::Vector(vec) => Ok(Value::Vector(vec.with_meta(meta))),
        Value::Map(map) => Ok(Value::Map(map.with_meta(meta))),
        Value::Set(set) => Ok(Value::Set(set.with_meta(meta))),
        Value::Symbol(sym) => Ok(Value::Symbol(sym.with_meta(meta))),
        Value::Nil
        | Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Keyword(_)
        | Value::String(_)
        | Value::Binary(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | _ => Err(NativeError::TypeError {
            expected: TypeExpectation::MetaSupporting,
            got: obj.kind(),
            arg_index: 0_u8,
        }),
    }
}
