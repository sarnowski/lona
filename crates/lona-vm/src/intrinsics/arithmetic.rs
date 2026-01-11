// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Arithmetic and comparison intrinsics.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;

use super::{IntrinsicError, expect_int};

// --- Arithmetic intrinsics ---

pub fn intrinsic_add(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::int(a.wrapping_add(b)))
}

pub fn intrinsic_sub(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::int(a.wrapping_sub(b)))
}

pub fn intrinsic_mul(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::int(a.wrapping_mul(b)))
}

pub fn intrinsic_div(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    if b == 0 {
        return Err(IntrinsicError::DivisionByZero);
    }
    Ok(Value::int(a.wrapping_div(b)))
}

pub fn intrinsic_mod(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    if b == 0 {
        return Err(IntrinsicError::DivisionByZero);
    }
    // Modulus: result has same sign as divisor (b)
    // This differs from remainder where sign follows dividend (a)
    let rem = a.wrapping_rem(b);
    let result = if (rem < 0 && b > 0) || (rem > 0 && b < 0) {
        rem.wrapping_add(b)
    } else {
        rem
    };
    Ok(Value::int(result))
}

// --- Comparison intrinsics ---

pub fn intrinsic_eq<M: MemorySpace>(proc: &Process, mem: &M) -> Value {
    let a = proc.x_regs[1];
    let b = proc.x_regs[2];
    Value::bool(values_equal(a, b, proc, mem))
}

/// Reference identity comparison.
///
/// Returns true if two values are the exact same object (same address for
/// heap-allocated values, same value for immediates).
pub const fn intrinsic_identical(proc: &Process) -> Value {
    let a = proc.x_regs[1];
    let b = proc.x_regs[2];
    Value::bool(values_identical(a, b))
}

/// Check if two values are identical (reference equality).
///
/// For heap-allocated values, this compares addresses.
/// For immediate values, this compares the values directly.
const fn values_identical(a: Value, b: Value) -> bool {
    match (a, b) {
        // Immediate values without payload
        (Value::Nil, Value::Nil) | (Value::Unbound, Value::Unbound) => true,
        // Immediate values with payload - compare directly
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::NativeFn(x), Value::NativeFn(y)) => x == y,
        // Heap-allocated values - compare addresses
        (Value::String(a), Value::String(b))
        | (Value::Pair(a), Value::Pair(b))
        | (Value::Symbol(a), Value::Symbol(b))
        | (Value::Keyword(a), Value::Keyword(b))
        | (Value::Tuple(a), Value::Tuple(b))
        | (Value::Vector(a), Value::Vector(b))
        | (Value::Map(a), Value::Map(b))
        | (Value::CompiledFn(a), Value::CompiledFn(b))
        | (Value::Closure(a), Value::Closure(b))
        | (Value::Var(a), Value::Var(b))
        | (Value::Namespace(a), Value::Namespace(b)) => a.as_u64() == b.as_u64(),
        // Different types are never identical
        _ => false,
    }
}

/// Maximum recursion depth for structural equality to prevent stack overflow.
const MAX_EQ_DEPTH: usize = 64;

/// Compare two values for equality.
///
/// - Immediate values (nil, bool, int) compare by value
/// - Strings compare by content
/// - Keywords compare by content
/// - Symbols compare by content (fast path: address comparison for interned symbols)
/// - Pairs compare structurally (recursive element comparison)
/// - Tuples compare structurally (recursive element comparison)
/// - Maps compare structurally (same keys with equal values)
pub fn values_equal<M: MemorySpace>(a: Value, b: Value, proc: &Process, mem: &M) -> bool {
    values_equal_depth(a, b, proc, mem, 0)
}

