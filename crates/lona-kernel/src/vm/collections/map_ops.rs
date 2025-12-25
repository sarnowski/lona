// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Map-related collection operations.
//!
//! Provides native implementations for map manipulation:
//! - `hash-map` - create a map from key-value pairs
//! - `get` - lookup value by key in a map

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::map::Map;
use lona_core::value::{self, Value};

use crate::vm::natives::{NativeContext, NativeError};

/// `(hash-map & kvs)` - create map from key-value pairs.
///
/// Creates a new map from alternating key-value pairs.
/// Requires an even number of arguments.
///
/// # Errors
///
/// Returns an error if an odd number of arguments is provided.
#[inline]
pub fn native_hash_map(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if !args.len().is_multiple_of(2) {
        return Err(NativeError::Error(
            "hash-map requires even number of arguments",
        ));
    }

    // Note: We iterate twice (once for collect, once for from_pairs) due to the
    // error handling pattern. This is acceptable for map literal construction.
    let pairs = args
        .chunks_exact(2_usize)
        .map(|chunk| {
            // chunks_exact(2) guarantees exactly 2 elements per chunk
            let &[ref key, ref value] = chunk else {
                // Cannot happen due to chunks_exact guarantee
                return Err(NativeError::Error("internal error: invalid chunk size"));
            };
            Ok((key.clone(), value.clone()))
        })
        .collect::<Result<alloc::vec::Vec<_>, NativeError>>()?;

    let map = Map::from_pairs(pairs);
    Ok(Value::Map(map))
}

/// `(get map key)` or `(get map key not-found)` - lookup value in map.
///
/// Returns the value mapped to `key`, or `nil` if not found.
/// If `not-found` is provided, returns that value instead of `nil` when key is missing.
/// If `map` is `nil`, returns `nil` (or `not-found` if provided).
///
/// # Lonala-First Justification
///
/// This is a native primitive because:
/// - It requires inspecting runtime type tags to distinguish Map from other types
/// - It requires direct access to the HAMT internals for efficient O(log32 n) lookup
/// - Listed in `docs/architecture/minimal-rust.md` under "Map Operations"
///
/// # Current Limitations
///
/// Currently only supports Map and Nil as the collection argument. In the future,
/// this should be extended to support vectors (indexed access via `nth`), which
/// matches Clojure's polymorphic `get` behavior. When vector support is added,
/// `get` will delegate to `nth` for indexed access.
///
/// # Examples
///
/// ```text
/// (get {:a 1} :a)        ; => 1
/// (get {:a 1} :b)        ; => nil
/// (get {:a 1} :b :default) ; => :default
/// (get nil :a)           ; => nil
/// ```
#[inline]
pub fn native_get(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let (map_value, key, not_found) = match *args {
        [ref map_val, ref key_val] => (map_val, key_val, None),
        [ref map_val, ref key_val, ref not_found_val] => (map_val, key_val, Some(not_found_val)),
        _ => {
            return Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Range {
                    min: 2_u8,
                    max: 3_u8,
                },
                got: u8::try_from(args.len()).unwrap_or(u8::MAX),
            });
        }
    };

    // Default not-found value is nil
    let default = not_found.cloned().unwrap_or(Value::Nil);

    // Handle nil map as empty map (returns default)
    let Value::Map(ref map) = *map_value else {
        if matches!(map_value, Value::Nil) {
            return Ok(default);
        }
        // For now, only maps and nil are supported
        // In the future, we may extend to vectors (get by index)
        return Err(NativeError::TypeError {
            expected: TypeExpectation::Single(value::Kind::Map),
            got: map_value.kind(),
            arg_index: 0_u8,
        });
    };

    // Look up the key in the map
    Ok(map.get(key).cloned().unwrap_or(default))
}
