// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Term allocation methods for Process.
//!
//! This module provides methods for allocating heap values using the new
//! BEAM-style Term representation. The heap layout uses 8-byte headers
//! and 8-byte Term pointers for more efficient memory usage.

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::{
    HeapClosure, HeapFloat, HeapFun, HeapMap, HeapPid, HeapRef, HeapString, HeapTuple, HeapVector,
};
use crate::term::pair::Pair;
use crate::term::tag::primary;

use super::Process;

impl Process {
    /// Allocate a string on the young heap using Term representation.
    ///
    /// Returns a boxed Term pointing to the allocated string, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_string<M: MemorySpace>(&mut self, mem: &mut M, s: &str) -> Option<Term> {
        let len = s.len();
        let total_size = HeapString::alloc_size(len);

        // Allocate space (8-byte aligned for header)
        let addr = self.alloc(total_size, 8)?;

        // Write header
        let header = HeapString::make_header(len);
        mem.write(addr, header);

        // Write string data after header
        let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
        let dest = mem.slice_mut(data_addr, len);
        dest.copy_from_slice(s.as_bytes());

        // SAFETY: We just allocated and initialized a valid string object
        Some(unsafe { term_from_boxed_addr(addr) })
    }

    // Note: alloc_term_symbol and alloc_term_keyword have been removed.
    // Symbols and keywords are now immediate values created via Realm::intern_symbol()
    // and Realm::intern_keyword(). They are not heap-allocated on the process heap.

    /// Allocate a pair (cons cell) on the young heap using Term representation.
    ///
    /// Pairs are headerless - they are identified by the LIST tag on the pointer.
    /// Returns a list Term pointing to the allocated pair, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_pair<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        head: Term,
        rest: Term,
    ) -> Option<Term> {
        // Allocate space (8-byte aligned)
        let addr = self.alloc(Pair::SIZE, 8)?;

        // Write the pair (no header - identified by LIST tag)
        let pair = Pair::new(head, rest);
        mem.write(addr, pair);

        // SAFETY: We just allocated and initialized a valid pair cell
        Some(unsafe { term_from_list_addr(addr) })
    }

    /// Allocate a tuple on the young heap using Term representation.
    ///
    /// Returns a boxed Term pointing to the allocated tuple, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_tuple<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        elements: &[Term],
    ) -> Option<Term> {
        let len = elements.len();
        let total_size = HeapTuple::alloc_size(len);

        // Allocate space (8-byte aligned)
        let addr = self.alloc(total_size, 8)?;

        // Write header
        let header = HeapTuple::make_header(len);
        mem.write(addr, header);

        // Write elements
        let data_addr = addr.add(HeapTuple::HEADER_SIZE as u64);
        for (i, &elem) in elements.iter().enumerate() {
            let elem_addr = data_addr.add((i * 8) as u64);
            mem.write(elem_addr, elem);
        }

        // SAFETY: We just allocated and initialized a valid tuple object
        Some(unsafe { term_from_boxed_addr(addr) })
    }

    /// Allocate a vector on the young heap using Term representation.
    ///
    /// Vectors have a capacity (stored in header) and a length field.
    /// Returns a boxed Term pointing to the allocated vector, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_vector<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        elements: &[Term],
    ) -> Option<Term> {
        let len = elements.len();
        let total_size = HeapVector::alloc_size(len);

        // Allocate space (8-byte aligned)
        let addr = self.alloc(total_size, 8)?;

        // Write header (capacity = length for now)
        let header = HeapVector::make_header(len);
        mem.write(addr, header);

        // Write length field
        let length_addr = addr.add(8);
        mem.write(length_addr, len as u64);

        // Write elements
        let data_addr = addr.add(HeapVector::PREFIX_SIZE as u64);
        for (i, &elem) in elements.iter().enumerate() {
            let elem_addr = data_addr.add((i * 8) as u64);
            mem.write(elem_addr, elem);
        }

        // SAFETY: We just allocated and initialized a valid vector object
        Some(unsafe { term_from_boxed_addr(addr) })
    }

    /// Allocate a map on the young heap using Term representation.
    ///
    /// Maps are association lists stored as pair chains.
    /// Returns a boxed Term pointing to the allocated map, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_map<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        entries: Term,
        entry_count: usize,
    ) -> Option<Term> {
        // Allocate space (8-byte aligned)
        let addr = self.alloc(HeapMap::SIZE, 8)?;

        // Write header with entry count
        let header = HeapMap::make_header(entry_count);
        mem.write(addr, header);

        // Write entries field
        let entries_addr = addr.add(8);
        mem.write(entries_addr, entries);

        // SAFETY: We just allocated and initialized a valid map object
        Some(unsafe { term_from_boxed_addr(addr) })
    }

    /// Allocate a float on the young heap using Term representation.
    ///
    /// Returns a boxed Term pointing to the allocated float, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_float<M: MemorySpace>(&mut self, mem: &mut M, value: f64) -> Option<Term> {
        // Allocate space (8-byte aligned)
        let addr = self.alloc(HeapFloat::SIZE, 8)?;

        // Write header
        let header = HeapFloat::make_header();
        mem.write(addr, header);

        // Write value
        let value_addr = addr.add(8);
        mem.write(value_addr, value);

        // SAFETY: We just allocated and initialized a valid float object
        Some(unsafe { term_from_boxed_addr(addr) })
    }

    /// Allocate a closure on the young heap using Term representation.
    ///
    /// Returns a boxed Term pointing to the allocated closure, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_closure<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        function: Term,
        captures: &[Term],
    ) -> Option<Term> {
        let capture_count = captures.len();
        let total_size = HeapClosure::alloc_size(capture_count);

        // Allocate space (8-byte aligned)
        let addr = self.alloc(total_size, 8)?;

        // Write header
        let header = HeapClosure::make_header(capture_count);
        mem.write(addr, header);

        // Write function pointer
        let fn_addr = addr.add(8);
        mem.write(fn_addr, function);

        // Write captures
        let data_addr = addr.add(HeapClosure::PREFIX_SIZE as u64);
        for (i, &capture) in captures.iter().enumerate() {
            let capture_addr = data_addr.add((i * 8) as u64);
            mem.write(capture_addr, capture);
        }

        // SAFETY: We just allocated and initialized a valid closure object
        Some(unsafe { term_from_boxed_addr(addr) })
    }

    /// Allocate a compiled function on the young heap using Term representation.
    ///
    /// Returns a boxed Term pointing to the allocated function, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_compiled_fn<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        arity: u8,
        variadic: bool,
        num_locals: u8,
        code: &[u32],
        constants: &[Term],
    ) -> Option<Term> {
        let code_len_bytes = code.len() * 4; // u32 instructions to bytes
        let const_count = constants.len();
        let total_size = HeapFun::alloc_size(code_len_bytes, const_count);

        // Allocate space (8-byte aligned)
        let addr = self.alloc(total_size, 8)?;

        // Write header
        let header = HeapFun::make_header(code_len_bytes, const_count);
        mem.write(addr, header);

        // Write metadata
        let meta_addr = addr.add(8);
        let variadic_byte: u8 = u8::from(variadic);
        mem.write(meta_addr, arity);
        mem.write(meta_addr.add(1), variadic_byte);
        mem.write(meta_addr.add(2), num_locals);
        mem.write(meta_addr.add(3), 0u8); // padding
        mem.write(meta_addr.add(4), code_len_bytes as u16);
        mem.write(meta_addr.add(6), const_count as u16);

        // Write bytecode
        let code_addr = addr.add(HeapFun::PREFIX_SIZE as u64);
        for (i, &instr) in code.iter().enumerate() {
            let instr_addr = code_addr.add((i * 4) as u64);
            mem.write(instr_addr, instr);
        }

        // Write constants at aligned offset
        let constants_offset = HeapFun::constants_offset(code_len_bytes);
        let constants_addr = addr.add(constants_offset as u64);
        for (i, &constant) in constants.iter().enumerate() {
            let const_addr = constants_addr.add((i * 8) as u64);
            mem.write(const_addr, constant);
        }

        // SAFETY: We just allocated and initialized a valid function object
        Some(unsafe { term_from_boxed_addr(addr) })
    }

    /// Allocate a PID (process identifier) on the young heap.
    ///
    /// Packs index and generation into a 64-bit data field:
    /// `data = (generation << 32) | index`.
    ///
    /// Returns a boxed Term pointing to the allocated PID, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_pid<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        index: u32,
        generation: u32,
    ) -> Option<Term> {
        let addr = self.alloc(HeapPid::SIZE, 8)?;

        let header = HeapPid::make_header();
        mem.write(addr, header);

        let data = u64::from(generation) << 32 | u64::from(index);
        let data_addr = addr.add(8);
        mem.write(data_addr, data);

        // SAFETY: We just allocated and initialized a valid PID object
        Some(unsafe { term_from_boxed_addr(addr) })
    }

    /// Allocate a REF (unique reference) on the young heap.
    ///
    /// Returns a boxed Term pointing to the allocated ref, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_ref<M: MemorySpace>(&mut self, mem: &mut M, id: u64) -> Option<Term> {
        let addr = self.alloc(HeapRef::SIZE, 8)?;

        let header = HeapRef::make_header();
        mem.write(addr, header);

        let id_addr = addr.add(8);
        mem.write(id_addr, id);

        // SAFETY: We just allocated and initialized a valid REF object
        Some(unsafe { term_from_boxed_addr(addr) })
    }

    // ========================================================================
    // Term Reading Methods
    // ========================================================================

    /// Read a heap-allocated string as a str slice.
    ///
    /// Returns `None` if the term is not a string or contains invalid UTF-8.
    ///
    /// Note: Symbols and keywords are now immediate values. Use `Realm::symbol_name()`
    /// and `Realm::keyword_name()` to get their string content.
    #[must_use]
    pub fn read_term_string<'a, M: MemorySpace>(&self, mem: &'a M, term: Term) -> Option<&'a str> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        let tag = header.object_tag();

        // Only accept STRING - symbols/keywords are now immediate values
        if tag != crate::term::tag::object::STRING {
            return None;
        }

        let len = header.arity() as usize;
        let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
        let bytes = mem.slice(data_addr, len);

        core::str::from_utf8(bytes).ok()
    }

    /// Read a pair from the heap.
    ///
    /// Returns the (head, rest) pair, or `None` if the term is not a list.
    #[must_use]
    pub fn read_term_pair<M: MemorySpace>(&self, mem: &M, term: Term) -> Option<(Term, Term)> {
        if !term.is_list() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let pair: Pair = mem.read(addr);
        Some((pair.head, pair.rest))
    }

    /// Read a tuple's length from the heap.
    ///
    /// Returns `None` if the term is not a tuple.
    #[must_use]
    pub fn read_term_tuple_len<M: MemorySpace>(&self, mem: &M, term: Term) -> Option<usize> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);

        if header.object_tag() != crate::term::tag::object::TUPLE {
            return None;
        }

        Some(header.arity() as usize)
    }

    /// Read a tuple element at the given index.
    ///
    /// Returns `None` if the term is not a tuple or index is out of bounds.
    #[must_use]
    pub fn read_term_tuple_element<M: MemorySpace>(
        &self,
        mem: &M,
        term: Term,
        index: usize,
    ) -> Option<Term> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);

        if header.object_tag() != crate::term::tag::object::TUPLE {
            return None;
        }

        let len = header.arity() as usize;
        if index >= len {
            return None;
        }

        let data_addr = addr.add(HeapTuple::HEADER_SIZE as u64);
        let elem_addr = data_addr.add((index * 8) as u64);
        Some(mem.read(elem_addr))
    }

    /// Read a vector's length from the heap.
    ///
    /// Returns `None` if the term is not a vector.
    #[must_use]
    pub fn read_term_vector_len<M: MemorySpace>(&self, mem: &M, term: Term) -> Option<usize> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);

        if header.object_tag() != crate::term::tag::object::VECTOR {
            return None;
        }

        // Length is stored in a separate field, not the header arity
        let length_addr = addr.add(8);
        let length: u64 = mem.read(length_addr);
        Some(length as usize)
    }

    /// Read a vector element at the given index.
    ///
    /// Returns `None` if the term is not a vector or index is out of bounds.
    #[must_use]
    pub fn read_term_vector_element<M: MemorySpace>(
        &self,
        mem: &M,
        term: Term,
        index: usize,
    ) -> Option<Term> {
        let len = self.read_term_vector_len(mem, term)?;
        if index >= len {
            return None;
        }

        let addr = term_to_vaddr(term);
        let data_addr = addr.add(HeapVector::PREFIX_SIZE as u64);
        let elem_addr = data_addr.add((index * 8) as u64);
        Some(mem.read(elem_addr))
    }

    /// Read a float value from the heap.
    ///
    /// Returns `None` if the term is not a float.
    #[must_use]
    pub fn read_term_float<M: MemorySpace>(&self, mem: &M, term: Term) -> Option<f64> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);

        if header.object_tag() != crate::term::tag::object::FLOAT {
            return None;
        }

        let value_addr = addr.add(8);
        Some(mem.read(value_addr))
    }

    /// Read a map's entries from the heap.
    ///
    /// Returns the pair chain (or nil for empty map), or `None` if not a map.
    #[must_use]
    pub fn read_term_map_entries<M: MemorySpace>(&self, mem: &M, term: Term) -> Option<Term> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);

        if header.object_tag() != crate::term::tag::object::MAP {
            return None;
        }

        let entries_addr = addr.add(8);
        Some(mem.read(entries_addr))
    }

    /// Read a PID from the heap.
    ///
    /// Returns `(index, generation)`, or `None` if the term is not a PID.
    #[must_use]
    pub fn read_term_pid<M: MemorySpace>(&self, mem: &M, term: Term) -> Option<(u32, u32)> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);

        if header.object_tag() != crate::term::tag::object::PID {
            return None;
        }

        let data_addr = addr.add(8);
        let data: u64 = mem.read(data_addr);
        let index = data as u32;
        let generation = (data >> 32) as u32;
        Some((index, generation))
    }

    /// Read a REF from the heap.
    ///
    /// Returns the unique reference ID, or `None` if the term is not a REF.
    #[must_use]
    pub fn read_term_ref<M: MemorySpace>(&self, mem: &M, term: Term) -> Option<u64> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);

        if header.object_tag() != crate::term::tag::object::REF {
            return None;
        }

        let id_addr = addr.add(8);
        Some(mem.read(id_addr))
    }

    // ========================================================================
    // Type checking helpers
    // ========================================================================

    /// Check if a term is a keyword.
    ///
    /// Keywords are always immediate values (interned) with their index encoded.
    #[inline]
    #[must_use]
    pub const fn is_term_keyword(&self, term: Term) -> bool {
        term.is_keyword()
    }

    /// Check if a term is a symbol.
    ///
    /// Symbols are always immediate values (interned) with their index encoded.
    #[inline]
    #[must_use]
    pub const fn is_term_symbol(&self, term: Term) -> bool {
        term.is_symbol()
    }

    /// Check if a term is a tuple.
    ///
    /// This method is safe to use with `MockVSpace` as it uses the `MemorySpace`
    /// trait instead of dereferencing raw pointers.
    #[must_use]
    pub fn is_term_tuple<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::TUPLE
    }

    /// Check if a term is a vector.
    ///
    /// This method is safe to use with `MockVSpace` as it uses the `MemorySpace`
    /// trait instead of dereferencing raw pointers.
    #[must_use]
    pub fn is_term_vector<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::VECTOR
    }

    /// Check if a term is a map.
    ///
    /// This method is safe to use with `MockVSpace` as it uses the `MemorySpace`
    /// trait instead of dereferencing raw pointers.
    #[must_use]
    pub fn is_term_map<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::MAP
    }

    /// Check if a term is a var.
    ///
    /// This method is safe to use with `MockVSpace` as it uses the `MemorySpace`
    /// trait instead of dereferencing raw pointers.
    #[must_use]
    pub fn is_term_var<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::VAR
    }

    /// Check if a term is a namespace.
    ///
    /// This method is safe to use with `MockVSpace` as it uses the `MemorySpace`
    /// trait instead of dereferencing raw pointers.
    #[must_use]
    pub fn is_term_namespace<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::NAMESPACE
    }

    /// Check if a term is a compiled function.
    ///
    /// This method is safe to use with `MockVSpace` as it uses the `MemorySpace`
    /// trait instead of dereferencing raw pointers.
    #[must_use]
    pub fn is_term_fun<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::FUN
    }

    /// Check if a term is a closure.
    ///
    /// This method is safe to use with `MockVSpace` as it uses the `MemorySpace`
    /// trait instead of dereferencing raw pointers.
    #[must_use]
    pub fn is_term_closure<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::CLOSURE
    }

    /// Check if a term is callable (function, closure, or native function).
    ///
    /// This method is safe to use with `MockVSpace` as it uses the `MemorySpace`
    /// trait instead of dereferencing raw pointers.
    #[must_use]
    pub fn is_term_callable<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        // Native functions are immediate values
        if term.is_native_fn() {
            return true;
        }

        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        let tag = header.object_tag();
        tag == crate::term::tag::object::FUN || tag == crate::term::tag::object::CLOSURE
    }

    /// Check if a term is a PID (process identifier).
    #[must_use]
    pub fn is_term_pid<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::PID
    }

    /// Check if a term is a REF (unique reference).
    #[must_use]
    pub fn is_term_ref<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::REF
    }

    /// Check if a term is a string.
    ///
    /// This method is safe to use with `MockVSpace` as it uses the `MemorySpace`
    /// trait instead of dereferencing raw pointers.
    #[must_use]
    pub fn is_term_string<M: MemorySpace>(&self, mem: &M, term: Term) -> bool {
        if !term.is_boxed() {
            return false;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);
        header.object_tag() == crate::term::tag::object::STRING
    }

    /// Allocate a namespace on the young heap.
    ///
    /// Returns a `Term` pointing to the allocated namespace, or `None` if OOM.
    #[must_use]
    pub fn alloc_term_namespace<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        name: Term,
    ) -> Option<Term> {
        use crate::term::heap::{HeapMap, HeapNamespace};

        // Create empty mappings map
        let empty_map_addr = self.alloc(HeapMap::SIZE, 8)?;
        let empty_map_header = HeapMap::make_header(0);
        mem.write(empty_map_addr, empty_map_header);
        let entries_addr = empty_map_addr.add(8);
        mem.write(entries_addr, Term::NIL);
        // SAFETY: We just allocated and initialized a valid map object
        let mappings = unsafe { term_from_boxed_addr(empty_map_addr) };

        // Allocate namespace
        let ns_addr = self.alloc(HeapNamespace::SIZE, 8)?;
        let ns = HeapNamespace {
            header: HeapNamespace::make_header(),
            name,
            mappings,
        };
        mem.write(ns_addr, ns);

        // SAFETY: We just allocated and initialized a valid namespace object
        Some(unsafe { term_from_boxed_addr(ns_addr) })
    }

    /// Read a namespace from the heap.
    ///
    /// Returns the namespace struct, or `None` if the term is not a namespace.
    #[must_use]
    pub fn read_term_namespace<M: MemorySpace>(
        &self,
        mem: &M,
        term: Term,
    ) -> Option<crate::term::heap::HeapNamespace> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term_to_vaddr(term);
        let header: Header = mem.read(addr);

        if header.object_tag() != crate::term::tag::object::NAMESPACE {
            return None;
        }

        Some(mem.read(addr))
    }

    /// Intern a var in a namespace.
    ///
    /// Creates a new var if one doesn't exist, or returns the existing var.
    /// Returns `None` on allocation failure.
    pub fn intern_var_term<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        ns: Term,
        name: Term,
        value: Term,
    ) -> Option<Term> {
        use crate::term::heap::{HeapNamespace, HeapPair, HeapVar};

        // Read namespace
        let ns_addr = term_to_vaddr(ns);
        let mut namespace: HeapNamespace = mem.read(ns_addr);

        // Search for existing var with this name
        let mut current = namespace.mappings;
        while current.is_list() {
            let pair: HeapPair = mem.read(term_to_vaddr(current));
            let entry = pair.head;

            // Each entry is a [symbol var] tuple
            if let Some(entry_name) = self.read_term_tuple_element(mem, entry, 0) {
                // Symbols are immediate values - compare directly by Term equality
                if name == entry_name {
                    // Found existing var - update its value
                    if let Some(var_term) = self.read_term_tuple_element(mem, entry, 1) {
                        let var_addr = term_to_vaddr(var_term);
                        let mut var: HeapVar = mem.read(var_addr);
                        var.root = value;
                        mem.write(var_addr, var);
                        return Some(var_term);
                    }
                }
            }
            current = pair.tail;
        }

        // Create new var
        let var_size = HeapVar::SIZE;
        let var_addr = self.alloc(var_size, 8)?;
        let var_header = HeapVar::make_header();
        let var = HeapVar {
            header: var_header,
            name,
            namespace: ns,
            root: value,
        };
        mem.write(var_addr, var);
        // SAFETY: We just allocated and initialized a valid var object
        let var_term = unsafe { term_from_boxed_addr(var_addr) };

        // Create [name var] tuple
        let entry_elems = [name, var_term];
        let entry = self.alloc_term_tuple(mem, &entry_elems)?;

        // Prepend to namespace mappings
        let new_pair = self.alloc_term_pair(mem, entry, namespace.mappings)?;
        namespace.mappings = new_pair;
        mem.write(ns_addr, namespace);

        Some(var_term)
    }

    /// Get the value of a var.
    ///
    /// Returns the root value, or checks process bindings for dynamic vars.
    pub fn var_get_term<M: MemorySpace>(&self, mem: &M, var: Term) -> Option<Term> {
        use crate::term::heap::HeapVar;

        let var_addr = term_to_vaddr(var);
        let slot: HeapVar = mem.read(var_addr);

        // Check process binding first
        if let Some(binding) = self.get_binding_term(var_addr) {
            return Some(binding);
        }

        // Return root value
        Some(slot.root)
    }

    /// Get a process-local binding for a var.
    fn get_binding_term(&self, var_addr: Vaddr) -> Option<Term> {
        self.bindings.get(&var_addr).copied()
    }

    /// Set a process-local binding for a var.
    pub fn set_binding_term(&mut self, var_addr: Vaddr, value: Term) -> Option<()> {
        self.bindings.insert(var_addr, value);
        Some(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a Vaddr to a boxed Term.
///
/// # Safety
///
/// The address must point to a valid heap object with a header.
#[inline]
const unsafe fn term_from_boxed_addr(addr: Vaddr) -> Term {
    // SAFETY: Caller guarantees addr points to valid heap object
    unsafe { Term::from_raw(addr.as_u64() | primary::BOXED) }
}

/// Convert a Vaddr to a list Term.
///
/// # Safety
///
/// The address must point to a valid pair cell.
#[inline]
const unsafe fn term_from_list_addr(addr: Vaddr) -> Term {
    // SAFETY: Caller guarantees addr points to valid pair cell
    unsafe { Term::from_raw(addr.as_u64() | primary::LIST) }
}

/// Extract a Vaddr from a Term (works for both boxed and list terms).
#[inline]
const fn term_to_vaddr(term: Term) -> Vaddr {
    Vaddr::new(term.as_raw() & !primary::MASK)
}
