// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for heap fragments.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::heap_fragment::HeapFragment;
use crate::Vaddr;
use crate::platform::{MemorySpace, MockVSpace};
use crate::term::Term;

/// Create a fragment backed by a `MockVSpace` region.
fn make_fragment(base: u64, size: usize) -> HeapFragment {
    HeapFragment::new(Vaddr::new(base), size)
}

#[test]
fn new_fragment_is_empty() {
    let frag = make_fragment(0x1000, 256);
    assert_eq!(frag.used(), 0);
    assert_eq!(frag.capacity(), 256);
    assert_eq!(frag.base(), Vaddr::new(0x1000));
    assert_eq!(frag.message(), Term::NIL);
}

#[test]
fn alloc_advances_bump_pointer() {
    let mut frag = make_fragment(0x1000, 256);

    let addr1 = frag.alloc(16, 8).unwrap();
    assert_eq!(addr1, Vaddr::new(0x1000));
    assert_eq!(frag.used(), 16);

    let addr2 = frag.alloc(32, 8).unwrap();
    assert_eq!(addr2, Vaddr::new(0x1010));
    assert_eq!(frag.used(), 48);
}

#[test]
fn alloc_respects_alignment() {
    let mut frag = make_fragment(0x1000, 256);

    // Allocate 3 bytes (not 8-aligned)
    let addr1 = frag.alloc(3, 1).unwrap();
    assert_eq!(addr1, Vaddr::new(0x1000));
    assert_eq!(frag.used(), 3);

    // Next 8-aligned allocation should skip to 0x1008
    let addr2 = frag.alloc(8, 8).unwrap();
    assert_eq!(addr2, Vaddr::new(0x1008));
    assert_eq!(frag.used(), 16);
}

#[test]
fn alloc_returns_none_when_full() {
    let mut frag = make_fragment(0x1000, 32);

    let addr1 = frag.alloc(24, 8).unwrap();
    assert_eq!(addr1, Vaddr::new(0x1000));

    // Only 8 bytes left, but asking for 16
    assert!(frag.alloc(16, 8).is_none());

    // Exactly 8 bytes left
    let addr2 = frag.alloc(8, 8).unwrap();
    assert_eq!(addr2, Vaddr::new(0x1018));

    // Now completely full
    assert!(frag.alloc(1, 1).is_none());
}

#[test]
fn alloc_zero_bytes() {
    let mut frag = make_fragment(0x1000, 256);
    let addr = frag.alloc(0, 8).unwrap();
    assert_eq!(addr, Vaddr::new(0x1000));
    assert_eq!(frag.used(), 0); // zero-size alloc doesn't advance
}

#[test]
fn set_and_get_message() {
    let mut frag = make_fragment(0x1000, 256);
    assert_eq!(frag.message(), Term::NIL);

    let msg = Term::small_int(42).unwrap();
    frag.set_message(msg);
    assert_eq!(frag.message(), msg);
}

#[test]
fn linked_list_chaining() {
    let mut frag1 = HeapFragment::new(Vaddr::new(0x1000), 64);
    let mut frag2 = HeapFragment::new(Vaddr::new(0x2000), 64);
    let frag3 = HeapFragment::new(Vaddr::new(0x3000), 64);

    frag1.set_message(Term::small_int(1).unwrap());
    frag2.set_message(Term::small_int(2).unwrap());

    // Chain: frag1 -> frag2 -> frag3
    frag2.next = Some(Box::new(frag3));
    frag1.next = Some(Box::new(frag2));

    // Walk the chain
    assert_eq!(frag1.message(), Term::small_int(1).unwrap());
    let f2 = frag1.next.as_ref().unwrap();
    assert_eq!(f2.message(), Term::small_int(2).unwrap());
    assert!(f2.next.is_some());
    let f3 = f2.next.as_ref().unwrap();
    assert_eq!(f3.base(), Vaddr::new(0x3000));
    assert!(f3.next.is_none());
}

#[test]
fn fragment_memory_is_addressable_via_memoryspace() {
    // Allocate a fragment within MockVSpace's range
    let base = Vaddr::new(0x1000);
    let mut mem = MockVSpace::new(4096, base);
    let mut frag = HeapFragment::new(base, 256);

    // Allocate 8 bytes from fragment
    let addr = frag.alloc(8, 8).unwrap();
    assert_eq!(addr, base);

    // Write and read through MemorySpace
    let value: u64 = 0xDEAD_BEEF;
    mem.write(addr, value);
    let read_back: u64 = mem.read(addr);
    assert_eq!(read_back, value);
}

#[test]
fn top_addr() {
    let mut frag = make_fragment(0x1000, 256);
    assert_eq!(frag.top_addr(), Vaddr::new(0x1000));

    frag.alloc(24, 8).unwrap();
    assert_eq!(frag.top_addr(), Vaddr::new(0x1018));
}