fn values_equal_depth<M: MemorySpace>(
    a: Value,
    b: Value,
    proc: &Process,
    mem: &M,
    depth: usize,
) -> bool {
    if depth > MAX_EQ_DEPTH {
        return false;
    }

    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::String(_), Value::String(_)) => {
            // Compare string contents
            let Some(sa) = proc.read_string(mem, a) else {
                return false;
            };
            let Some(sb) = proc.read_string(mem, b) else {
                return false;
            };
            sa == sb
        }
        (Value::Symbol(addr_a), Value::Symbol(addr_b)) => {
            // Fast path: same address (interned symbols)
            if addr_a == addr_b {
                return true;
            }
            // Fallback: compare by content in case interning table was full
            let Some(sa) = proc.read_string(mem, a) else {
                return false;
            };
            let Some(sb) = proc.read_string(mem, b) else {
                return false;
            };
            sa == sb
        }
        (Value::Keyword(_), Value::Keyword(_)) => {
            // Keywords compare by content (like strings) to handle cases
            // where interning table is full and identical keywords get
            // different addresses.
            let Some(sa) = proc.read_string(mem, a) else {
                return false;
            };
            let Some(sb) = proc.read_string(mem, b) else {
                return false;
            };
            sa == sb
        }
        (Value::Pair(addr_a), Value::Pair(addr_b)) => {
            // Fast path: same address
            if addr_a == addr_b {
                return true;
            }
            // Structural comparison
            let Some(pa) = proc.read_pair(mem, a) else {
                return false;
            };
            let Some(pb) = proc.read_pair(mem, b) else {
                return false;
            };
            values_equal_depth(pa.first, pb.first, proc, mem, depth + 1)
                && values_equal_depth(pa.rest, pb.rest, proc, mem, depth + 1)
        }
        // Tuples and vectors share the same memory layout
        (Value::Tuple(addr_a), Value::Tuple(addr_b))
        | (Value::Vector(addr_a), Value::Vector(addr_b)) => {
            // Fast path: same address
            if addr_a == addr_b {
                return true;
            }
            indexed_equal(a, b, proc, mem, depth)
        }
        (Value::Map(addr_a), Value::Map(addr_b)) => {
            // Fast path: same address
            if addr_a == addr_b {
                return true;
            }
            maps_equal(proc, mem, a, b, depth + 1)
        }
        (Value::CompiledFn(addr_a), Value::CompiledFn(addr_b)) => {
            // Functions compare by identity
            addr_a == addr_b
        }
        (Value::Closure(addr_a), Value::Closure(addr_b)) => {
            // Closures compare by identity
            addr_a == addr_b
        }
        (Value::NativeFn(id_a), Value::NativeFn(id_b)) => {
            // Native functions compare by intrinsic ID
            id_a == id_b
        }
        (Value::Namespace(addr_a), Value::Namespace(addr_b)) => {
            // Namespaces compare by identity
            addr_a == addr_b
        }
        // Nil and Unbound are immediate values that compare equal to themselves
        (Value::Nil, Value::Nil) | (Value::Unbound, Value::Unbound) => true,
        _ => false, // Different types are never equal
    }
}

/// Compare two indexed collections (tuples or vectors) for structural equality.
fn indexed_equal<M: MemorySpace>(
    a: Value,
    b: Value,
    proc: &Process,
    mem: &M,
    depth: usize,
) -> bool {
    let Some(len_a) = proc.read_tuple_len(mem, a) else {
        return false;
    };
    let Some(len_b) = proc.read_tuple_len(mem, b) else {
        return false;
    };
    if len_a != len_b {
        return false;
    }
    for i in 0..len_a {
        let Some(ea) = proc.read_tuple_element(mem, a, i) else {
            return false;
        };
        let Some(eb) = proc.read_tuple_element(mem, b, i) else {
            return false;
        };
        if !values_equal_depth(ea, eb, proc, mem, depth + 1) {
            return false;
        }
    }
    true
}

/// Count entries in a map's entry list.
fn count_map_entries<M: MemorySpace>(proc: &Process, mem: &M, mut entries: Value) -> usize {
    let mut count = 0;
    while let Some(pair) = proc.read_pair(mem, entries) {
        count += 1;
        entries = pair.rest;
    }
    count
}

/// Look up a key in a map, returning the value if found.
///
/// Unlike `Process::map_get`, this returns `Some(Value::Nil)` for keys
/// with nil values instead of returning `None`.
fn map_lookup<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    map_val: Value,
    key: Value,
) -> Option<Value> {
    let map = proc.read_map(mem, map_val)?;
    let mut current = map.entries;
    while let Some(pair) = proc.read_pair(mem, current) {
        let entry_key = proc.read_tuple_element(mem, pair.first, 0)?;
        if values_equal(entry_key, key, proc, mem) {
            return proc.read_tuple_element(mem, pair.first, 1);
        }
        current = pair.rest;
    }
    None
}

/// Compare two maps for structural equality.
fn maps_equal<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    map_a: Value,
    map_b: Value,
    depth: usize,
) -> bool {
    let Some(ma) = proc.read_map(mem, map_a) else {
        return false;
    };
    let Some(mb) = proc.read_map(mem, map_b) else {
        return false;
    };

    // Count entries in both maps
    let count_a = count_map_entries(proc, mem, ma.entries);
    let count_b = count_map_entries(proc, mem, mb.entries);
    if count_a != count_b {
        return false;
    }

    // Check each entry in map_a exists in map_b with equal value
    let mut current = ma.entries;
    while let Some(pair) = proc.read_pair(mem, current) {
        // Each entry is a [key value] tuple
        let Some(key) = proc.read_tuple_element(mem, pair.first, 0) else {
            return false;
        };
        let Some(val_a) = proc.read_tuple_element(mem, pair.first, 1) else {
            return false;
        };

        // Look up key in map_b using Unbound as sentinel to distinguish
        // "key not found" from "key has nil value"
        match map_lookup(proc, mem, map_b, key) {
            None => return false, // Key not found
            Some(val_b) => {
                if !values_equal_depth(val_a, val_b, proc, mem, depth) {
                    return false;
                }
            }
        }
        current = pair.rest;
    }
    true
}

pub fn intrinsic_lt(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::bool(a < b))
}

pub fn intrinsic_gt(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::bool(a > b))
}

pub fn intrinsic_le(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::bool(a <= b))
}

pub fn intrinsic_ge(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::bool(a >= b))
}

// --- Boolean intrinsic ---

pub const fn intrinsic_not(proc: &Process) -> Value {
    // (not x) returns true if x is falsy (nil or false), false otherwise
    Value::bool(!proc.x_regs[1].is_truthy())
}
