// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! GC-aware memory allocation.
//!
//! This module provides allocation functions that automatically trigger
//! garbage collection when the heap is full, following this sequence:
//!
//! 1. Try normal allocation
//! 2. Run minor GC (promote live objects to old heap)
//! 3. Retry allocation
//! 4. Grow young heap (allocate larger region, copy live data)
//! 5. Retry allocation
//! 6. Run major GC (compact both heaps)
//! 7. Retry allocation
//! 8. Return OOM only if all attempts fail
//!
//! This is the blessed allocation path - all code (REPL, e2e tests, spec tests)
//! uses this same implementation.

use crate::Vaddr;
use crate::gc::growth::grow_young_heap_with_gc;
use crate::gc::{major_gc, minor_gc};
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::scheduler::Worker;

/// Allocate memory with automatic GC on failure.
///
/// This is the primary allocation function that should be used for all
/// heap allocations. It handles the full GC cycle when allocation fails.
///
/// # Arguments
///
/// * `process` - The process to allocate in
/// * `worker` - The worker (contains X registers as GC roots)
/// * `realm` - The realm (contains memory pool for heap growth)
/// * `mem` - The memory space
/// * `size` - Number of bytes to allocate
/// * `align` - Alignment requirement
///
/// # Returns
///
/// The allocated address, or `None` if all recovery attempts fail.
pub fn alloc_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
    size: usize,
    align: usize,
) -> Option<Vaddr> {
    // 1. Try normal allocation
    if let Some(addr) = process.alloc(size, align) {
        return Some(addr);
    }

    // 2. Run minor GC
    let _ = minor_gc(process, worker, mem);
    if let Some(addr) = process.alloc(size, align) {
        return Some(addr);
    }

    // 3. Grow young heap (includes GC to compact live data)
    // Request enough space for the allocation plus some headroom
    let required = size.saturating_mul(2).max(1024);
    let _ = grow_young_heap_with_gc(process, worker, realm.pool_mut(), mem, required);
    if let Some(addr) = process.alloc(size, align) {
        return Some(addr);
    }

    // 4. Run major GC (full collection)
    let _ = major_gc(process, worker, realm.pool_mut(), mem);
    if let Some(addr) = process.alloc(size, align) {
        return Some(addr);
    }

    // 5. Last resort: try growing heap one more time after major GC
    let _ = grow_young_heap_with_gc(process, worker, realm.pool_mut(), mem, required);
    process.alloc(size, align)
}

/// Result of a GC-aware allocation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocResult {
    /// Allocation succeeded without GC.
    Success(Vaddr),
    /// Allocation succeeded after GC.
    SuccessAfterGc(Vaddr),
    /// Allocation failed - out of memory.
    OutOfMemory,
}

impl AllocResult {
    /// Convert to Option, discarding GC information.
    #[must_use]
    pub const fn ok(self) -> Option<Vaddr> {
        match self {
            Self::Success(addr) | Self::SuccessAfterGc(addr) => Some(addr),
            Self::OutOfMemory => None,
        }
    }

    /// Returns true if allocation succeeded (with or without GC).
    #[must_use]
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Success(_) | Self::SuccessAfterGc(_))
    }
}

/// Specification for a compiled function.
///
/// This bundles the metadata needed to allocate a compiled function,
/// reducing the number of parameters to `alloc_compiled_fn_with_gc`.
pub struct CompiledFnSpec<'a> {
    /// Number of parameters the function takes.
    pub arity: u8,
    /// Whether the function accepts variadic arguments.
    pub variadic: bool,
    /// Number of local variables (Y registers) used.
    pub num_locals: u8,
    /// The compiled bytecode instructions.
    pub code: &'a [u32],
    /// Constants referenced by the bytecode.
    pub constants: &'a [Term],
}

// ============================================================================
// GC-aware Term Allocation
// ============================================================================

use crate::term::Term;
use crate::term::heap::{
    HeapClosure, HeapFloat, HeapFun, HeapMap, HeapString, HeapTuple, HeapVector,
};
use crate::term::pair::Pair;
use crate::term::tag::primary;

/// Allocate a tuple with GC support.
pub fn alloc_tuple_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
    elements: &[Term],
) -> Option<Term> {
    let len = elements.len();
    let total_size = HeapTuple::alloc_size(len);

    let addr = alloc_with_gc(process, worker, realm, mem, total_size, 8)?;

    // Write header
    let header = HeapTuple::make_header(len);
    mem.write(addr, header);

    // Write elements
    let data_addr = addr.add(HeapTuple::HEADER_SIZE as u64);
    for (i, &elem) in elements.iter().enumerate() {
        let elem_addr = data_addr.add((i * 8) as u64);
        mem.write(elem_addr, elem);
    }

    Some(term_from_boxed_addr(addr))
}

