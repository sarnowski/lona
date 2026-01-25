// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! String and keyword intrinsics.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::term::Term;
use crate::term::tag::object;

use super::{IntrinsicError, XRegs};

/// Maximum buffer size for string concatenation.
const STR_BUFFER_SIZE: usize = 1024;

/// Buffer for copying strings in intrinsics that need to allocate.
const INTRINSIC_STRING_BUFFER_SIZE: usize = 256;

pub fn intrinsic_str<M: MemorySpace>(
    x_regs: &XRegs,
    argc: u8,
    proc: &mut Process,
    realm: &Realm,
    mem: &mut M,
) -> Result<Term, IntrinsicError> {
    // Build the concatenated string in a buffer
    let mut buffer = [0u8; STR_BUFFER_SIZE];
    let mut pos = 0;

    for i in 0..argc as usize {
        let term = x_regs[i + 1]; // Args start at X1
        pos = write_term_to_buffer(&mut buffer, pos, term, proc, realm, mem)?;
    }

    // Allocate the result string
    let s = core::str::from_utf8(&buffer[..pos]).map_err(|_| IntrinsicError::OutOfMemory)?;
    proc.alloc_term_string(mem, s)
        .ok_or(IntrinsicError::OutOfMemory)
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

/// Write a term's string representation to a buffer.
fn write_term_to_buffer<M: MemorySpace>(
    buffer: &mut [u8; STR_BUFFER_SIZE],
    pos: usize,
    term: Term,
    proc: &Process,
    realm: &Realm,
    mem: &M,
) -> Result<usize, IntrinsicError> {
    // Check immediates first
    if term.is_nil() {
        return write_static_to_buffer(buffer, pos, b"nil");
    }
    if term.is_true() {
        return write_static_to_buffer(buffer, pos, b"true");
    }
    if term.is_false() {
        return write_static_to_buffer(buffer, pos, b"false");
    }
    if let Some(n) = term.as_small_int() {
        return write_int_to_buffer(buffer, pos, n);
    }
    // Immediate symbols - look up name from realm
    if let Some(idx) = term.as_symbol_index() {
        if let Some(name) = realm.symbol_name(mem, idx) {
            return write_static_to_buffer(buffer, pos, name.as_bytes());
        }
        return write_static_to_buffer(buffer, pos, b"<symbol>");
    }
    // Immediate keywords - look up name from realm, include colon prefix
    if let Some(idx) = term.as_keyword_index() {
        let new_pos = write_static_to_buffer(buffer, pos, b":")?;
        if let Some(name) = realm.keyword_name(mem, idx) {
            return write_static_to_buffer(buffer, new_pos, name.as_bytes());
        }
        return write_static_to_buffer(buffer, new_pos, b"<keyword>");
    }

    // Check boxed types
    if term.is_boxed() {
        use crate::term::header::Header;
        let addr = term.to_vaddr();
        let header: Header = mem.read(addr);

        match header.object_tag() {
            object::STRING => {
                let Some(s) = proc.read_term_string(mem, term) else {
                    return Err(IntrinsicError::OutOfMemory);
                };
                write_static_to_buffer(buffer, pos, s.as_bytes())
            }
            object::TUPLE => write_static_to_buffer(buffer, pos, b"<tuple>"),
            object::VECTOR => write_static_to_buffer(buffer, pos, b"<vector>"),
            object::MAP => write_static_to_buffer(buffer, pos, b"<map>"),
            object::VAR => write_static_to_buffer(buffer, pos, b"<var>"),
            object::NAMESPACE => write_static_to_buffer(buffer, pos, b"<namespace>"),
            object::FUN => write_static_to_buffer(buffer, pos, b"<fn>"),
            object::CLOSURE => write_static_to_buffer(buffer, pos, b"<closure>"),
            _ => write_static_to_buffer(buffer, pos, b"<unknown>"),
        }
    } else if term.is_list() {
        // Pair (list)
        write_static_to_buffer(buffer, pos, b"<pair>")
    } else {
        // Other (shouldn't happen with valid Terms)
        write_static_to_buffer(buffer, pos, b"<unknown>")
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

#[inline]
pub const fn intrinsic_is_keyword(x_regs: &XRegs, proc: &Process) -> Term {
    Term::bool(proc.is_term_keyword(x_regs[1]))
}

pub fn intrinsic_keyword<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &Process,
    realm: &mut Realm,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let term = x_regs[1];

    // Copy string to local buffer to avoid borrow conflict
    let mut buffer = [0u8; INTRINSIC_STRING_BUFFER_SIZE];

    // Check if it's a string-like type
    let len = if let Some(s) = proc.read_term_string(mem, term) {
        let len = s.len().min(INTRINSIC_STRING_BUFFER_SIZE);
        buffer[..len].copy_from_slice(&s.as_bytes()[..len]);
        len
    } else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "string, symbol, or keyword",
        });
    };

    let s = core::str::from_utf8(&buffer[..len]).map_err(|_| IntrinsicError::OutOfMemory)?;
    // Keywords are interned at realm level for consistent identity semantics
    realm
        .intern_keyword(mem, s)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Get the name part of a keyword or symbol.
