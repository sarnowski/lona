// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Built-in intrinsic functions for the Lona VM.
//!
//! Intrinsics are operations implemented in Rust that are called from bytecode.
//! They use a fixed calling convention:
//! - Arguments in X1, X2, ..., X(argc)
//! - Result in X0
//!
//! See `docs/architecture/virtual-machine.md` for the full specification.

#[cfg(test)]
mod intrinsics_test;

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;

/// Intrinsic function IDs.
///
/// These match the intrinsic dispatch table order.
pub mod id {
    /// Addition: `(+ a b)` -> `a + b`
    pub const ADD: u8 = 0;
    /// Subtraction: `(- a b)` -> `a - b`
    pub const SUB: u8 = 1;
    /// Multiplication: `(* a b)` -> `a * b`
    pub const MUL: u8 = 2;
    /// Division: `(/ a b)` -> `a / b`
    pub const DIV: u8 = 3;
    /// Modulo: `(mod a b)` -> `a % b`
    pub const MOD: u8 = 4;
    /// Equality: `(= a b)` -> `a == b`
    pub const EQ: u8 = 5;
    /// Less than: `(< a b)` -> `a < b`
    pub const LT: u8 = 6;
    /// Greater than: `(> a b)` -> `a > b`
    pub const GT: u8 = 7;
    /// Less or equal: `(<= a b)` -> `a <= b`
    pub const LE: u8 = 8;
    /// Greater or equal: `(>= a b)` -> `a >= b`
    pub const GE: u8 = 9;
    /// Boolean not: `(not x)` -> `!x`
    pub const NOT: u8 = 10;
    /// Nil predicate: `(nil? x)` -> `x == nil`
    pub const IS_NIL: u8 = 11;
    /// Integer predicate: `(integer? x)` -> is x an integer?
    pub const IS_INT: u8 = 12;
    /// String predicate: `(string? x)` -> is x a string?
    pub const IS_STR: u8 = 13;
    /// String concatenation: `(str a b ...)` -> concatenated string
    pub const STR: u8 = 14;
    /// Keyword predicate: `(keyword? x)` -> is x a keyword?
    pub const IS_KEYWORD: u8 = 15;
    /// Keyword constructor: `(keyword s)` -> :s
    pub const KEYWORD: u8 = 16;
    /// Get name: `(name x)` -> name string
    pub const NAME: u8 = 17;
    /// Get namespace: `(namespace x)` -> namespace string or nil
    pub const NAMESPACE: u8 = 18;
    /// Tuple predicate: `(tuple? x)` -> is x a tuple?
    pub const IS_TUPLE: u8 = 19;
    /// Get element at index: `(nth tuple index)` -> element
    pub const NTH: u8 = 20;
    /// Get count: `(count coll)` -> length
    pub const COUNT: u8 = 21;
    /// Symbol predicate: `(symbol? x)` -> is x a symbol?
    pub const IS_SYMBOL: u8 = 22;
}

/// Number of defined intrinsics.
pub const INTRINSIC_COUNT: usize = 23;

/// Intrinsic name lookup table.
const INTRINSIC_NAMES: [&str; INTRINSIC_COUNT] = [
    "+",         // 0: ADD
    "-",         // 1: SUB
    "*",         // 2: MUL
    "/",         // 3: DIV
    "mod",       // 4: MOD
    "=",         // 5: EQ
    "<",         // 6: LT
    ">",         // 7: GT
    "<=",        // 8: LE
    ">=",        // 9: GE
    "not",       // 10: NOT
    "nil?",      // 11: IS_NIL
    "integer?",  // 12: IS_INT
    "string?",   // 13: IS_STR
    "str",       // 14: STR
    "keyword?",  // 15: IS_KEYWORD
    "keyword",   // 16: KEYWORD
    "name",      // 17: NAME
    "namespace", // 18: NAMESPACE
    "tuple?",    // 19: IS_TUPLE
    "nth",       // 20: NTH
    "count",     // 21: COUNT
    "symbol?",   // 22: IS_SYMBOL
];

