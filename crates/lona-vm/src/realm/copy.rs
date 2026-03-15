// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Deep copy infrastructure for copying Terms from process heap to realm code region.
//!
//! When `def` stores a value in the realm, all heap-allocated data must be copied
//! to the realm's code region. This module provides the deep copy machinery.
//!
//! Both process heap and realm code region exist within the same `MemorySpace`.
//! Deep copy allocates in the realm's region and writes to the same `mem`.

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::{
    HeapClosure, HeapFun, HeapMap, HeapPair, HeapString, HeapTuple, HeapVector,
};
use crate::term::tag::object;

use super::Realm;

/// Maximum depth for visited tracking (prevents stack overflow on deeply nested data).
const MAX_VISITED: usize = 256;

/// Tracks visited addresses during deep copy to handle shared structure.
///
/// When the same heap object is referenced multiple times in a value graph,
/// we only copy it once and reuse the destination address for subsequent references.
pub struct VisitedTracker {
    /// Source addresses that have been copied.
    src: [Vaddr; MAX_VISITED],
    /// Corresponding destination addresses.
    dst: [Vaddr; MAX_VISITED],
    /// Number of tracked entries.
    len: usize,
}

impl VisitedTracker {
    /// Create a new empty tracker.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            src: [Vaddr::new(0); MAX_VISITED],
            dst: [Vaddr::new(0); MAX_VISITED],
            len: 0,
        }
    }

    /// Check if a source address has already been copied.
    ///
    /// Returns the destination address if found, `None` otherwise.
    #[must_use]
    pub fn check(&self, src_addr: Vaddr) -> Option<Vaddr> {
        for i in 0..self.len {
            if self.src[i] == src_addr {
                return Some(self.dst[i]);
            }
        }
        None
    }

    /// Record a source→destination mapping.
    ///
    /// Returns `false` if the tracker is full (should not happen in practice
    /// as `MAX_VISITED` is sized for typical use cases).
    pub const fn record(&mut self, src_addr: Vaddr, dst_addr: Vaddr) -> bool {
        if self.len >= MAX_VISITED {
            return false;
        }
        self.src[self.len] = src_addr;
        self.dst[self.len] = dst_addr;
        self.len += 1;
        true
    }
}

impl Default for VisitedTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Deep copy a Term from process heap to realm code region.
///
/// This is the main entry point for copying Terms to the realm. It handles
/// all term types, recursively copying heap-allocated structures.
///
/// # Arguments
/// * `term` - The Term to copy
/// * `realm` - The realm to allocate in
/// * `mem` - The memory space (shared between process and realm)
/// * `visited` - Tracker for already-copied addresses
///
/// # Returns
/// The copied Term with all pointers updated to realm addresses,
/// or `None` if allocation fails.
pub fn deep_copy_term_to_realm<M: MemorySpace>(
    term: Term,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    // Immediates: no copy needed, pass through directly
    if term.is_immediate() {
        return Some(term);
    }

    // Nil: pass through
    if term.is_nil() {
        return Some(term);
    }

    // List (cons cell): recursive copy
    if term.is_list() {
        let addr = term.to_vaddr();
        if let Some(dst) = visited.check(addr) {
            return Some(Term::list_vaddr(dst));
        }
        return deep_copy_pair(addr, realm, mem, visited);
    }

    // Boxed values: check header to determine type
    if term.is_boxed() {
        let addr = term.to_vaddr();

        // Check visited first
        if let Some(dst) = visited.check(addr) {
            return Some(Term::boxed_vaddr(dst));
        }

        // Read header to determine type
        let header: Header = mem.read(addr);
        let tag = header.object_tag();

        return match tag {
            object::STRING => deep_copy_string(addr, realm, mem, visited),
            // Note: SYMBOL and KEYWORD are now immediate values, not boxed
            object::TUPLE => deep_copy_tuple(addr, realm, mem, visited),
            object::VECTOR => deep_copy_vector(addr, realm, mem, visited),
            object::MAP => deep_copy_map(addr, realm, mem, visited),
            object::FUN => deep_copy_fun(addr, realm, mem, visited),
            object::CLOSURE => deep_copy_closure(addr, realm, mem, visited),
            object::VAR | object::NAMESPACE => {
                // Vars and Namespaces are already in realm, pass through
                Some(term)
            }
            _ => None, // Unknown object type
        };
    }

    // Unknown term type
    None
}

