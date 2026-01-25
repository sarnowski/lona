// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Collection intrinsics (tuple, map, list operations).
//!
//! This module provides both intrinsic functions (called from bytecode) and
//! core functions (pure lookup logic) that can be reused by callable data
//! structures.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::term::Term;

use super::arithmetic::terms_equal;
use super::{IntrinsicError, XRegs};

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
/// * `map_term` - The map to search (must be a boxed MAP)
/// * `key` - The key to look up
/// * `default` - Value to return if key not found
///
/// # Errors
/// Returns `CoreCollectionError::NotAMap` if `map_term` is not a map.
pub fn core_get<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    map_term: Term,
    key: Term,
    default: Term,
) -> Result<Term, CoreCollectionError> {
    if !proc.is_term_map(mem, map_term) {
        return Err(CoreCollectionError::NotAMap);
    }

    let entries = proc
        .read_term_map_entries(mem, map_term)
        .ok_or(CoreCollectionError::OutOfMemory)?;

    // Search the association list for the key
    let mut current = entries;
    while let Some((entry, rest)) = proc.read_term_pair(mem, current) {
        // Each entry is a [key value] tuple
        if let Some(entry_key) = proc.read_term_tuple_element(mem, entry, 0) {
            if terms_equal(entry_key, key, proc, mem) {
                // Found the key, return the value
                return proc
                    .read_term_tuple_element(mem, entry, 1)
                    .ok_or(CoreCollectionError::OutOfMemory);
            }
        }
        current = rest;
    }

    // Key not found, return default
    Ok(default)
}

/// Core map contains - checks if a key exists in a map.
///
/// This is the pure existence check logic.
/// Used by pattern matching to detect missing keys.
///
/// # Arguments
/// * `proc` - Process for heap access
/// * `mem` - Memory space
/// * `map_term` - The map to search (must be a boxed MAP)
/// * `key` - The key to check
///
/// # Errors
/// Returns `CoreCollectionError::NotAMap` if `map_term` is not a map.
pub fn core_contains<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    map_term: Term,
    key: Term,
) -> Result<bool, CoreCollectionError> {
    if !proc.is_term_map(mem, map_term) {
        return Err(CoreCollectionError::NotAMap);
    }

    let entries = proc
        .read_term_map_entries(mem, map_term)
        .ok_or(CoreCollectionError::OutOfMemory)?;

    // Search the association list for the key
    let mut current = entries;
    while let Some((entry, rest)) = proc.read_term_pair(mem, current) {
        // Each entry is a [key value] tuple
        if let Some(entry_key) = proc.read_term_tuple_element(mem, entry, 0) {
            if terms_equal(entry_key, key, proc, mem) {
                return Ok(true);
            }
        }
        current = rest;
    }

    Ok(false)
}

/// Core tuple/indexed access - returns element at index, or default/error on OOB.
///
/// This is the pure index logic extracted from `intrinsic_nth`.
/// Used by both the `nth` intrinsic and tuple callables.
///
/// # Arguments
/// * `proc` - Process for heap access
/// * `mem` - Memory space
/// * `coll` - The collection (must be a tuple)
/// * `idx` - The index to access (must be a `small_int`)
/// * `default` - If `Some`, return this on out-of-bounds. If `None`, return error.
///
/// # Errors
/// - `CoreCollectionError::NotATuple` if `coll` is not a tuple
/// - `CoreCollectionError::InvalidIndex` if `idx` is not an integer
/// - `CoreCollectionError::IndexOutOfBounds` if index is out of range and no default
pub fn core_nth<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    coll: Term,
    idx: Term,
    default: Option<Term>,
) -> Result<Term, CoreCollectionError> {
    let Some(idx_i64) = idx.as_small_int() else {
        return Err(CoreCollectionError::InvalidIndex);
    };

    if !proc.is_term_tuple(mem, coll) {
        return Err(CoreCollectionError::NotATuple);
    }

    let len = proc
        .read_term_tuple_len(mem, coll)
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
            proc.read_term_tuple_element(mem, coll, i)
                .ok_or(CoreCollectionError::OutOfMemory)
        },
    )
}

// --- Tuple intrinsics ---

pub fn intrinsic_is_tuple<M: MemorySpace>(x_regs: &XRegs, proc: &Process, mem: &M) -> Term {
    Term::bool(proc.is_term_tuple(mem, x_regs[1]))
}

pub fn intrinsic_is_vector<M: MemorySpace>(x_regs: &XRegs, proc: &Process, mem: &M) -> Term {
    Term::bool(proc.is_term_vector(mem, x_regs[1]))
}