/// Look up an intrinsic ID by name.
///
/// Returns `Some(id)` if the name matches a known intrinsic, `None` otherwise.
#[must_use]
pub fn lookup_intrinsic(name: &str) -> Option<u8> {
    INTRINSIC_NAMES
        .iter()
        .position(|&n| n == name)
        .map(|i| i as u8)
}

/// Get the name of an intrinsic by ID.
///
/// Returns `Some(name)` if the ID is valid, `None` otherwise.
#[must_use]
pub fn intrinsic_name(id: u8) -> Option<&'static str> {
    INTRINSIC_NAMES.get(id as usize).copied()
}

/// Runtime error from intrinsic execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntrinsicError {
    /// Type error: expected a specific type.
    TypeError {
        /// Which intrinsic was called.
        intrinsic: u8,
        /// Which argument (0-indexed).
        arg: u8,
        /// What type was expected.
        expected: &'static str,
    },
    /// Division by zero.
    DivisionByZero,
    /// Integer overflow.
    Overflow,
    /// Unknown intrinsic ID.
    UnknownIntrinsic(u8),
    /// Out of memory during string allocation.
    OutOfMemory,
    /// Index out of bounds.
    IndexOutOfBounds {
        /// The index that was requested.
        index: i64,
        /// The length of the collection.
        len: usize,
    },
}

/// Execute an intrinsic function.
///
/// # Arguments
/// * `intrinsic_id` - The intrinsic to call
/// * `argc` - Number of arguments
/// * `proc` - Process containing registers and heap
/// * `mem` - Memory space
///
/// # Errors
/// Returns an error if the intrinsic fails (type error, division by zero, etc.)
pub fn call_intrinsic<M: MemorySpace>(
    intrinsic_id: u8,
    argc: u8,
    proc: &mut Process,
    mem: &mut M,
) -> Result<(), IntrinsicError> {
    let result = match intrinsic_id {
        id::ADD => intrinsic_add(proc, intrinsic_id)?,
        id::SUB => intrinsic_sub(proc, intrinsic_id)?,
        id::MUL => intrinsic_mul(proc, intrinsic_id)?,
        id::DIV => intrinsic_div(proc, intrinsic_id)?,
        id::MOD => intrinsic_mod(proc, intrinsic_id)?,
        id::EQ => intrinsic_eq(proc, mem),
        id::LT => intrinsic_lt(proc, intrinsic_id)?,
        id::GT => intrinsic_gt(proc, intrinsic_id)?,
        id::LE => intrinsic_le(proc, intrinsic_id)?,
        id::GE => intrinsic_ge(proc, intrinsic_id)?,
        id::NOT => intrinsic_not(proc),
        id::IS_NIL => intrinsic_is_nil(proc),
        id::IS_INT => intrinsic_is_int(proc),
        id::IS_STR => intrinsic_is_str(proc),
        id::STR => intrinsic_str(proc, argc, mem)?,
        id::IS_KEYWORD => intrinsic_is_keyword(proc),
        id::KEYWORD => intrinsic_keyword(proc, mem, intrinsic_id)?,
        id::NAME => intrinsic_name_fn(proc, mem, intrinsic_id)?,
        id::NAMESPACE => intrinsic_namespace(proc, mem, intrinsic_id)?,
        id::IS_TUPLE => intrinsic_is_tuple(proc),
        id::NTH => intrinsic_nth(proc, mem, intrinsic_id)?,
        id::COUNT => intrinsic_count(proc, mem, intrinsic_id)?,
        id::IS_SYMBOL => intrinsic_is_symbol(proc),
        _ => return Err(IntrinsicError::UnknownIntrinsic(intrinsic_id)),
    };
    proc.x_regs[0] = result;
    Ok(())
}

/// Extract an integer from a register, returning a type error if not an int.
const fn expect_int(
    proc: &Process,
    reg: usize,
    intrinsic: u8,
    arg: u8,
) -> Result<i64, IntrinsicError> {
    match proc.x_regs[reg] {
        Value::Int(n) => Ok(n),
        _ => Err(IntrinsicError::TypeError {
            intrinsic,
            arg,
            expected: "integer",
        }),
    }
}