/// Allocate a vector with GC support.
pub fn alloc_vector_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
    elements: &[Term],
) -> Option<Term> {
    let len = elements.len();
    let total_size = HeapVector::alloc_size(len);

    let addr = alloc_with_gc(process, worker, realm, mem, total_size, 8)?;

    // Write header (capacity = length)
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

    Some(term_from_boxed_addr(addr))
}

/// Allocate a pair (cons cell) with GC support.
pub fn alloc_pair_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
    head: Term,
    rest: Term,
) -> Option<Term> {
    let addr = alloc_with_gc(process, worker, realm, mem, Pair::SIZE, 8)?;

    let pair = Pair::new(head, rest);
    mem.write(addr, pair);

    Some(term_from_list_addr(addr))
}

/// Allocate a map with GC support.
pub fn alloc_map_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
    entries: Term,
    entry_count: usize,
) -> Option<Term> {
    let addr = alloc_with_gc(process, worker, realm, mem, HeapMap::SIZE, 8)?;

    let header = HeapMap::make_header(entry_count);
    mem.write(addr, header);

    let entries_addr = addr.add(8);
    mem.write(entries_addr, entries);

    Some(term_from_boxed_addr(addr))
}

/// Allocate a string with GC support.
pub fn alloc_string_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
    s: &str,
) -> Option<Term> {
    let len = s.len();
    let total_size = HeapString::alloc_size(len);

    let addr = alloc_with_gc(process, worker, realm, mem, total_size, 8)?;

    let header = HeapString::make_header(len);
    mem.write(addr, header);

    let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
    let dest = mem.slice_mut(data_addr, len);
    dest.copy_from_slice(s.as_bytes());

    Some(term_from_boxed_addr(addr))
}

/// Allocate a float with GC support.
pub fn alloc_float_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
    value: f64,
) -> Option<Term> {
    let addr = alloc_with_gc(process, worker, realm, mem, HeapFloat::SIZE, 8)?;

    let header = HeapFloat::make_header();
    mem.write(addr, header);

    let value_addr = addr.add(8);
    mem.write(value_addr, value);

    Some(term_from_boxed_addr(addr))
}

/// Allocate a closure with GC support.
pub fn alloc_closure_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
    function: Term,
    captures: &[Term],
) -> Option<Term> {
    let capture_count = captures.len();
    let total_size = HeapClosure::alloc_size(capture_count);

    let addr = alloc_with_gc(process, worker, realm, mem, total_size, 8)?;

    let header = HeapClosure::make_header(capture_count);
    mem.write(addr, header);

    let fn_addr = addr.add(8);
    mem.write(fn_addr, function);

    let data_addr = addr.add(HeapClosure::PREFIX_SIZE as u64);
    for (i, &capture) in captures.iter().enumerate() {
        let capture_addr = data_addr.add((i * 8) as u64);
        mem.write(capture_addr, capture);
    }

    Some(term_from_boxed_addr(addr))
}

/// Allocate a compiled function with GC support.
pub fn alloc_compiled_fn_with_gc<M: MemorySpace>(
    process: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
    spec: &CompiledFnSpec<'_>,
) -> Option<Term> {
    let code_len_bytes = spec.code.len() * 4;
    let const_count = spec.constants.len();
    let total_size = HeapFun::alloc_size(code_len_bytes, const_count);

    let addr = alloc_with_gc(process, worker, realm, mem, total_size, 8)?;

    let header = HeapFun::make_header(code_len_bytes, const_count);
    mem.write(addr, header);

    let meta_addr = addr.add(8);
    let variadic_byte: u8 = u8::from(spec.variadic);
    mem.write(meta_addr, spec.arity);
    mem.write(meta_addr.add(1), variadic_byte);
    mem.write(meta_addr.add(2), spec.num_locals);
    mem.write(meta_addr.add(3), 0u8);
    mem.write(meta_addr.add(4), code_len_bytes as u16);
    mem.write(meta_addr.add(6), const_count as u16);

    let code_addr = addr.add(HeapFun::PREFIX_SIZE as u64);
    for (i, &instr) in spec.code.iter().enumerate() {
        let instr_addr = code_addr.add((i * 4) as u64);
        mem.write(instr_addr, instr);
    }

    let constants_offset = HeapFun::constants_offset(code_len_bytes);
    let constants_addr = addr.add(constants_offset as u64);
    for (i, &constant) in spec.constants.iter().enumerate() {
        let const_addr = constants_addr.add((i * 8) as u64);
        mem.write(const_addr, constant);
    }

    Some(term_from_boxed_addr(addr))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a Vaddr to a boxed Term.
#[inline]
const fn term_from_boxed_addr(addr: Vaddr) -> Term {
    // SAFETY: Caller guarantees addr points to valid heap object
    unsafe { Term::from_raw(addr.as_u64() | primary::BOXED) }
}

/// Convert a Vaddr to a list Term.
#[inline]
const fn term_from_list_addr(addr: Vaddr) -> Term {
    // SAFETY: Caller guarantees addr points to valid pair cell
    unsafe { Term::from_raw(addr.as_u64() | primary::LIST) }
}
