// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Sequence intrinsics (first, rest, empty?).
//!
//! These polymorphic intrinsics work on any collection type.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::term::Term;

use super::{IntrinsicError, XRegs};

/// Get the first element of a collection.
///
/// - `(first nil)` → nil
/// - `(first '(1 2 3))` → 1
/// - `(first [1 2 3])` → 1 (tuple)
/// - `(first {1 2 3})` → 1 (vector)
/// - `(first %{:a 1})` → [:a 1]
/// - `(first '())` → nil
pub fn intrinsic_first<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let coll = x_regs[1];

    // Check immediate nil
    if coll.is_nil() {
        return Ok(Term::NIL);
    }

    // Check list (pair)
    if coll.is_list() {
        let (head, _) = proc
            .read_term_pair(mem, coll)
            .ok_or(IntrinsicError::OutOfMemory)?;
        return Ok(head);
    }

    // Check boxed types
    if coll.is_boxed() {
        // Try tuple
        if let Some(len) = proc.read_term_tuple_len(mem, coll) {
            if len == 0 {
                return Ok(Term::NIL);
            }
            return proc
                .read_term_tuple_element(mem, coll, 0)
                .ok_or(IntrinsicError::OutOfMemory);
        }

        // Try vector
        if let Some(len) = proc.read_term_vector_len(mem, coll) {
            if len == 0 {
                return Ok(Term::NIL);
            }
            return proc
                .read_term_vector_element(mem, coll, 0)
                .ok_or(IntrinsicError::OutOfMemory);
        }

        // Try map
        if let Some(entries) = proc.read_term_map_entries(mem, coll) {
            // First entry's pair.first is already a [key value] tuple
            return Ok(proc
                .read_term_pair(mem, entries)
                .map_or(Term::NIL, |(head, _)| head));
        }
    }

    Err(IntrinsicError::TypeError {
        intrinsic: id,
        arg: 0,
        expected: "sequence",
    })
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
    x_regs: &XRegs,
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let coll = x_regs[1];

    // Check immediate nil
    if coll.is_nil() {
        return Ok(Term::NIL);
    }

    // Check list (pair)
    if coll.is_list() {
        let (_, rest) = proc
            .read_term_pair(mem, coll)
            .ok_or(IntrinsicError::OutOfMemory)?;
        return Ok(rest);
    }

    // Check boxed types
    if coll.is_boxed() {
        // Try tuple
        if let Some(len) = proc.read_term_tuple_len(mem, coll) {
            if len <= 1 {
                return Ok(Term::NIL);
            }
            // Build list from elements 1..len (back to front)
            return build_list_from_tuple(proc, mem, coll, 1, len);
        }

        // Try vector
        if let Some(len) = proc.read_term_vector_len(mem, coll) {
            if len <= 1 {
                return Ok(Term::NIL);
            }
            // Build list from elements 1..len (back to front)
            return build_list_from_vector(proc, mem, coll, 1, len);
        }

        // Try map
        if let Some(entries) = proc.read_term_map_entries(mem, coll) {
            // Return the rest of the entries list
            return Ok(proc
                .read_term_pair(mem, entries)
                .map_or(Term::NIL, |(_, rest)| rest));
        }
    }

    Err(IntrinsicError::TypeError {
        intrinsic: id,
        arg: 0,
        expected: "sequence",
    })
}

/// Build a list from indexed elements [start..end) of a tuple.
fn build_list_from_tuple<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    coll: Term,
    start: usize,
    end: usize,
) -> Result<Term, IntrinsicError> {
    let mut result = Term::NIL;
    for i in (start..end).rev() {
        let elem = proc
            .read_term_tuple_element(mem, coll, i)
            .ok_or(IntrinsicError::OutOfMemory)?;
        result = proc
            .alloc_term_pair(mem, elem, result)
            .ok_or(IntrinsicError::OutOfMemory)?;
    }
    Ok(result)
}

/// Build a list from indexed elements [start..end) of a vector.
fn build_list_from_vector<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    coll: Term,
    start: usize,
    end: usize,
) -> Result<Term, IntrinsicError> {
    let mut result = Term::NIL;
    for i in (start..end).rev() {
        let elem = proc
            .read_term_vector_element(mem, coll, i)
            .ok_or(IntrinsicError::OutOfMemory)?;
        result = proc
            .alloc_term_pair(mem, elem, result)
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
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let coll = x_regs[1];

    // Check immediate nil
    if coll.is_nil() {
        return Ok(Term::TRUE);
    }

    // Check list (pair) - pairs are never empty
    if coll.is_list() {
        return Ok(Term::FALSE);
    }

    // Check boxed types
    if coll.is_boxed() {
        // Try tuple
        if let Some(len) = proc.read_term_tuple_len(mem, coll) {
            return Ok(Term::bool(len == 0));
        }

        // Try vector
        if let Some(len) = proc.read_term_vector_len(mem, coll) {
            return Ok(Term::bool(len == 0));
        }

        // Try map
        if let Some(entries) = proc.read_term_map_entries(mem, coll) {
            return Ok(Term::bool(entries.is_nil()));
        }
    }

    Err(IntrinsicError::TypeError {
        intrinsic: id,
        arg: 0,
        expected: "sequence",
    })
}
