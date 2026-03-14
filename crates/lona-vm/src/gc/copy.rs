// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Object copying for garbage collection using Cheney's algorithm.
//!
//! Cheney's algorithm is a semi-space copying collector that uses two memory regions:
//! - **From-space**: Where the live objects currently reside
//! - **To-space**: Where live objects will be copied
//!
//! The algorithm works by:
//! 1. Copy roots from from-space to to-space
//! 2. Scan copied objects in to-space, copying any referenced objects
//! 3. Repeat step 2 until `scan_ptr` catches up with `alloc_ptr`
//!
//! Objects in from-space are replaced with forwarding pointers to prevent
//! double-copying and enable pointer updates.

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::term::Term;
use crate::term::header::Header;
use crate::term::pair::Pair;
use crate::term::tag::object;

use super::GcError;
use super::utils::{is_in_old_heap, is_in_young_heap, needs_tracing, object_size_from_header};

/// Copier state for Cheney's algorithm.
///
/// The copier manages the to-space allocation and scan pointers.
/// It copies objects from the process's young/old heaps to the to-space.
pub struct Copier {
    /// Start of to-space (allocation begins here).
    pub to_space_start: Vaddr,
    /// End of to-space (exclusive).
    pub to_space_end: Vaddr,
    /// Where to allocate next in to-space.
    pub alloc_ptr: Vaddr,
    /// Where to scan next in to-space (Cheney's scan pointer).
    pub scan_ptr: Vaddr,
}

impl Copier {
    /// Create a new copier targeting the given to-space region.
    #[must_use]
    pub const fn new(to_space_start: Vaddr, to_space_end: Vaddr) -> Self {
        Self {
            to_space_start,
            to_space_end,
            alloc_ptr: to_space_start,
            scan_ptr: to_space_start,
        }
    }

    /// Get the number of bytes copied so far.
    #[must_use]
    pub const fn bytes_copied(&self) -> usize {
        (self.alloc_ptr.as_u64() - self.to_space_start.as_u64()) as usize
    }

    /// Allocate space in to-space for an object of given size.
    ///
    /// Returns the address where the object should be written,
    /// or `None` if there isn't enough space.
    pub const fn allocate(&mut self, size: usize) -> Option<Vaddr> {
        let new_alloc = Vaddr::new(self.alloc_ptr.as_u64() + size as u64);
        if new_alloc.as_u64() > self.to_space_end.as_u64() {
            return None;
        }
        let addr = self.alloc_ptr;
        self.alloc_ptr = new_alloc;
        Some(addr)
    }

    /// Scan all copied objects in to-space, copying any objects they reference.
    ///
    /// This is the main loop of Cheney's algorithm. It scans objects from
    /// `scan_ptr` to `alloc_ptr`, copying any referenced objects and updating
    /// the pointers to point to the new locations.
    ///
    /// # Errors
    ///
    /// Returns `GcError::NeedsMajorGc` if to-space runs out of space during copying.
    pub fn scan_copied_objects<M: MemorySpace>(
        &mut self,
        process: &Process,
        mem: &mut M,
    ) -> Result<(), GcError> {
        while self.scan_ptr.as_u64() < self.alloc_ptr.as_u64() {
            // Determine what kind of object is at scan_ptr
            // We need to figure out if it's a pair or a boxed object
            // by looking at what we copied there

            // Read the first 8 bytes
            let first_word: u64 = mem.read(self.scan_ptr);

            // Check if it's a header (boxed object) or a term (pair)
            // Headers have primary tag 00, which is also HEADER
            // Pairs don't have headers, so the first word is the head Term

            // SAFETY: first_word was read from a valid to-space address (scan_ptr).
            // It represents either a header word or the head of a pair that we
            // previously copied. The bit pattern is valid for interpretation as Term.
            let first_term = unsafe { Term::from_raw(first_word) };

            if first_term.is_header() {
                // This is a boxed object - scan its fields
                let header = Header::from_raw(first_word);
                let obj_size = object_size_from_header(header);

                self.scan_boxed_object(process, mem, header)?;
                self.scan_ptr = Vaddr::new(self.scan_ptr.as_u64() + obj_size as u64);
            } else {
                // This is a pair - scan head and rest
                self.scan_pair(process, mem)?;
                self.scan_ptr = Vaddr::new(self.scan_ptr.as_u64() + Pair::SIZE as u64);
            }
        }
        Ok(())
    }