/// Deep copy a string to the realm's code region.
fn deep_copy_string<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    // Read source header
    let header: Header = mem.read(src_addr);
    let len = header.arity() as usize;
    let total_size = HeapString::alloc_size(len);

    // Allocate in realm
    let dst_addr = realm.alloc(total_size, 8)?;
    visited.record(src_addr, dst_addr);

    // Copy header
    mem.write(dst_addr, header);

    // Copy string data byte by byte to avoid borrow conflict
    let src_data = src_addr.add(HeapString::HEADER_SIZE as u64);
    let dst_data = dst_addr.add(HeapString::HEADER_SIZE as u64);

    for i in 0..len {
        let byte: u8 = mem.read(src_data.add(i as u64));
        mem.write(dst_data.add(i as u64), byte);
    }

    Some(Term::boxed_vaddr(dst_addr))
}

// Note: deep_copy_symbol and deep_copy_keyword removed - symbols/keywords are now
// immediate values with indices into the realm's intern tables. Within the same realm,
// they pass through unchanged. Cross-realm copy requires re-interning (see 8.7).

/// Deep copy a pair (cons cell) to the realm's code region.
fn deep_copy_pair<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    // Allocate in realm first (record early to handle cycles)
    let dst_addr = realm.alloc(HeapPair::SIZE, 8)?;
    visited.record(src_addr, dst_addr);

    // Read source pair
    let pair: HeapPair = mem.read(src_addr);

    // Deep copy head and tail
    let dst_head = deep_copy_term_to_realm(pair.head, realm, mem, visited)?;
    let dst_tail = deep_copy_term_to_realm(pair.tail, realm, mem, visited)?;

    // Write destination pair
    let dst_pair = HeapPair {
        head: dst_head,
        tail: dst_tail,
    };
    mem.write(dst_addr, dst_pair);

    Some(Term::list_vaddr(dst_addr))
}

/// Deep copy a tuple to the realm's code region.
fn deep_copy_tuple<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    // Read source header
    let header: Header = mem.read(src_addr);
    let len = header.arity() as usize;
    let total_size = HeapTuple::alloc_size(len);

    // Allocate in realm first
    let dst_addr = realm.alloc(total_size, 8)?;
    visited.record(src_addr, dst_addr);

    // Write header
    mem.write(dst_addr, header);

    // Deep copy each element
    let src_elements = src_addr.add(HeapTuple::HEADER_SIZE as u64);
    let dst_elements = dst_addr.add(HeapTuple::HEADER_SIZE as u64);

    for i in 0..len {
        let offset = (i * core::mem::size_of::<Term>()) as u64;
        let src_elem: Term = mem.read(src_elements.add(offset));
        let dst_elem = deep_copy_term_to_realm(src_elem, realm, mem, visited)?;
        mem.write(dst_elements.add(offset), dst_elem);
    }

    Some(Term::boxed_vaddr(dst_addr))
}

/// Deep copy a vector to the realm's code region.
fn deep_copy_vector<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    // Read source header
    let header: Header = mem.read(src_addr);
    let len = header.arity() as usize;
    let total_size = HeapVector::alloc_size(len);

    // Allocate in realm first
    let dst_addr = realm.alloc(total_size, 8)?;
    visited.record(src_addr, dst_addr);

    // Write header
    mem.write(dst_addr, header);

    // Deep copy each element
    // Note: Using HeapTuple layout (header + elements) for vectors during migration
    let src_elements = src_addr.add(HeapTuple::HEADER_SIZE as u64);
    let dst_elements = dst_addr.add(HeapTuple::HEADER_SIZE as u64);

    for i in 0..len {
        let offset = (i * core::mem::size_of::<Term>()) as u64;
        let src_elem: Term = mem.read(src_elements.add(offset));
        let dst_elem = deep_copy_term_to_realm(src_elem, realm, mem, visited)?;
        mem.write(dst_elements.add(offset), dst_elem);
    }

    Some(Term::boxed_vaddr(dst_addr))
}