// --- Arithmetic intrinsics ---

fn intrinsic_add(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::int(a.wrapping_add(b)))
}

fn intrinsic_sub(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::int(a.wrapping_sub(b)))
}

fn intrinsic_mul(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::int(a.wrapping_mul(b)))
}

fn intrinsic_div(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    if b == 0 {
        return Err(IntrinsicError::DivisionByZero);
    }
    Ok(Value::int(a.wrapping_div(b)))
}

fn intrinsic_mod(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    if b == 0 {
        return Err(IntrinsicError::DivisionByZero);
    }
    Ok(Value::int(a.wrapping_rem(b)))
}

// --- Comparison intrinsics ---

fn intrinsic_eq<M: MemorySpace>(proc: &Process, mem: &M) -> Value {
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
fn values_equal<M: MemorySpace>(a: Value, b: Value, proc: &Process, mem: &M) -> bool {
    match (a, b) {
        (Value::Nil, Value::Nil) => true,
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
        _ => false, // Different types are never equal
    }
}

fn intrinsic_lt(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::bool(a < b))
}

fn intrinsic_gt(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::bool(a > b))
}

fn intrinsic_le(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::bool(a <= b))
}

fn intrinsic_ge(proc: &Process, id: u8) -> Result<Value, IntrinsicError> {
    let a = expect_int(proc, 1, id, 0)?;
    let b = expect_int(proc, 2, id, 1)?;
    Ok(Value::bool(a >= b))
}

// --- Boolean intrinsic ---

const fn intrinsic_not(proc: &Process) -> Value {
    // (not x) returns true if x is falsy (nil or false), false otherwise
    Value::bool(!proc.x_regs[1].is_truthy())
}

// --- Type predicate intrinsics ---

const fn intrinsic_is_nil(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_nil())
}

const fn intrinsic_is_int(proc: &Process) -> Value {
    Value::bool(matches!(proc.x_regs[1], Value::Int(_)))
}

const fn intrinsic_is_str(proc: &Process) -> Value {
    Value::bool(matches!(proc.x_regs[1], Value::String(_)))
}

// --- String intrinsic ---

/// Maximum buffer size for string concatenation.
const STR_BUFFER_SIZE: usize = 1024;

fn intrinsic_str<M: MemorySpace>(
    proc: &mut Process,
    argc: u8,
    mem: &mut M,
) -> Result<Value, IntrinsicError> {
    // Build the concatenated string in a buffer
    let mut buffer = [0u8; STR_BUFFER_SIZE];
    let mut pos = 0;

    for i in 0..argc as usize {
        let val = proc.x_regs[i + 1]; // Args start at X1
        pos = write_value_to_buffer(&mut buffer, pos, val, proc, mem)?;
    }

    // Allocate the result string
    let s = core::str::from_utf8(&buffer[..pos]).map_err(|_| IntrinsicError::OutOfMemory)?;
    proc.alloc_string(mem, s).ok_or(IntrinsicError::OutOfMemory)
}

