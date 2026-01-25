// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Arithmetic and comparison intrinsics.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::term::Term;

use super::{IntrinsicError, XRegs, expect_int};

// --- Arithmetic intrinsics ---

pub fn intrinsic_add(x_regs: &XRegs, id: u8) -> Result<Term, IntrinsicError> {
    let a = expect_int(x_regs, 1, id, 0)?;
    let b = expect_int(x_regs, 2, id, 1)?;
    let result = a.checked_add(b).ok_or(IntrinsicError::Overflow)?;
    Term::small_int(result).ok_or(IntrinsicError::Overflow)
}

pub fn intrinsic_sub(x_regs: &XRegs, id: u8) -> Result<Term, IntrinsicError> {
    let a = expect_int(x_regs, 1, id, 0)?;
    let b = expect_int(x_regs, 2, id, 1)?;
    let result = a.checked_sub(b).ok_or(IntrinsicError::Overflow)?;
    Term::small_int(result).ok_or(IntrinsicError::Overflow)
}

pub fn intrinsic_mul(x_regs: &XRegs, id: u8) -> Result<Term, IntrinsicError> {
    let a = expect_int(x_regs, 1, id, 0)?;
    let b = expect_int(x_regs, 2, id, 1)?;
    let result = a.checked_mul(b).ok_or(IntrinsicError::Overflow)?;
    Term::small_int(result).ok_or(IntrinsicError::Overflow)
}

pub fn intrinsic_div(x_regs: &XRegs, id: u8) -> Result<Term, IntrinsicError> {
    let a = expect_int(x_regs, 1, id, 0)?;
    let b = expect_int(x_regs, 2, id, 1)?;
    if b == 0 {
        return Err(IntrinsicError::DivisionByZero);
    }
    // Division can overflow in one case: i64::MIN / -1
    let result = a.checked_div(b).ok_or(IntrinsicError::Overflow)?;
    Term::small_int(result).ok_or(IntrinsicError::Overflow)
}

pub fn intrinsic_mod(x_regs: &XRegs, id: u8) -> Result<Term, IntrinsicError> {
    let a = expect_int(x_regs, 1, id, 0)?;
    let b = expect_int(x_regs, 2, id, 1)?;
    if b == 0 {
        return Err(IntrinsicError::DivisionByZero);
    }
    // Modulus: result has same sign as divisor (b)
    // This differs from remainder where sign follows dividend (a)
    // checked_rem can overflow in one case: i64::MIN % -1
    let rem = a.checked_rem(b).ok_or(IntrinsicError::Overflow)?;
    let result = if (rem < 0 && b > 0) || (rem > 0 && b < 0) {
        rem.checked_add(b).ok_or(IntrinsicError::Overflow)?
    } else {
        rem
    };
    Term::small_int(result).ok_or(IntrinsicError::Overflow)
}

// --- Comparison intrinsics ---

pub fn intrinsic_eq<M: MemorySpace>(x_regs: &XRegs, proc: &Process, mem: &M) -> Term {
    let a = x_regs[1];
    let b = x_regs[2];
    Term::bool(terms_equal(a, b, proc, mem))
}

/// Reference identity comparison.
///
/// Returns true if two values are the exact same object (same bits for
/// all values - immediates compare directly, heap pointers compare addresses).
pub const fn intrinsic_identical(x_regs: &XRegs) -> Term {
    let a = x_regs[1];
    let b = x_regs[2];
    Term::bool(terms_identical(a, b))
}

/// Check if two terms are identical (reference equality).
///
/// For Term representation, this is simply bit equality - same tag + payload.
/// Heap-allocated values are identical only if they point to the same address.
const fn terms_identical(a: Term, b: Term) -> bool {
    a.as_raw() == b.as_raw()
}

/// Maximum recursion depth for structural equality to prevent stack overflow.
const MAX_EQ_DEPTH: usize = 64;

/// Compare two terms for equality.
///
/// - Immediate values (nil, bool, int) compare by value
/// - Strings compare by content
/// - Keywords compare by content
/// - Symbols compare by content (fast path: address comparison for interned symbols)
/// - Pairs compare structurally (recursive element comparison)
/// - Tuples compare structurally (recursive element comparison)
/// - Maps compare structurally (same keys with equal values)
pub fn terms_equal<M: MemorySpace>(a: Term, b: Term, proc: &Process, mem: &M) -> bool {
    terms_equal_depth(a, b, proc, mem, 0)
}

