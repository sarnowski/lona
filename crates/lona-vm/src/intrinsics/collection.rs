// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Collection intrinsics (tuple, map, list operations).
//!
//! This module provides both intrinsic functions (called from bytecode) and
//! core functions (pure lookup logic) that can be reused by callable data
//! structures.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;

use super::IntrinsicError;
use super::arithmetic::values_equal;

/// Error from core collection operations.
///
/// These are internal errors that callers convert to appropriate runtime errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreCollectionError {
    /// Value was not the expected collection type.
    NotAMap,
    /// Value was not a tuple.
    NotATuple,
    /// Index was not an integer.
    InvalidIndex,
    /// Index out of bounds.
    IndexOutOfBounds {
        /// The index that was requested.
        index: i64,
        /// The length of the collection.
        len: usize,
    },
    /// Memory read failed.
    OutOfMemory,
}

// --- Core functions (pure lookup logic, reusable by callable data structures) ---

/// Core map lookup - returns value for key, or default if not found.
///
/// This is the pure lookup logic extracted from `intrinsic_get`.
/// Used by both the `get` intrinsic and keyword/map callables.
///
/// # Arguments
/// * `proc` - Process for heap access
/// * `mem` - Memory space
/// * `map_val` - The map to search (must be `Value::Map`)
/// * `key` - The key to look up
/// * `default` - Value to return if key not found
///
/// # Errors
/// Returns `CoreCollectionError::NotAMap` if `map_val` is not a map.
pub fn core_get<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    map_val: Value,
    key: Value,
    default: Value,
) -> Result<Value, CoreCollectionError> {
    let Value::Map(_) = map_val else {
        return Err(CoreCollectionError::NotAMap);
    };

    let map = proc
        .read_map(mem, map_val)
        .ok_or(CoreCollectionError::OutOfMemory)?;

    // Search the association list for the key
    let mut current = map.entries;
    while let Some(pair) = proc.read_pair(mem, current) {
        // Each pair.first is a [key value] tuple
        if let Some(entry_key) = proc.read_tuple_element(mem, pair.first, 0) {
            if values_equal(entry_key, key, proc, mem) {
                // Found the key, return the value
                return proc
                    .read_tuple_element(mem, pair.first, 1)
                    .ok_or(CoreCollectionError::OutOfMemory);
            }
        }
        current = pair.rest;
    }

    // Key not found, return default
    Ok(default)
}

/// Core tuple/indexed access - returns element at index, or default/error on OOB.
///
/// This is the pure index logic extracted from `intrinsic_nth`.
/// Used by both the `nth` intrinsic and tuple callables.
///
/// # Arguments
/// * `proc` - Process for heap access
/// * `mem` - Memory space
/// * `coll` - The collection (must be `Value::Tuple`)
/// * `idx` - The index to access (must be `Value::Int`)
/// * `default` - If `Some`, return this on out-of-bounds. If `None`, return error.
///
/// # Errors
/// - `CoreCollectionError::NotATuple` if `coll` is not a tuple
/// - `CoreCollectionError::InvalidIndex` if `idx` is not an integer
/// - `CoreCollectionError::IndexOutOfBounds` if index is out of range and no default
pub fn core_nth<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    coll: Value,
    idx: Value,
    default: Option<Value>,
) -> Result<Value, CoreCollectionError> {
    let Value::Int(idx_i64) = idx else {
        return Err(CoreCollectionError::InvalidIndex);
    };

    let Value::Tuple(_) = coll else {
        return Err(CoreCollectionError::NotATuple);
    };

    let len = proc
        .read_tuple_len(mem, coll)
        .ok_or(CoreCollectionError::OutOfMemory)?;

    // Convert to usize and check bounds
    let idx_usize = usize::try_from(idx_i64).ok().filter(|&i| i < len);

    idx_usize.map_or_else(
        || {
            // Out of bounds - return default or error
            default.ok_or(CoreCollectionError::IndexOutOfBounds {
                index: idx_i64,
                len,
            })
        },
        |i| {
            proc.read_tuple_element(mem, coll, i)
                .ok_or(CoreCollectionError::OutOfMemory)
        },
    )
}

// --- Tuple intrinsics ---

pub const fn intrinsic_is_tuple(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_tuple())
}

/// Get element at index from a tuple.
///
/// `(nth tuple index)` - returns element or errors on OOB
/// `(nth tuple index not-found)` - returns element or not-found on OOB
pub fn intrinsic_nth<M: MemorySpace>(
    proc: &Process,
    argc: u8,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let coll = proc.x_regs[1];
    let idx = proc.x_regs[2];
    let default = if argc >= 3 {
        Some(proc.x_regs[3])
    } else {
        None
    };

    core_nth(proc, mem, coll, idx, default).map_err(|e| match e {
        CoreCollectionError::NotATuple => IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "tuple",
        },
        CoreCollectionError::InvalidIndex => IntrinsicError::TypeError {
            intrinsic: id,
            arg: 1,
            expected: "integer",
        },
        CoreCollectionError::IndexOutOfBounds { index, len } => {
            IntrinsicError::IndexOutOfBounds { index, len }
        }
        CoreCollectionError::OutOfMemory | CoreCollectionError::NotAMap => {
            IntrinsicError::OutOfMemory
        }
    })
}