/// Write a value's string representation to a buffer.
fn write_value_to_buffer<M: MemorySpace>(
    buffer: &mut [u8; STR_BUFFER_SIZE],
    mut pos: usize,
    value: Value,
    proc: &Process,
    mem: &M,
) -> Result<usize, IntrinsicError> {
    match value {
        Value::Nil => {
            let s = b"nil";
            if pos + s.len() > STR_BUFFER_SIZE {
                return Err(IntrinsicError::OutOfMemory);
            }
            buffer[pos..pos + s.len()].copy_from_slice(s);
            pos += s.len();
        }
        Value::Bool(true) => {
            let s = b"true";
            if pos + s.len() > STR_BUFFER_SIZE {
                return Err(IntrinsicError::OutOfMemory);
            }
            buffer[pos..pos + s.len()].copy_from_slice(s);
            pos += s.len();
        }
        Value::Bool(false) => {
            let s = b"false";
            if pos + s.len() > STR_BUFFER_SIZE {
                return Err(IntrinsicError::OutOfMemory);
            }
            buffer[pos..pos + s.len()].copy_from_slice(s);
            pos += s.len();
        }
        Value::Int(n) => {
            pos = write_int_to_buffer(buffer, pos, n)?;
        }
        Value::String(_) | Value::Symbol(_) => {
            let Some(s) = proc.read_string(mem, value) else {
                return Err(IntrinsicError::OutOfMemory);
            };
            if pos + s.len() > STR_BUFFER_SIZE {
                return Err(IntrinsicError::OutOfMemory);
            }
            buffer[pos..pos + s.len()].copy_from_slice(s.as_bytes());
            pos += s.len();
        }
        Value::Keyword(_) => {
            // Print keyword with leading colon
            if pos + 1 > STR_BUFFER_SIZE {
                return Err(IntrinsicError::OutOfMemory);
            }
            buffer[pos] = b':';
            pos += 1;
            let Some(s) = proc.read_string(mem, value) else {
                return Err(IntrinsicError::OutOfMemory);
            };
            if pos + s.len() > STR_BUFFER_SIZE {
                return Err(IntrinsicError::OutOfMemory);
            }
            buffer[pos..pos + s.len()].copy_from_slice(s.as_bytes());
            pos += s.len();
        }
        Value::Pair(_) => {
            // Print pairs as "<pair>"
            let s = b"<pair>";
            if pos + s.len() > STR_BUFFER_SIZE {
                return Err(IntrinsicError::OutOfMemory);
            }
            buffer[pos..pos + s.len()].copy_from_slice(s);
            pos += s.len();
        }
        Value::Tuple(_) => {
            // Print tuples as "<tuple>"
            let s = b"<tuple>";
            if pos + s.len() > STR_BUFFER_SIZE {
                return Err(IntrinsicError::OutOfMemory);
            }
            buffer[pos..pos + s.len()].copy_from_slice(s);
            pos += s.len();
        }
    }
    Ok(pos)
}

/// Write an integer to a buffer as decimal.
fn write_int_to_buffer(
    buffer: &mut [u8; STR_BUFFER_SIZE],
    pos: usize,
    n: i64,
) -> Result<usize, IntrinsicError> {
    // Handle i64::MIN specially (can't negate it)
    if n == i64::MIN {
        let s = b"-9223372036854775808";
        if pos + s.len() > STR_BUFFER_SIZE {
            return Err(IntrinsicError::OutOfMemory);
        }
        buffer[pos..pos + s.len()].copy_from_slice(s);
        return Ok(pos + s.len());
    }

    // Build digits in reverse
    let mut temp = [0u8; 20]; // Max 20 digits for i64
    let mut temp_pos = 0;
    let negative = n < 0;
    // SAFETY: We handled i64::MIN above, so -n is always valid for n < 0
    let mut val = n.unsigned_abs();

    if val == 0 {
        temp[temp_pos] = b'0';
        temp_pos += 1;
    } else {
        while val > 0 {
            temp[temp_pos] = b'0' + (val % 10) as u8;
            val /= 10;
            temp_pos += 1;
        }
    }

    // Calculate total length
    let total_len = if negative { temp_pos + 1 } else { temp_pos };
    if pos + total_len > STR_BUFFER_SIZE {
        return Err(IntrinsicError::OutOfMemory);
    }

    // Write to buffer
    let mut write_pos = pos;
    if negative {
        buffer[write_pos] = b'-';
        write_pos += 1;
    }
    for i in (0..temp_pos).rev() {
        buffer[write_pos] = temp[i];
        write_pos += 1;
    }

    Ok(write_pos)
}

// --- Keyword intrinsics ---

const fn intrinsic_is_keyword(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_keyword())
}

/// Buffer for copying strings in intrinsics that need to allocate.
const INTRINSIC_STRING_BUFFER_SIZE: usize = 256;