    /// Scan a pair at `scan_ptr` and copy any referenced objects.
    fn scan_pair<M: MemorySpace>(&mut self, process: &Process, mem: &mut M) -> Result<(), GcError> {
        let pair_addr = self.scan_ptr;

        // Read pair
        let pair: Pair = mem.read(pair_addr);

        // Update head if needed
        if needs_tracing(pair.head) && should_copy(process, pair.head) {
            let new_head = copy_term(self, process, mem, pair.head)?;
            // Write updated head
            mem.write(pair_addr, new_head);
        }

        // Update rest if needed
        let rest_addr = Vaddr::new(pair_addr.as_u64() + 8);
        if needs_tracing(pair.rest) && should_copy(process, pair.rest) {
            let new_rest = copy_term(self, process, mem, pair.rest)?;
            mem.write(rest_addr, new_rest);
        }

        Ok(())
    }

    /// Scan a boxed object at `scan_ptr` and copy any referenced objects.
    fn scan_boxed_object<M: MemorySpace>(
        &mut self,
        process: &Process,
        mem: &mut M,
        header: Header,
    ) -> Result<(), GcError> {
        let obj_addr = self.scan_ptr;

        match header.object_tag() {
            object::TUPLE => self.scan_tuple_fields(process, mem, obj_addr, header.arity()),
            object::VECTOR => self.scan_vector_fields(process, mem, obj_addr),
            object::MAP => self.scan_field_at(process, mem, obj_addr, 8),
            object::CLOSURE => self.scan_closure_fields(process, mem, obj_addr, header.arity()),
            object::FUN => self.scan_fun_fields(process, mem, obj_addr),
            object::NAMESPACE => self.scan_field_at(process, mem, obj_addr, 16),
            object::VAR => self.scan_var_fields(process, mem, obj_addr),
            // STRING, BINARY, FLOAT, BIGNUM, PID, REF, PROCBIN, SUBBIN,
            // SYMBOL, KEYWORD, and unknown types have no pointer fields
            _ => Ok(()),
        }
    }

    /// Scan a field at a given offset from an object address.
    fn scan_field_at<M: MemorySpace>(
        &mut self,
        process: &Process,
        mem: &mut M,
        obj_addr: Vaddr,
        offset: u64,
    ) -> Result<(), GcError> {
        let field_addr = Vaddr::new(obj_addr.as_u64() + offset);
        let term: Term = mem.read(field_addr);
        if needs_tracing(term) && should_copy(process, term) {
            let new_term = copy_term(self, process, mem, term)?;
            mem.write(field_addr, new_term);
        }
        Ok(())
    }

    /// Scan multiple consecutive Term fields starting at a base address.
    fn scan_term_array<M: MemorySpace>(
        &mut self,
        process: &Process,
        mem: &mut M,
        base_addr: Vaddr,
        count: usize,
    ) -> Result<(), GcError> {
        for i in 0..count {
            let field_addr = Vaddr::new(base_addr.as_u64() + (i as u64) * 8);
            let term: Term = mem.read(field_addr);
            if needs_tracing(term) && should_copy(process, term) {
                let new_term = copy_term(self, process, mem, term)?;
                mem.write(field_addr, new_term);
            }
        }
        Ok(())
    }

    /// Scan tuple element fields.
    fn scan_tuple_fields<M: MemorySpace>(
        &mut self,
        process: &Process,
        mem: &mut M,
        obj_addr: Vaddr,
        arity: u64,
    ) -> Result<(), GcError> {
        // Elements start after the 8-byte header
        let elements_base = Vaddr::new(obj_addr.as_u64() + 8);
        self.scan_term_array(process, mem, elements_base, arity as usize)
    }

    /// Scan vector element fields.
    fn scan_vector_fields<M: MemorySpace>(
        &mut self,
        process: &Process,
        mem: &mut M,
        obj_addr: Vaddr,
    ) -> Result<(), GcError> {
        // Vector: header (8) + length (8) + elements
        let length: u64 = mem.read(Vaddr::new(obj_addr.as_u64() + 8));
        let elements_base = Vaddr::new(obj_addr.as_u64() + 16);
        self.scan_term_array(process, mem, elements_base, length as usize)
    }

