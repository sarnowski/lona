// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Sequence intrinsics (first, rest, empty?).
//!
//! These polymorphic intrinsics work on any collection type.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;

use super::IntrinsicError;

/// Get the first element of a collection.
///
/// - `(first nil)` → nil
/// - `(first '(1 2 3))` → 1
/// - `(first [1 2 3])` → 1 (tuple)
/// - `(first {1 2 3})` → 1 (vector)
/// - `(first %{:a 1})` → [:a 1]
/// - `(first '())` → nil
pub fn intrinsic_first<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let coll = proc.x_regs[1];

    match coll {
        Value::Nil => Ok(Value::Nil),
        Value::Pair(_) => {
            let pair = proc
                .read_pair(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            Ok(pair.first)
        }
        Value::Tuple(_) | Value::Vector(_) => {
            let len = proc
                .read_tuple_len(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            if len == 0 {
                Ok(Value::Nil)
            } else {
                proc.read_tuple_element(mem, coll, 0)
                    .ok_or(IntrinsicError::OutOfMemory)
            }
        }
        Value::Map(_) => {
            let map = proc
                .read_map(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            // pair.first is already a [key value] tuple
            Ok(proc
                .read_pair(mem, map.entries)
                .map_or(Value::Nil, |pair| pair.first))
        }
        _ => Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "sequence",
        }),
    }
}

/// Get the rest of a collection (all elements except the first).
///
/// Always returns a list (pair chain or nil), regardless of input type.
///
/// - `(rest nil)` → ()
/// - `(rest '(1 2 3))` → (2 3)
/// - `(rest [1 2 3])` → (2 3) (list, not tuple)
/// - `(rest {1 2 3})` → (2 3) (list, not vector)
/// - `(rest '(1))` → ()
pub fn intrinsic_rest<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let coll = proc.x_regs[1];

    match coll {
        Value::Nil => Ok(Value::Nil),
        Value::Pair(_) => {
            let pair = proc
                .read_pair(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            Ok(pair.rest)
        }
        Value::Tuple(_) | Value::Vector(_) => {
            let len = proc
                .read_tuple_len(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            if len <= 1 {
                return Ok(Value::Nil);
            }
            // Build list from elements 1..len (back to front)
            build_list_from_indexed(proc, mem, coll, 1, len)
        }
        Value::Map(_) => {
            let map = proc
                .read_map(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            // Return the rest of the entries list
            Ok(proc
                .read_pair(mem, map.entries)
                .map_or(Value::Nil, |pair| pair.rest))
        }
        _ => Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "sequence",
        }),
    }
}

/// Build a list from indexed elements [start..end) of a tuple/vector.
fn build_list_from_indexed<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    coll: Value,
    start: usize,
    end: usize,
) -> Result<Value, IntrinsicError> {
    let mut result = Value::Nil;
    for i in (start..end).rev() {
        let elem = proc
            .read_tuple_element(mem, coll, i)
            .ok_or(IntrinsicError::OutOfMemory)?;
        result = proc
            .alloc_pair(mem, elem, result)
            .ok_or(IntrinsicError::OutOfMemory)?;
    }
    Ok(result)
}

/// Check if a collection is empty.
///
/// - `(empty? nil)` → true
/// - `(empty? '())` → true (nil is the empty list)
/// - `(empty? '(1))` → false
/// - `(empty? [])` → true
/// - `(empty? {})` → true
/// - `(empty? %{})` → true
pub fn intrinsic_is_empty<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let coll = proc.x_regs[1];

    match coll {
        Value::Nil => Ok(Value::bool(true)),
        Value::Pair(_) => Ok(Value::bool(false)), // Pairs are never empty
        Value::Tuple(_) | Value::Vector(_) => {
            let len = proc
                .read_tuple_len(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            Ok(Value::bool(len == 0))
        }
        Value::Map(_) => {
            let map = proc
                .read_map(mem, coll)
                .ok_or(IntrinsicError::OutOfMemory)?;
            Ok(Value::bool(map.entries.is_nil()))
        }
        _ => Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "sequence",
        }),
    }
}