/// For `:ns/name` returns `"name"`, for `:name` returns `"name"`.
pub fn intrinsic_name_fn<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &mut Process,
    realm: &Realm,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let term = x_regs[1];

    // Copy string to local buffer to avoid borrow conflict
    let mut buffer = [0u8; INTRINSIC_STRING_BUFFER_SIZE];
    let len = if let Some(idx) = term.as_keyword_index() {
        // Immediate keyword - look up name from realm
        let Some(s) = realm.keyword_name(mem, idx) else {
            return Err(IntrinsicError::OutOfMemory);
        };
        let len = s.len().min(INTRINSIC_STRING_BUFFER_SIZE);
        buffer[..len].copy_from_slice(&s.as_bytes()[..len]);
        len
    } else if let Some(idx) = term.as_symbol_index() {
        // Immediate symbol - look up name from realm
        let Some(s) = realm.symbol_name(mem, idx) else {
            return Err(IntrinsicError::OutOfMemory);
        };
        let len = s.len().min(INTRINSIC_STRING_BUFFER_SIZE);
        buffer[..len].copy_from_slice(&s.as_bytes()[..len]);
        len
    } else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "keyword or symbol",
        });
    };

    let s = core::str::from_utf8(&buffer[..len]).map_err(|_| IntrinsicError::OutOfMemory)?;

    // Find the last '/' and return everything after it
    let name = s.rfind('/').map_or(s, |pos| &s[pos + 1..]);
    proc.alloc_term_string(mem, name)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Get the namespace part of a qualified keyword or symbol.
/// For `:ns/name` returns `"ns"`, for `:name` returns `nil`.
pub fn intrinsic_namespace<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &mut Process,
    realm: &Realm,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let term = x_regs[1];

    // Copy string to local buffer to avoid borrow conflict
    let mut buffer = [0u8; INTRINSIC_STRING_BUFFER_SIZE];
    let len = if let Some(idx) = term.as_keyword_index() {
        // Immediate keyword - look up name from realm
        let Some(s) = realm.keyword_name(mem, idx) else {
            return Err(IntrinsicError::OutOfMemory);
        };
        let len = s.len().min(INTRINSIC_STRING_BUFFER_SIZE);
        buffer[..len].copy_from_slice(&s.as_bytes()[..len]);
        len
    } else if let Some(idx) = term.as_symbol_index() {
        // Immediate symbol - look up name from realm
        let Some(s) = realm.symbol_name(mem, idx) else {
            return Err(IntrinsicError::OutOfMemory);
        };
        let len = s.len().min(INTRINSIC_STRING_BUFFER_SIZE);
        buffer[..len].copy_from_slice(&s.as_bytes()[..len]);
        len
    } else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "keyword or symbol",
        });
    };

    let s = core::str::from_utf8(&buffer[..len]).map_err(|_| IntrinsicError::OutOfMemory)?;

    // Find the last '/' - if present, return everything before it
    s.rfind('/').map_or_else(
        || Ok(Term::NIL),
        |pos| {
            let ns = &s[..pos];
            proc.alloc_term_string(mem, ns)
                .ok_or(IntrinsicError::OutOfMemory)
        },
    )
}