    /// Scan closure fields (function + captures).
    fn scan_closure_fields<M: MemorySpace>(
        &mut self,
        process: &Process,
        mem: &mut M,
        obj_addr: Vaddr,
        capture_count: u64,
    ) -> Result<(), GcError> {
        // Function pointer at offset 8
        self.scan_field_at(process, mem, obj_addr, 8)?;
        // Captures start at offset 16
        let captures_base = Vaddr::new(obj_addr.as_u64() + 16);
        self.scan_term_array(process, mem, captures_base, capture_count as usize)
    }

    /// Scan function constant fields.
    fn scan_fun_fields<M: MemorySpace>(
        &mut self,
        process: &Process,
        mem: &mut M,
        obj_addr: Vaddr,
    ) -> Result<(), GcError> {
        // Constants are Terms at end of the function object
        let code_len: u16 = mem.read(Vaddr::new(obj_addr.as_u64() + 12));
        let const_count: u16 = mem.read(Vaddr::new(obj_addr.as_u64() + 14));
        // Constants offset (aligned to 8 bytes)
        let constants_offset = (16 + code_len as usize + 7) & !7;
        let constants_base = Vaddr::new(obj_addr.as_u64() + constants_offset as u64);
        self.scan_term_array(process, mem, constants_base, const_count as usize)
    }

    /// Scan var fields (namespace + root).
    fn scan_var_fields<M: MemorySpace>(
        &mut self,
        process: &Process,
        mem: &mut M,
        obj_addr: Vaddr,
    ) -> Result<(), GcError> {
        // Name is symbol (immediate), namespace at 16, root at 24
        self.scan_field_at(process, mem, obj_addr, 16)?;
        self.scan_field_at(process, mem, obj_addr, 24)
    }
}

/// Check if a term points to an address that should be copied.
///
/// Returns true if the term points to the young heap or old heap of the process.
/// For minor GC, we only copy from young heap.
/// For major GC, we copy from both heaps.
const fn should_copy(process: &Process, term: Term) -> bool {
    let addr = term.to_vaddr();
    is_in_young_heap(process, addr) || is_in_old_heap(process, addr)
}

/// Copy a single term to to-space, returning the new term.
///
/// - If the term is an immediate, it's returned unchanged.
/// - If the term points to an object that's already been forwarded, return the
///   forwarded address.
/// - Otherwise, copy the object to to-space, leave a forwarding pointer, and
///   return a new term pointing to the new location.
///
/// Note: This function only copies the immediate object, not its children.
/// Use `Copier::scan_copied_objects` to transitively copy the entire graph.
///
/// # Errors
///
/// Returns `GcError::NeedsMajorGc` if to-space runs out of space during copying.
pub fn copy_term<M: MemorySpace>(
    copier: &mut Copier,
    process: &Process,
    mem: &mut M,
    term: Term,
) -> Result<Term, GcError> {
    // Immediates don't need copying
    if !needs_tracing(term) {
        return Ok(term);
    }

    // Check if it's in a heap we should copy from
    let addr = term.to_vaddr();
    if !is_in_young_heap(process, addr) && !is_in_old_heap(process, addr) {
        // Not in process heap (e.g., code region) - return unchanged
        return Ok(term);
    }

    if term.is_list() {
        copy_pair(copier, mem, term)
    } else if term.is_boxed() {
        copy_boxed(copier, mem, term)
    } else {
        // Shouldn't happen for terms that need_tracing
        Ok(term)
    }
}

/// Copy a pair (cons cell) to to-space.
fn copy_pair<M: MemorySpace>(
    copier: &mut Copier,
    mem: &mut M,
    term: Term,
) -> Result<Term, GcError> {
    let addr = term.to_vaddr();

    // Read the pair
    let pair: Pair = mem.read(addr);

    // Check if already forwarded
    if pair.is_forwarded() {
        let new_addr = pair.forward_address();
        return Ok(Term::list_vaddr(Vaddr::new(new_addr as u64)));
    }

    // Allocate space in to-space
    let new_addr = copier.allocate(Pair::SIZE).ok_or(GcError::NeedsMajorGc)?;

    // Copy the pair to new location (note: pointers within are NOT yet updated)
    mem.write(new_addr, pair);

    // Leave forwarding pointer at original location
    let mut forwarded_pair = pair;
    // SAFETY: new_addr is a valid Pair address in to-space
    unsafe {
        forwarded_pair.set_forward(new_addr.as_u64() as *const Pair);
    }
    mem.write(addr, forwarded_pair);

    Ok(Term::list_vaddr(new_addr))
}