/// Get element at index from a tuple.
///
/// `(nth tuple index)` - returns element or errors on OOB
/// `(nth tuple index not-found)` - returns element or not-found on OOB
pub fn intrinsic_nth<M: MemorySpace>(
    x_regs: &XRegs,
    argc: u8,
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let coll = x_regs[1];
    let idx = x_regs[2];
    let default = if argc >= 3 { Some(x_regs[3]) } else { None };

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
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let coll = x_regs[1];

    // Check immediate types first
    if coll.is_nil() {
        return Term::small_int(0).ok_or(IntrinsicError::Overflow);
    }

    // Check for list (pair)
    if coll.is_list() {
        // Count list length
        let mut count: i64 = 0;
        let mut current = coll;
        while let Some((_, rest)) = proc.read_term_pair(mem, current) {
            count += 1;
            current = rest;
        }
        return Term::small_int(count).ok_or(IntrinsicError::Overflow);
    }

    // Check for boxed types
    if coll.is_boxed() {
        // Try tuple
        if let Some(len) = proc.read_term_tuple_len(mem, coll) {
            let len_i64 = i64::try_from(len).map_err(|_| IntrinsicError::Overflow)?;
            return Term::small_int(len_i64).ok_or(IntrinsicError::Overflow);
        }

        // Try vector
        if let Some(len) = proc.read_term_vector_len(mem, coll) {
            let len_i64 = i64::try_from(len).map_err(|_| IntrinsicError::Overflow)?;
            return Term::small_int(len_i64).ok_or(IntrinsicError::Overflow);
        }

        // Try string
        if let Some(s) = proc.read_term_string(mem, coll) {
            let len_i64 = i64::try_from(s.len()).map_err(|_| IntrinsicError::Overflow)?;
            return Term::small_int(len_i64).ok_or(IntrinsicError::Overflow);
        }

        // Try map
        if let Some(entries) = proc.read_term_map_entries(mem, coll) {
            let mut count: i64 = 0;
            let mut current = entries;
            while let Some((_, rest)) = proc.read_term_pair(mem, current) {
                count += 1;
                current = rest;
            }
            return Term::small_int(count).ok_or(IntrinsicError::Overflow);
        }
    }

    Err(IntrinsicError::TypeError {
        intrinsic: id,
        arg: 0,
        expected: "collection",
    })
}

// --- Symbol intrinsic ---

#[inline]
pub const fn intrinsic_is_symbol(x_regs: &XRegs, proc: &Process) -> Term {
    Term::bool(proc.is_term_symbol(x_regs[1]))
}

// --- Map intrinsics ---

pub fn intrinsic_is_map<M: MemorySpace>(x_regs: &XRegs, proc: &Process, mem: &M) -> Term {
    Term::bool(proc.is_term_map(mem, x_regs[1]))
}

/// Get value from map by key.
///
/// `(get m k)` - returns value or nil
/// `(get m k default)` - returns value or default
pub fn intrinsic_get<M: MemorySpace>(
    x_regs: &XRegs,
    argc: u8,
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let map_term = x_regs[1];
    let key = x_regs[2];
    let default = if argc >= 3 { x_regs[3] } else { Term::NIL };

    core_get(proc, mem, map_term, key, default).map_err(|e| match e {
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
    x_regs: &XRegs,
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let map_term = x_regs[1];
    let key = x_regs[2];
    let value = x_regs[3];

    if !proc.is_term_map(mem, map_term) {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "map",
        });
    }

    let entries = proc
        .read_term_map_entries(mem, map_term)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Create new [key value] tuple
    let kv_elements = [key, value];
    let kv_tuple = proc
        .alloc_term_tuple(mem, &kv_elements)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Prepend to existing entries (structural sharing)
    let new_entries = proc
        .alloc_term_pair(mem, kv_tuple, entries)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Count entries for header (simple count, not deduped)
    let mut entry_count = 0;
    let mut current = new_entries;
    while let Some((_, rest)) = proc.read_term_pair(mem, current) {
        entry_count += 1;
        current = rest;
    }

    // Allocate new map with updated entries
    proc.alloc_term_map(mem, new_entries, entry_count)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Get list of keys from map.
///
/// `(keys m)` - returns list of keys
pub fn intrinsic_keys<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let map_term = x_regs[1];

    if !proc.is_term_map(mem, map_term) {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "map",
        });
    }

    let entries = proc
        .read_term_map_entries(mem, map_term)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Build list of keys from the association list
    // We need to reverse the order since we're prepending
    let mut keys = Term::NIL;
    let mut current = entries;
    while let Some((entry, rest)) = proc.read_term_pair(mem, current) {
        if let Some(key) = proc.read_term_tuple_element(mem, entry, 0) {
            keys = proc
                .alloc_term_pair(mem, key, keys)
                .ok_or(IntrinsicError::OutOfMemory)?;
        }
        current = rest;
    }

    // Reverse the list to match iteration order
    reverse_list(proc, mem, keys)
}

/// Get list of values from map.
///
/// `(vals m)` - returns list of values
pub fn intrinsic_vals<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let map_term = x_regs[1];

    if !proc.is_term_map(mem, map_term) {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "map",
        });
    }

    let entries = proc
        .read_term_map_entries(mem, map_term)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Build list of values from the association list
    let mut vals = Term::NIL;
    let mut current = entries;
    while let Some((entry, rest)) = proc.read_term_pair(mem, current) {
        if let Some(val) = proc.read_term_tuple_element(mem, entry, 1) {
            vals = proc
                .alloc_term_pair(mem, val, vals)
                .ok_or(IntrinsicError::OutOfMemory)?;
        }
        current = rest;
    }

    // Reverse the list to match iteration order
    reverse_list(proc, mem, vals)
}

/// Reverse a list (helper for keys/vals).
fn reverse_list<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    list: Term,
) -> Result<Term, IntrinsicError> {
    let mut result = Term::NIL;
    let mut current = list;
    while let Some((head, rest)) = proc.read_term_pair(mem, current) {
        result = proc
            .alloc_term_pair(mem, head, result)
            .ok_or(IntrinsicError::OutOfMemory)?;
        current = rest;
    }
    Ok(result)
}

/// Check if map contains key.
///
/// `(contains? m k)` - returns true if key exists in map
pub fn intrinsic_contains<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let map_term = x_regs[1];
    let key = x_regs[2];

    core_contains(proc, mem, map_term, key)
        .map(Term::bool)
        .map_err(|e| match e {
            CoreCollectionError::NotAMap => IntrinsicError::TypeError {
                intrinsic: id,
                arg: 0,
                expected: "map",
            },
            _ => IntrinsicError::OutOfMemory,
        })
}