fn intrinsic_keyword<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let val = proc.x_regs[1];

    // Copy string to local buffer to avoid borrow conflict
    let mut buffer = [0u8; INTRINSIC_STRING_BUFFER_SIZE];
    let len = match val {
        Value::String(_) | Value::Symbol(_) | Value::Keyword(_) => {
            let s = proc
                .read_string(mem, val)
                .ok_or(IntrinsicError::OutOfMemory)?;
            let len = s.len().min(INTRINSIC_STRING_BUFFER_SIZE);
            buffer[..len].copy_from_slice(&s.as_bytes()[..len]);
            len
        }
        _ => {
            return Err(IntrinsicError::TypeError {
                intrinsic: id,
                arg: 0,
                expected: "string, symbol, or keyword",
            });
        }
    };

    let s = core::str::from_utf8(&buffer[..len]).map_err(|_| IntrinsicError::OutOfMemory)?;
    proc.alloc_keyword(mem, s)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Get the name part of a keyword or symbol.
/// For `:ns/name` returns `"name"`, for `:name` returns `"name"`.
fn intrinsic_name_fn<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let val = proc.x_regs[1];

    // Copy string to local buffer to avoid borrow conflict
    let mut buffer = [0u8; INTRINSIC_STRING_BUFFER_SIZE];
    let len = match val {
        Value::Keyword(_) | Value::Symbol(_) => {
            let s = proc
                .read_string(mem, val)
                .ok_or(IntrinsicError::OutOfMemory)?;
            let len = s.len().min(INTRINSIC_STRING_BUFFER_SIZE);
            buffer[..len].copy_from_slice(&s.as_bytes()[..len]);
            len
        }
        _ => {
            return Err(IntrinsicError::TypeError {
                intrinsic: id,
                arg: 0,
                expected: "keyword or symbol",
            });
        }
    };

    let s = core::str::from_utf8(&buffer[..len]).map_err(|_| IntrinsicError::OutOfMemory)?;

    // Find the last '/' and return everything after it
    let name = s.rfind('/').map_or(s, |pos| &s[pos + 1..]);
    proc.alloc_string(mem, name)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Get the namespace part of a qualified keyword or symbol.
/// For `:ns/name` returns `"ns"`, for `:name` returns `nil`.
fn intrinsic_namespace<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let val = proc.x_regs[1];

    // Copy string to local buffer to avoid borrow conflict
    let mut buffer = [0u8; INTRINSIC_STRING_BUFFER_SIZE];
    let len = match val {
        Value::Keyword(_) | Value::Symbol(_) => {
            let s = proc
                .read_string(mem, val)
                .ok_or(IntrinsicError::OutOfMemory)?;
            let len = s.len().min(INTRINSIC_STRING_BUFFER_SIZE);
            buffer[..len].copy_from_slice(&s.as_bytes()[..len]);
            len
        }
        _ => {
            return Err(IntrinsicError::TypeError {
                intrinsic: id,
                arg: 0,
                expected: "keyword or symbol",
            });
        }
    };

    let s = core::str::from_utf8(&buffer[..len]).map_err(|_| IntrinsicError::OutOfMemory)?;

    // Find the last '/' - if present, return everything before it
    s.rfind('/').map_or_else(
        || Ok(Value::nil()),
        |pos| {
            let ns = &s[..pos];
            proc.alloc_string(mem, ns)
                .ok_or(IntrinsicError::OutOfMemory)
        },
    )
}

// --- Tuple intrinsics ---

const fn intrinsic_is_tuple(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_tuple())
}

fn intrinsic_nth<M: MemorySpace>(proc: &Process, mem: &M, id: u8) -> Result<Value, IntrinsicError> {
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

fn intrinsic_count<M: MemorySpace>(
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
        _ => Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "collection",
        }),
    }
}

// --- Symbol intrinsic ---

const fn intrinsic_is_symbol(proc: &Process) -> Value {
    Value::bool(matches!(proc.x_regs[1], Value::Symbol(_)))
}