/// Copy a boxed object to to-space.
fn copy_boxed<M: MemorySpace>(
    copier: &mut Copier,
    mem: &mut M,
    term: Term,
) -> Result<Term, GcError> {
    let addr = term.to_vaddr();

    // Read the header
    let header: Header = mem.read(addr);

    // Check if already forwarded
    if header.is_forward() {
        let new_addr = header.forward_address();
        return Ok(Term::boxed_vaddr(Vaddr::new(new_addr as u64)));
    }

    // Calculate object size
    let size = object_size_from_header(header);

    // Allocate space in to-space
    let new_addr = copier.allocate(size).ok_or(GcError::NeedsMajorGc)?;

    // Copy the object byte by byte
    // Note: We could optimize this with larger word copies
    for offset in 0..size {
        let src = Vaddr::new(addr.as_u64() + offset as u64);
        let dst = Vaddr::new(new_addr.as_u64() + offset as u64);
        let byte: u8 = mem.read(src);
        mem.write(dst, byte);
    }

    // Leave forwarding header at original location
    let forward_header = Header::forward(new_addr.as_u64() as *const u8);
    mem.write(addr, forward_header);

    Ok(Term::boxed_vaddr(new_addr))
}

/// Iterate over pointer fields in a heap object, calling the callback for each.
///
/// This is useful for implementing custom scanning logic.
/// The callback receives the address of the field and the current term value.
pub fn scan_object_fields<M: MemorySpace, F>(mem: &M, term: Term, mut callback: F)
where
    F: FnMut(Vaddr, Term),
{
    if !needs_tracing(term) {
        return;
    }

    let addr = term.to_vaddr();

    if term.is_list() {
        // Pair: head and rest
        let pair: Pair = mem.read(addr);
        callback(addr, pair.head);
        callback(Vaddr::new(addr.as_u64() + 8), pair.rest);
    } else if term.is_boxed() {
        let header: Header = mem.read(addr);

        match header.object_tag() {
            object::TUPLE => {
                let arity = header.arity() as usize;
                for i in 0..arity {
                    let elem_addr = Vaddr::new(addr.as_u64() + 8 + (i as u64) * 8);
                    let elem: Term = mem.read(elem_addr);
                    callback(elem_addr, elem);
                }
            }
            object::VECTOR => {
                let length: u64 = mem.read(Vaddr::new(addr.as_u64() + 8));
                for i in 0..(length as usize) {
                    let elem_addr = Vaddr::new(addr.as_u64() + 16 + (i as u64) * 8);
                    let elem: Term = mem.read(elem_addr);
                    callback(elem_addr, elem);
                }
            }
            object::MAP => {
                let entries_addr = Vaddr::new(addr.as_u64() + 8);
                let entries: Term = mem.read(entries_addr);
                callback(entries_addr, entries);
            }
            object::CLOSURE => {
                // Function pointer
                let func_addr = Vaddr::new(addr.as_u64() + 8);
                let func: Term = mem.read(func_addr);
                callback(func_addr, func);

                // Captures
                let capture_count = header.arity() as usize;
                for i in 0..capture_count {
                    let cap_addr = Vaddr::new(addr.as_u64() + 16 + (i as u64) * 8);
                    let cap: Term = mem.read(cap_addr);
                    callback(cap_addr, cap);
                }
            }
            object::FUN => {
                // Constants in the function
                let code_len: u16 = mem.read(Vaddr::new(addr.as_u64() + 12));
                let const_count: u16 = mem.read(Vaddr::new(addr.as_u64() + 14));
                let constants_offset = (16 + code_len as usize + 7) & !7;

                for i in 0..(const_count as usize) {
                    let const_addr =
                        Vaddr::new(addr.as_u64() + constants_offset as u64 + (i as u64) * 8);
                    let constant: Term = mem.read(const_addr);
                    callback(const_addr, constant);
                }
            }
            object::NAMESPACE => {
                // Mappings
                let mappings_addr = Vaddr::new(addr.as_u64() + 16);
                let mappings: Term = mem.read(mappings_addr);
                callback(mappings_addr, mappings);
            }
            object::VAR => {
                // Namespace and root
                let ns_addr = Vaddr::new(addr.as_u64() + 16);
                let ns: Term = mem.read(ns_addr);
                callback(ns_addr, ns);

                let root_addr = Vaddr::new(addr.as_u64() + 24);
                let root: Term = mem.read(root_addr);
                callback(root_addr, root);
            }
            // No pointer fields
            _ => {}
        }
    }
}
