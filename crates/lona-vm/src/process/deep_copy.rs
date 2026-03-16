// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Deep copy for message passing.
//!
//! Provides `deep_copy_message_to_process` and `deep_copy_message_to_fragment`
//! for copying terms between processes (send) or into heap fragments
//! (cross-worker send when receiver is taken).
//!
//! These functions copy all heap-allocated data referenced by a Term,
//! producing an independent copy that can be safely owned by the receiver.
//! Immediates (integers, symbols, keywords, nil, booleans) pass through
//! without copying.

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::realm::copy::VisitedTracker;
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::{
    HeapClosure, HeapFloat, HeapFun, HeapMap, HeapPair, HeapString, HeapTuple, HeapVector,
};
use crate::term::tag::object;

use super::Process;
use super::heap_fragment::HeapFragment;

/// Deep copy a Term into a target process's heap.
///
/// Used for the fast path of `send`: the receiver is in the `ProcessTable`
/// (not taken), so we can allocate directly on its heap.
pub fn deep_copy_message_to_process<M: MemorySpace>(
    term: Term,
    dst_proc: &mut Process,
    mem: &mut M,
) -> Option<Term> {
    let mut visited = VisitedTracker::new();
    copy_term(
        term,
        &mut |size, align| dst_proc.alloc(size, align),
        mem,
        &mut visited,
    )
}

/// Deep copy a Term into a heap fragment.
///
/// Used for the fallback path of `send`: the receiver is taken (running
/// on another worker), so we allocate into a fragment that will be
/// pushed to the receiver's slot inbox.
pub fn deep_copy_message_to_fragment<M: MemorySpace>(
    term: Term,
    fragment: &mut HeapFragment,
    mem: &mut M,
) -> Option<Term> {
    let mut visited = VisitedTracker::new();
    copy_term(
        term,
        &mut |size, align| fragment.alloc(size, align),
        mem,
        &mut visited,
    )
}

/// Recursive deep copy using a generic allocator function.
fn copy_term<M, A>(
    term: Term,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    // Immediates: no copy needed
    if term.is_immediate() || term.is_nil() {
        return Some(term);
    }

    // List (cons cell)
    if term.is_list() {
        let addr = term.to_vaddr();
        if let Some(dst) = visited.check(addr) {
            return Some(Term::list_vaddr(dst));
        }
        let pair: HeapPair = mem.read(addr);
        let dst_addr = alloc(HeapPair::SIZE, 8)?;
        visited.record(addr, dst_addr);
        let head = copy_term(pair.head, alloc, mem, visited)?;
        let tail = copy_term(pair.tail, alloc, mem, visited)?;
        mem.write(dst_addr, HeapPair { head, tail });
        return Some(Term::list_vaddr(dst_addr));
    }

    // Boxed values
    if term.is_boxed() {
        let addr = term.to_vaddr();
        if let Some(dst) = visited.check(addr) {
            return Some(Term::boxed_vaddr(dst));
        }
        return copy_boxed(term, addr, alloc, mem, visited);
    }

    None
}

/// Deep copy a boxed term.
fn copy_boxed<M, A>(
    original: Term,
    addr: Vaddr,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    let header: Header = mem.read(addr);
    let tag = header.object_tag();

    match tag {
        object::STRING => copy_string(addr, header, alloc, mem, visited),
        object::TUPLE => copy_tuple(addr, header, alloc, mem, visited),
        object::VECTOR => copy_vector(addr, alloc, mem, visited),
        object::MAP => copy_map(addr, header, alloc, mem, visited),
        object::FUN => copy_fun(addr, alloc, mem, visited),
        object::CLOSURE => copy_closure(addr, alloc, mem, visited),
        object::PID => copy_pid(addr, header, alloc, mem, visited),
        object::REF => copy_ref(addr, header, alloc, mem, visited),
        object::FLOAT => copy_float(addr, header, alloc, mem, visited),
        // Vars and Namespaces live in realm, pass through
        object::VAR | object::NAMESPACE => Some(original),
        _ => None,
    }
}

fn copy_string<M, A>(
    addr: Vaddr,
    header: Header,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    let len = header.arity() as usize;
    let total = HeapString::alloc_size(len);
    let dst = alloc(total, 8)?;
    visited.record(addr, dst);
    mem.write(dst, header);
    let src_data = addr.add(HeapString::HEADER_SIZE as u64);
    let dst_data = dst.add(HeapString::HEADER_SIZE as u64);
    for i in 0..len {
        let byte: u8 = mem.read(src_data.add(i as u64));
        mem.write(dst_data.add(i as u64), byte);
    }
    Some(Term::boxed_vaddr(dst))
}

fn copy_tuple<M, A>(
    addr: Vaddr,
    header: Header,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    let len = header.arity() as usize;
    let total = HeapTuple::alloc_size(len);
    let dst = alloc(total, 8)?;
    visited.record(addr, dst);
    mem.write(dst, header);
    let src_data = addr.add(HeapTuple::HEADER_SIZE as u64);
    let dst_data = dst.add(HeapTuple::HEADER_SIZE as u64);
    for i in 0..len {
        let offset = (i * 8) as u64;
        let elem: Term = mem.read(src_data.add(offset));
        let copied = copy_term(elem, alloc, mem, visited)?;
        mem.write(dst_data.add(offset), copied);
    }
    Some(Term::boxed_vaddr(dst))
}

