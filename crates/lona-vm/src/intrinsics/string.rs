// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! String and keyword intrinsics.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;

use super::IntrinsicError;

/// Maximum buffer size for string concatenation.
const STR_BUFFER_SIZE: usize = 1024;

/// Buffer for copying strings in intrinsics that need to allocate.
const INTRINSIC_STRING_BUFFER_SIZE: usize = 256;

pub fn intrinsic_str<M: MemorySpace>(
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

/// Helper to write a static byte slice to a buffer.
fn write_static_to_buffer(
    buffer: &mut [u8; STR_BUFFER_SIZE],
    pos: usize,
    s: &[u8],
) -> Result<usize, IntrinsicError> {
    if pos + s.len() > STR_BUFFER_SIZE {
        return Err(IntrinsicError::OutOfMemory);
    }
    buffer[pos..pos + s.len()].copy_from_slice(s);
    Ok(pos + s.len())
}

/// Write a value's string representation to a buffer.
fn write_value_to_buffer<M: MemorySpace>(
    buffer: &mut [u8; STR_BUFFER_SIZE],
    pos: usize,
    value: Value,
    proc: &Process,
    mem: &M,
) -> Result<usize, IntrinsicError> {
    match value {
        Value::Nil => write_static_to_buffer(buffer, pos, b"nil"),
        Value::Bool(true) => write_static_to_buffer(buffer, pos, b"true"),
        Value::Bool(false) => write_static_to_buffer(buffer, pos, b"false"),
        Value::Int(n) => write_int_to_buffer(buffer, pos, n),
        Value::String(_) | Value::Symbol(_) => {
            let Some(s) = proc.read_string(mem, value) else {
                return Err(IntrinsicError::OutOfMemory);
            };
            write_static_to_buffer(buffer, pos, s.as_bytes())
        }
        Value::Keyword(_) => {
            let pos = write_static_to_buffer(buffer, pos, b":")?;
            let Some(s) = proc.read_string(mem, value) else {
                return Err(IntrinsicError::OutOfMemory);
            };
            write_static_to_buffer(buffer, pos, s.as_bytes())
        }
        Value::Pair(_) => write_static_to_buffer(buffer, pos, b"<pair>"),
        Value::Tuple(_) => write_static_to_buffer(buffer, pos, b"<tuple>"),
        Value::Map(_) => write_static_to_buffer(buffer, pos, b"<map>"),
        Value::Var(_) => write_static_to_buffer(buffer, pos, b"<var>"),
        Value::Namespace(_) => write_static_to_buffer(buffer, pos, b"<namespace>"),
        Value::CompiledFn(_) => write_static_to_buffer(buffer, pos, b"<fn>"),
        Value::Closure(_) => write_static_to_buffer(buffer, pos, b"<closure>"),
        Value::NativeFn(_) => write_static_to_buffer(buffer, pos, b"<native-fn>"),
        Value::Unbound => write_static_to_buffer(buffer, pos, b"<unbound>"),
    }
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

pub const fn intrinsic_is_keyword(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_keyword())
}

pub fn intrinsic_keyword<M: MemorySpace>(
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
pub fn intrinsic_name_fn<M: MemorySpace>(
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
pub fn intrinsic_namespace<M: MemorySpace>(
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