pub fn intrinsic_count<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let coll = proc.x_regs[1];

    match coll {
        Value::Nil => Ok(Value::int(0)),
        Value::Tuple(_) => {
            let len = proc
                .read_tuple_len(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            let len_i64 = i64::try_from(len).map_err(|_| IntrinsicError::Overflow)?;
            Ok(Value::int(len_i64))
        }
        Value::Pair(_) => {
            // Count list length
            let mut count: i64 = 0;
            let mut current = coll;
            while let Some(pair) = proc.read_pair(mem, current) {
                count += 1;
                current = pair.rest;
            }
            Ok(Value::int(count))
        }
        Value::String(_) => {
            let s = proc
                .read_string(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            let len_i64 = i64::try_from(s.len()).map_err(|_| IntrinsicError::Overflow)?;
            Ok(Value::int(len_i64))
        }
        Value::Map(_) => {
            // Count map entries
            let map = proc
                .read_map(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            let mut count: i64 = 0;
            let mut current = map.entries;
            while let Some(pair) = proc.read_pair(mem, current) {
                count += 1;
                current = pair.rest;
            }
            Ok(Value::int(count))
        }
        _ => Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "collection",
        }),
    }
}

// --- Symbol intrinsic ---

pub const fn intrinsic_is_symbol(proc: &Process) -> Value {
    Value::bool(matches!(proc.x_regs[1], Value::Symbol(_)))
}

// --- Map intrinsics ---

pub const fn intrinsic_is_map(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_map())
}

/// Get value from map by key.
///
/// `(get m k)` - returns value or nil
/// `(get m k default)` - returns value or default
pub fn intrinsic_get<M: MemorySpace>(
    proc: &Process,
    argc: u8,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let map_val = proc.x_regs[1];
    let key = proc.x_regs[2];
    let default = if argc >= 3 {
        proc.x_regs[3]
    } else {
        Value::Nil
    };

    core_get(proc, mem, map_val, key, default).map_err(|e| match e {
        CoreCollectionError::NotAMap => IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "map",
        },
        CoreCollectionError::OutOfMemory
        | CoreCollectionError::NotATuple
        | CoreCollectionError::InvalidIndex
        | CoreCollectionError::IndexOutOfBounds { .. } => IntrinsicError::OutOfMemory,
    })
}

/// Put key-value pair into map (persistent).
///
/// `(put m k v)` - returns new map with k->v added/updated
pub fn intrinsic_put<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let map_val = proc.x_regs[1];
    let key = proc.x_regs[2];
    let value = proc.x_regs[3];

    let Value::Map(_) = map_val else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "map",
        });
    };

    let map = proc
        .read_map(mem, map_val)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Create new [key value] tuple
    let kv_elements = [key, value];
    let kv_tuple = proc
        .alloc_tuple(mem, &kv_elements)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Prepend to existing entries (structural sharing)
    let new_entries = proc
        .alloc_pair(mem, kv_tuple, map.entries)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Allocate new map with updated entries
    proc.alloc_map(mem, new_entries)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Get list of keys from map.
///
/// `(keys m)` - returns list of keys
pub fn intrinsic_keys<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let map_val = proc.x_regs[1];

    let Value::Map(_) = map_val else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "map",
        });
    };

    let map = proc
        .read_map(mem, map_val)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Build list of keys from the association list
    // We need to reverse the order since we're prepending
    let mut keys = Value::Nil;
    let mut current = map.entries;
    while let Some(pair) = proc.read_pair(mem, current) {
        if let Some(key) = proc.read_tuple_element(mem, pair.first, 0) {
            keys = proc
                .alloc_pair(mem, key, keys)
                .ok_or(IntrinsicError::OutOfMemory)?;
        }
        current = pair.rest;
    }

    // Reverse the list to match iteration order
    reverse_list(proc, mem, keys)
}

/// Get list of values from map.
///
/// `(vals m)` - returns list of values
pub fn intrinsic_vals<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let map_val = proc.x_regs[1];

    let Value::Map(_) = map_val else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "map",
        });
    };

    let map = proc
        .read_map(mem, map_val)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Build list of values from the association list
    let mut vals = Value::Nil;
    let mut current = map.entries;
    while let Some(pair) = proc.read_pair(mem, current) {
        if let Some(val) = proc.read_tuple_element(mem, pair.first, 1) {
            vals = proc
                .alloc_pair(mem, val, vals)
                .ok_or(IntrinsicError::OutOfMemory)?;
        }
        current = pair.rest;
    }

    // Reverse the list to match iteration order
    reverse_list(proc, mem, vals)
}

/// Reverse a list (helper for keys/vals).
fn reverse_list<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    list: Value,
) -> Result<Value, IntrinsicError> {
    let mut result = Value::Nil;
    let mut current = list;
    while let Some(pair) = proc.read_pair(mem, current) {
        result = proc
            .alloc_pair(mem, pair.first, result)
            .ok_or(IntrinsicError::OutOfMemory)?;
        current = pair.rest;
    }
    Ok(result)
}