/// Deep copy a map to the realm's code region.
fn deep_copy_map<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    // Read source map header
    let header: Header = mem.read(src_addr);
    let entries_addr = src_addr.add(8);
    let entries: Term = mem.read(entries_addr);

    // Allocate in realm first
    let dst_addr = realm.alloc(HeapMap::SIZE, 8)?;
    visited.record(src_addr, dst_addr);

    // Deep copy the entries list (association list of pairs)
    let dst_entries = deep_copy_term_to_realm(entries, realm, mem, visited)?;

    // Write header
    mem.write(dst_addr, header);

    // Write entries
    let dst_entries_addr = dst_addr.add(8);
    mem.write(dst_entries_addr, dst_entries);

    Some(Term::boxed_vaddr(dst_addr))
}

/// Deep copy a function to the realm's code region.
fn deep_copy_fun<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    // Read source header
    let header: HeapFun = mem.read(src_addr);
    let code_len = header.code_len as usize;
    let const_count = header.const_count as usize;
    let total_size = HeapFun::alloc_size(code_len, const_count);

    // Allocate in realm first
    let dst_addr = realm.alloc(total_size, 8)?;
    visited.record(src_addr, dst_addr);

    // Copy header (metadata is value-only, no pointers to deep copy)
    mem.write(dst_addr, header);

    // Copy bytecode (no deep copy needed, just raw bytes)
    let src_code = src_addr.add(HeapFun::PREFIX_SIZE as u64);
    let dst_code = dst_addr.add(HeapFun::PREFIX_SIZE as u64);
    for i in 0..code_len {
        let byte: u8 = mem.read(src_code.add(i as u64));
        mem.write(dst_code.add(i as u64), byte);
    }

    // Deep copy constants (at aligned offset after bytecode)
    let constants_offset = HeapFun::constants_offset(code_len);
    let src_constants = src_addr.add(constants_offset as u64);
    let dst_constants = dst_addr.add(constants_offset as u64);
    for i in 0..const_count {
        let offset = (i * core::mem::size_of::<Term>()) as u64;
        let src_const: Term = mem.read(src_constants.add(offset));
        let dst_const = deep_copy_term_to_realm(src_const, realm, mem, visited)?;
        mem.write(dst_constants.add(offset), dst_const);
    }

    Some(Term::boxed_vaddr(dst_addr))
}

/// Deep copy a closure to the realm's code region.
fn deep_copy_closure<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    // Read source closure header
    let header: HeapClosure = mem.read(src_addr);
    let captures_len = header.capture_count();
    let total_size = HeapClosure::alloc_size(captures_len);

    // Allocate in realm first
    let dst_addr = realm.alloc(total_size, 8)?;
    visited.record(src_addr, dst_addr);

    // Deep copy the underlying function (header.function is a Term)
    let dst_func_term = deep_copy_term_to_realm(header.function, realm, mem, visited)?;

    // Write header with new function pointer
    let dst_header = HeapClosure {
        header: HeapClosure::make_header(captures_len),
        function: dst_func_term,
    };
    mem.write(dst_addr, dst_header);

    // Deep copy captures (follows header + function term)
    let src_captures = src_addr.add(HeapClosure::PREFIX_SIZE as u64);
    let dst_captures = dst_addr.add(HeapClosure::PREFIX_SIZE as u64);
    for i in 0..captures_len {
        let offset = (i * core::mem::size_of::<Term>()) as u64;
        let src_cap: Term = mem.read(src_captures.add(offset));
        let dst_cap = deep_copy_term_to_realm(src_cap, realm, mem, visited)?;
        mem.write(dst_captures.add(offset), dst_cap);
    }

    Some(Term::boxed_vaddr(dst_addr))
}

