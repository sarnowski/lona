// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Collection intrinsics (tuple, map, list operations).

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;

use super::IntrinsicError;
use super::arithmetic::values_equal;

// --- Tuple intrinsics ---

pub const fn intrinsic_is_tuple(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_tuple())
}

pub fn intrinsic_nth<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let coll = proc.x_regs[1];
    let idx_val = proc.x_regs[2];

    let Value::Int(idx) = idx_val else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 1,
            expected: "integer",
        });
    };

    match coll {
        Value::Tuple(_) => {
            let len = proc
                .read_tuple_len(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;

            // Check bounds - convert to usize safely
            let idx_usize = usize::try_from(idx)
                .ok()
                .filter(|&i| i < len)
                .ok_or(IntrinsicError::IndexOutOfBounds { index: idx, len })?;

            proc.read_tuple_element(mem, coll, idx_usize)
                .ok_or(IntrinsicError::OutOfMemory)
        }
        _ => Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "tuple",
        }),
    }
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

    // Search the association list for the key
    let mut current = map.entries;
    while let Some(pair) = proc.read_pair(mem, current) {
        // Each pair.first is a [key value] tuple
        if let Some(entry_key) = proc.read_tuple_element(mem, pair.first, 0) {
            if values_equal(entry_key, key, proc, mem) {
                // Found the key, return the value
                return proc
                    .read_tuple_element(mem, pair.first, 1)
                    .ok_or(IntrinsicError::OutOfMemory);
            }
        }
        current = pair.rest;
    }

    // Key not found, return default
    Ok(default)
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
