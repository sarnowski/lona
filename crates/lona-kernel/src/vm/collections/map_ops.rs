// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Map-related collection operations.
//!
//! Provides native implementations for map manipulation:
//! - `hash-map` - create a map from key-value pairs

use alloc::vec::Vec;

use lona_core::map::Map;
use lona_core::value::Value;

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
        .collect::<Result<Vec<_>, NativeError>>()?;

    let map = Map::from_pairs(pairs);
    Ok(Value::Map(map))
}