fn terms_equal_depth<M: MemorySpace>(
    a: Term,
    b: Term,
    proc: &Process,
    mem: &M,
    depth: usize,
) -> bool {
    if depth > MAX_EQ_DEPTH {
        return false;
    }

    // Fast path: identical bits means equal
    if a.as_raw() == b.as_raw() {
        return true;
    }

    // Different bits - check if they could still be structurally equal

    // Immediates with different bits are not equal
    if a.is_immediate() && b.is_immediate() {
        // Both immediates but different bits means not equal
        // (small_int, symbol index, keyword index, or special values)
        return false;
    }

    // Check for list pairs
    if a.is_list() && b.is_list() {
        // Both are pairs - compare structurally
        let Some((head_a, rest_a)) = proc.read_term_pair(mem, a) else {
            return false;
        };
        let Some((head_b, rest_b)) = proc.read_term_pair(mem, b) else {
            return false;
        };
        return terms_equal_depth(head_a, head_b, proc, mem, depth + 1)
            && terms_equal_depth(rest_a, rest_b, proc, mem, depth + 1);
    }

    // Check for boxed values
    if a.is_boxed() && b.is_boxed() {
        return boxed_equal(a, b, proc, mem, depth);
    }

    // One is boxed, one is not (or one is list, one is not) - not equal
    false
}

/// Compare two boxed values for structural equality.
fn boxed_equal<M: MemorySpace>(a: Term, b: Term, proc: &Process, mem: &M, depth: usize) -> bool {
    use crate::term::header::Header;
    use crate::term::tag::object;

    let addr_a = a.to_vaddr();
    let addr_b = b.to_vaddr();

    // Read headers to determine types
    let header_a: Header = mem.read(addr_a);
    let header_b: Header = mem.read(addr_b);

    let tag_a = header_a.object_tag();
    let tag_b = header_b.object_tag();

    // Different types are never equal
    if tag_a != tag_b {
        return false;
    }

    match tag_a {
        object::STRING => {
            // Compare string contents
            let Some(sa) = proc.read_term_string(mem, a) else {
                return false;
            };
            let Some(sb) = proc.read_term_string(mem, b) else {
                return false;
            };
            sa == sb
        }
        object::TUPLE => {
            // Compare tuples structurally
            tuple_equal(a, b, proc, mem, depth)
        }
        object::VECTOR => {
            // Compare vectors structurally
            vector_equal(a, b, proc, mem, depth)
        }
        object::MAP => {
            // Compare maps structurally
            maps_equal(proc, mem, a, b, depth + 1)
        }
        // Functions, namespaces, and unknown types compare by identity (already checked above)
        _ => false,
    }
}

/// Compare two tuples for structural equality.
fn tuple_equal<M: MemorySpace>(a: Term, b: Term, proc: &Process, mem: &M, depth: usize) -> bool {
    let Some(len_a) = proc.read_term_tuple_len(mem, a) else {
        return false;
    };
    let Some(len_b) = proc.read_term_tuple_len(mem, b) else {
        return false;
    };
    if len_a != len_b {
        return false;
    }
    for i in 0..len_a {
        let Some(ea) = proc.read_term_tuple_element(mem, a, i) else {
            return false;
        };
        let Some(eb) = proc.read_term_tuple_element(mem, b, i) else {
            return false;
        };
        if !terms_equal_depth(ea, eb, proc, mem, depth + 1) {
            return false;
        }
    }
    true
}

/// Compare two vectors for structural equality.
fn vector_equal<M: MemorySpace>(a: Term, b: Term, proc: &Process, mem: &M, depth: usize) -> bool {
    let Some(len_a) = proc.read_term_vector_len(mem, a) else {
        return false;
    };
    let Some(len_b) = proc.read_term_vector_len(mem, b) else {
        return false;
    };
    if len_a != len_b {
        return false;
    }
    for i in 0..len_a {
        let Some(ea) = proc.read_term_vector_element(mem, a, i) else {
            return false;
        };
        let Some(eb) = proc.read_term_vector_element(mem, b, i) else {
            return false;
        };
        if !terms_equal_depth(ea, eb, proc, mem, depth + 1) {
            return false;
        }
    }
    true
}

