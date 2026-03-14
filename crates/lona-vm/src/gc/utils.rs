// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! GC utility functions for object size calculation and address classification.
//!
//! These utilities are used by both minor and major garbage collection.

use crate::Vaddr;
use crate::process::Process;
use crate::term::Term;
use crate::term::header::Header;

/// Calculate the size in bytes of a heap object from its header.
///
/// This is a thin wrapper around `Header::object_size()` for GC convenience.
///
/// For objects with headers (BOXED tag), this reads the header and computes
/// the total allocation size including the header itself.
///
/// # Note
///
/// Pairs (LIST tag) do not have headers and are always 16 bytes.
/// Use `Pair::SIZE` directly for pairs.
#[inline]
#[must_use]
pub const fn object_size_from_header(header: Header) -> usize {
    header.object_size()
}

/// Check if an address is within the young heap.
///
/// Returns true if `addr` is in the range `[heap, hend)`.
#[inline]
#[must_use]
pub const fn is_in_young_heap(process: &Process, addr: Vaddr) -> bool {
    let addr_val = addr.as_u64();
    addr_val >= process.heap.as_u64() && addr_val < process.hend.as_u64()
}

/// Check if an address is within the old heap.
///
/// Returns true if `addr` is in the range `[old_heap, old_hend)`.
#[inline]
#[must_use]
pub const fn is_in_old_heap(process: &Process, addr: Vaddr) -> bool {
    let addr_val = addr.as_u64();
    addr_val >= process.old_heap.as_u64() && addr_val < process.old_hend.as_u64()
}

/// Check if a Term needs GC tracing.
///
/// Returns true if the term is a heap pointer (LIST or BOXED tag).
/// Immediate values (integers, symbols, keywords, nil, booleans) do not
/// need tracing as they don't contain heap references.
///
/// Note: Symbols and keywords are immediate values with indices into
/// realm tables - they are not heap-allocated in process memory.
#[inline]
#[must_use]
pub const fn needs_tracing(term: Term) -> bool {
    term.is_list() || term.is_boxed()
}

/// Check if a boxed object has been forwarded during GC.
///
/// During garbage collection, when an object is copied to a new location,
/// a forwarding header replaces the original header. This function checks
/// if the header indicates a forwarding pointer.
#[inline]
#[must_use]
pub const fn is_forwarded_boxed(header: Header) -> bool {
    header.is_forward()
}

/// Get the forwarding address from a forwarding header.
///
/// # Safety
///
/// Caller must ensure this is a forwarding header (`is_forwarded_boxed` returns true).
#[inline]
#[must_use]
pub fn forward_address_boxed(header: Header) -> Vaddr {
    Vaddr::new(header.forward_address() as u64)
}

/// Create a forwarding header pointing to a new address.
///
/// The address must be 8-byte aligned.
#[inline]
#[must_use]
pub fn make_forward_header(new_addr: Vaddr) -> Header {
    Header::forward(new_addr.as_u64() as *const u8)
}