// ============================================================================
// Process-to-Process Deep Copy (for spawn)
// ============================================================================

use crate::process::Process;

/// Deep copy a Term from one process's heap to another's.
///
/// Used by `spawn` to copy the function (and its captures/constants)
/// to the new process's heap. Both processes share the same `MemorySpace`.
pub fn deep_copy_term_to_process<M: MemorySpace>(
    term: Term,
    _src_proc: &Process,
    dst_proc: &mut Process,
    mem: &mut M,
) -> Option<Term> {
    let mut visited = VisitedTracker::new();
    deep_copy_to_proc(term, dst_proc, mem, &mut visited)
}

/// Recursive deep copy to a process heap.
fn deep_copy_to_proc<M: MemorySpace>(
    term: Term,
    dst_proc: &mut Process,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    if term.is_immediate() || term.is_nil() {
        return Some(term);
    }

    if term.is_list() {
        let addr = term.to_vaddr();
        if let Some(dst) = visited.check(addr) {
            return Some(Term::list_vaddr(dst));
        }
        let pair: HeapPair = mem.read(addr);
        let dst_addr = dst_proc.alloc(HeapPair::SIZE, 8)?;
        visited.record(addr, dst_addr);
        let head = deep_copy_to_proc(pair.head, dst_proc, mem, visited)?;
        let tail = deep_copy_to_proc(pair.tail, dst_proc, mem, visited)?;
        let dst_pair = HeapPair { head, tail };
        mem.write(dst_addr, dst_pair);
        return Some(Term::list_vaddr(dst_addr));
    }

    if term.is_boxed() {
        let addr = term.to_vaddr();
        if let Some(dst) = visited.check(addr) {
            return Some(Term::boxed_vaddr(dst));
        }
        return deep_copy_boxed_to_proc(term, addr, dst_proc, mem, visited);
    }

    None
}

/// Deep copy a boxed term to a process heap.
fn deep_copy_boxed_to_proc<M: MemorySpace>(
    original_term: Term,
    addr: Vaddr,
    dst_proc: &mut Process,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    let header: Header = mem.read(addr);
    let tag = header.object_tag();

    match tag {
        object::STRING => {
            let len = header.arity() as usize;
            let total = HeapString::alloc_size(len);
            let dst_addr = dst_proc.alloc(total, 8)?;
            visited.record(addr, dst_addr);
            mem.write(dst_addr, header);
            let src_data = addr.add(HeapString::HEADER_SIZE as u64);
            let dst_data = dst_addr.add(HeapString::HEADER_SIZE as u64);
            for i in 0..len {
                let byte: u8 = mem.read(src_data.add(i as u64));
                mem.write(dst_data.add(i as u64), byte);
            }
            Some(Term::boxed_vaddr(dst_addr))
        }
        object::TUPLE => {
            let len = header.arity() as usize;
            let total = HeapTuple::alloc_size(len);
            let dst_addr = dst_proc.alloc(total, 8)?;
            visited.record(addr, dst_addr);
            mem.write(dst_addr, header);
            let src_data = addr.add(HeapTuple::HEADER_SIZE as u64);
            let dst_data = dst_addr.add(HeapTuple::HEADER_SIZE as u64);
            for i in 0..len {
                let offset = (i * 8) as u64;
                let elem: Term = mem.read(src_data.add(offset));
                let copied = deep_copy_to_proc(elem, dst_proc, mem, visited)?;
                mem.write(dst_data.add(offset), copied);
            }
            Some(Term::boxed_vaddr(dst_addr))
        }
        object::VECTOR => {
            let len_val: u64 = mem.read(addr.add(8));
            let len = len_val as usize;
            let total = HeapVector::alloc_size(len);
            let dst_addr = dst_proc.alloc(total, 8)?;
            visited.record(addr, dst_addr);
            mem.write(dst_addr, header);
            mem.write(dst_addr.add(8), len_val);
            let src_data = addr.add(HeapVector::PREFIX_SIZE as u64);
            let dst_data = dst_addr.add(HeapVector::PREFIX_SIZE as u64);
            for i in 0..len {
                let offset = (i * 8) as u64;
                let elem: Term = mem.read(src_data.add(offset));
                let copied = deep_copy_to_proc(elem, dst_proc, mem, visited)?;
                mem.write(dst_data.add(offset), copied);
            }
            Some(Term::boxed_vaddr(dst_addr))
        }
        object::MAP => {
            let entries: Term = mem.read(addr.add(8));
            let dst_addr = dst_proc.alloc(HeapMap::SIZE, 8)?;
            visited.record(addr, dst_addr);
            let copied_entries = deep_copy_to_proc(entries, dst_proc, mem, visited)?;
            mem.write(dst_addr, header);
            mem.write(dst_addr.add(8), copied_entries);
            Some(Term::boxed_vaddr(dst_addr))
        }
        object::FUN => copy_fun_to_proc(addr, dst_proc, mem, visited),
        object::CLOSURE => copy_closure_to_proc(addr, dst_proc, mem, visited),
        object::VAR | object::NAMESPACE => Some(original_term),
        _ => None,
    }
}

