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
    Ok(Value::int(a.wrapping_rem(b)))
}

// --- Comparison intrinsics ---

pub fn intrinsic_eq<M: MemorySpace>(proc: &Process, mem: &M) -> Value {
    let a = proc.x_regs[1];
    let b = proc.x_regs[2];
    Value::bool(values_equal(a, b, proc, mem))
}

/// Compare two values for equality.
///
/// - Immediate values (nil, bool, int) compare by value
/// - Strings compare by content
/// - Keywords compare by content
/// - Symbols compare by identity (address)
/// - Pairs compare by identity (address)
/// - Tuples compare by identity (address)
pub fn values_equal<M: MemorySpace>(a: Value, b: Value, proc: &Process, mem: &M) -> bool {
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
            // Symbols compare by identity
            addr_a == addr_b
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
            // Pairs compare by identity
            addr_a == addr_b
        }
        (Value::Tuple(addr_a), Value::Tuple(addr_b)) => {
            // Tuples compare by identity
            addr_a == addr_b
        }
        (Value::Map(addr_a), Value::Map(addr_b)) => {
            // Maps compare by identity
            addr_a == addr_b
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