fn copy_vector<M, A>(
    addr: Vaddr,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    let len_val: u64 = mem.read(addr.add(8));
    let len = len_val as usize;
    let total = HeapVector::alloc_size(len);
    let dst = alloc(total, 8)?;
    visited.record(addr, dst);
    // Reconstruct header with len as capacity (matches allocated size)
    mem.write(dst, HeapVector::make_header(len));
    mem.write(dst.add(8), len_val);
    let src_data = addr.add(HeapVector::PREFIX_SIZE as u64);
    let dst_data = dst.add(HeapVector::PREFIX_SIZE as u64);
    for i in 0..len {
        let offset = (i * 8) as u64;
        let elem: Term = mem.read(src_data.add(offset));
        let copied = copy_term(elem, alloc, mem, visited)?;
        mem.write(dst_data.add(offset), copied);
    }
    Some(Term::boxed_vaddr(dst))
}

fn copy_map<M, A>(
    addr: Vaddr,
    header: Header,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    let entries: Term = mem.read(addr.add(8));
    let dst = alloc(HeapMap::SIZE, 8)?;
    visited.record(addr, dst);
    let copied_entries = copy_term(entries, alloc, mem, visited)?;
    mem.write(dst, header);
    mem.write(dst.add(8), copied_entries);
    Some(Term::boxed_vaddr(dst))
}

fn copy_fun<M, A>(
    addr: Vaddr,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    let fun: HeapFun = mem.read(addr);
    let code_len = fun.code_len as usize;
    let const_count = fun.const_count as usize;
    let total = HeapFun::alloc_size(code_len, const_count);
    let dst = alloc(total, 8)?;
    visited.record(addr, dst);
    mem.write(dst, fun);

    // Copy bytecode
    let src_code = addr.add(HeapFun::PREFIX_SIZE as u64);
    let dst_code = dst.add(HeapFun::PREFIX_SIZE as u64);
    for i in 0..code_len {
        let byte: u8 = mem.read(src_code.add(i as u64));
        mem.write(dst_code.add(i as u64), byte);
    }

    // Deep copy constants
    let const_offset = HeapFun::constants_offset(code_len);
    let src_const = addr.add(const_offset as u64);
    let dst_const = dst.add(const_offset as u64);
    for i in 0..const_count {
        let offset = (i * core::mem::size_of::<Term>()) as u64;
        let c: Term = mem.read(src_const.add(offset));
        let copied = copy_term(c, alloc, mem, visited)?;
        mem.write(dst_const.add(offset), copied);
    }

    Some(Term::boxed_vaddr(dst))
}

fn copy_closure<M, A>(
    addr: Vaddr,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    let closure: HeapClosure = mem.read(addr);
    let captures_len = closure.capture_count();
    let total = HeapClosure::alloc_size(captures_len);
    let dst = alloc(total, 8)?;
    visited.record(addr, dst);

    let dst_func = copy_term(closure.function, alloc, mem, visited)?;
    let dst_hdr = HeapClosure {
        header: HeapClosure::make_header(captures_len),
        function: dst_func,
    };
    mem.write(dst, dst_hdr);

    let src_caps = addr.add(HeapClosure::PREFIX_SIZE as u64);
    let dst_caps = dst.add(HeapClosure::PREFIX_SIZE as u64);
    for i in 0..captures_len {
        let offset = (i * core::mem::size_of::<Term>()) as u64;
        let cap: Term = mem.read(src_caps.add(offset));
        let copied = copy_term(cap, alloc, mem, visited)?;
        mem.write(dst_caps.add(offset), copied);
    }

    Some(Term::boxed_vaddr(dst))
}

fn copy_pid<M, A>(
    addr: Vaddr,
    header: Header,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    use crate::term::heap::HeapPid;
    let total = HeapPid::SIZE;
    let dst = alloc(total, 8)?;
    visited.record(addr, dst);
    mem.write(dst, header);
    // Copy index and generation (8 bytes after header)
    let pid_data: u64 = mem.read(addr.add(8));
    mem.write(dst.add(8), pid_data);
    Some(Term::boxed_vaddr(dst))
}

fn copy_ref<M, A>(
    addr: Vaddr,
    header: Header,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    use crate::term::heap::HeapRef;
    let total = HeapRef::SIZE;
    let dst = alloc(total, 8)?;
    visited.record(addr, dst);
    mem.write(dst, header);
    // Copy ref ID (8 bytes after header)
    let ref_id: u64 = mem.read(addr.add(8));
    mem.write(dst.add(8), ref_id);
    Some(Term::boxed_vaddr(dst))
}

fn copy_float<M, A>(
    addr: Vaddr,
    header: Header,
    alloc: &mut A,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Term>
where
    M: MemorySpace,
    A: FnMut(usize, usize) -> Option<Vaddr>,
{
    let total = HeapFloat::SIZE;
    let dst = alloc(total, 8)?;
    visited.record(addr, dst);
    mem.write(dst, header);
    let float_bits: u64 = mem.read(addr.add(8));
    mem.write(dst.add(8), float_bits);
    Some(Term::boxed_vaddr(dst))
}