/// Copy a compiled function to a process heap.
fn copy_fun_to_proc<M: MemorySpace>(
    addr: Vaddr,
    dst_proc: &mut Process,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    let fun: HeapFun = mem.read(addr);
    let code_len = fun.code_len as usize;
    let const_count = fun.const_count as usize;
    let total = HeapFun::alloc_size(code_len, const_count);
    let dst_addr = dst_proc.alloc(total, 8)?;
    visited.record(addr, dst_addr);
    mem.write(dst_addr, fun);

    let src_code = addr.add(HeapFun::PREFIX_SIZE as u64);
    let dst_code = dst_addr.add(HeapFun::PREFIX_SIZE as u64);
    for i in 0..code_len {
        let byte: u8 = mem.read(src_code.add(i as u64));
        mem.write(dst_code.add(i as u64), byte);
    }

    let const_offset = HeapFun::constants_offset(code_len);
    let src_const = addr.add(const_offset as u64);
    let dst_const = dst_addr.add(const_offset as u64);
    for i in 0..const_count {
        let offset = (i * core::mem::size_of::<Term>()) as u64;
        let c: Term = mem.read(src_const.add(offset));
        let copied = deep_copy_to_proc(c, dst_proc, mem, visited)?;
        mem.write(dst_const.add(offset), copied);
    }

    Some(Term::boxed_vaddr(dst_addr))
}

/// Copy a closure to a process heap.
fn copy_closure_to_proc<M: MemorySpace>(
    addr: Vaddr,
    dst_proc: &mut Process,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term> {
    let closure: HeapClosure = mem.read(addr);
    let captures_len = closure.capture_count();
    let total = HeapClosure::alloc_size(captures_len);
    let dst_addr = dst_proc.alloc(total, 8)?;
    visited.record(addr, dst_addr);

    let dst_func = deep_copy_to_proc(closure.function, dst_proc, mem, visited)?;
    let dst_hdr = HeapClosure {
        header: HeapClosure::make_header(captures_len),
        function: dst_func,
    };
    mem.write(dst_addr, dst_hdr);

    let src_caps = addr.add(HeapClosure::PREFIX_SIZE as u64);
    let dst_caps = dst_addr.add(HeapClosure::PREFIX_SIZE as u64);
    for i in 0..captures_len {
        let offset = (i * core::mem::size_of::<Term>()) as u64;
        let cap: Term = mem.read(src_caps.add(offset));
        let copied = deep_copy_to_proc(cap, dst_proc, mem, visited)?;
        mem.write(dst_caps.add(offset), copied);
    }

    Some(Term::boxed_vaddr(dst_addr))
}