/// Count entries in a map's entry list.
fn count_map_entries<M: MemorySpace>(proc: &Process, mem: &M, mut entries: Term) -> usize {
    let mut count = 0;
    while let Some((_, rest)) = proc.read_term_pair(mem, entries) {
        count += 1;
        entries = rest;
    }
    count
}

/// Look up a key in a map, returning the value if found.
#[allow(dead_code)]
fn map_lookup<'a, M: MemorySpace>(
    proc: &'a Process,
    mem: &'a M,
    map_term: Term,
) -> Option<impl Iterator<Item = (Term, Term)> + 'a> {
    let entries = proc.read_term_map_entries(mem, map_term)?;
    Some(MapIter {
        proc,
        mem,
        current: entries,
    })
}

/// Iterator over map entries.
#[allow(dead_code)]
struct MapIter<'a, M> {
    proc: &'a Process,
    mem: &'a M,
    current: Term,
}

impl<M: MemorySpace> Iterator for MapIter<'_, M> {
    type Item = (Term, Term);

    fn next(&mut self) -> Option<Self::Item> {
        let (entry, rest) = self.proc.read_term_pair(self.mem, self.current)?;
        self.current = rest;

        // Each entry is a [key value] tuple
        let key = self.proc.read_term_tuple_element(self.mem, entry, 0)?;
        let value = self.proc.read_term_tuple_element(self.mem, entry, 1)?;
        Some((key, value))
    }
}

/// Compare two maps for structural equality.
fn maps_equal<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    map_a: Term,
    map_b: Term,
    depth: usize,
) -> bool {
    let Some(entries_a) = proc.read_term_map_entries(mem, map_a) else {
        return false;
    };
    let Some(entries_b) = proc.read_term_map_entries(mem, map_b) else {
        return false;
    };

    // Count entries in both maps
    let count_a = count_map_entries(proc, mem, entries_a);
    let count_b = count_map_entries(proc, mem, entries_b);
    if count_a != count_b {
        return false;
    }

    // Check each entry in map_a exists in map_b with equal value
    let mut current = entries_a;
    while let Some((entry, rest)) = proc.read_term_pair(mem, current) {
        // Each entry is a [key value] tuple
        let Some(key) = proc.read_term_tuple_element(mem, entry, 0) else {
            return false;
        };
        let Some(val_a) = proc.read_term_tuple_element(mem, entry, 1) else {
            return false;
        };

        // Look up key in map_b
        let mut found = false;
        let mut search = entries_b;
        while let Some((search_entry, search_rest)) = proc.read_term_pair(mem, search) {
            let Some(search_key) = proc.read_term_tuple_element(mem, search_entry, 0) else {
                return false;
            };
            if terms_equal_depth(key, search_key, proc, mem, depth) {
                // Found the key, compare values
                let Some(val_b) = proc.read_term_tuple_element(mem, search_entry, 1) else {
                    return false;
                };
                if !terms_equal_depth(val_a, val_b, proc, mem, depth) {
                    return false;
                }
                found = true;
                break;
            }
            search = search_rest;
        }
        if !found {
            return false;
        }
        current = rest;
    }
    true
}

pub fn intrinsic_lt(x_regs: &XRegs, id: u8) -> Result<Term, IntrinsicError> {
    let a = expect_int(x_regs, 1, id, 0)?;
    let b = expect_int(x_regs, 2, id, 1)?;
    Ok(Term::bool(a < b))
}

pub fn intrinsic_gt(x_regs: &XRegs, id: u8) -> Result<Term, IntrinsicError> {
    let a = expect_int(x_regs, 1, id, 0)?;
    let b = expect_int(x_regs, 2, id, 1)?;
    Ok(Term::bool(a > b))
}

pub fn intrinsic_le(x_regs: &XRegs, id: u8) -> Result<Term, IntrinsicError> {
    let a = expect_int(x_regs, 1, id, 0)?;
    let b = expect_int(x_regs, 2, id, 1)?;
    Ok(Term::bool(a <= b))
}

pub fn intrinsic_ge(x_regs: &XRegs, id: u8) -> Result<Term, IntrinsicError> {
    let a = expect_int(x_regs, 1, id, 0)?;
    let b = expect_int(x_regs, 2, id, 1)?;
    Ok(Term::bool(a >= b))
}

// --- Boolean intrinsic ---

pub const fn intrinsic_not(x_regs: &XRegs) -> Term {
    // (not x) returns true if x is falsy (nil or false), false otherwise
    Term::bool(!x_regs[1].is_truthy())
}
