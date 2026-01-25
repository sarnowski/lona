// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for Term allocation methods.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::platform::{MemorySpace, MockVSpace};
use crate::process::Process;
use crate::term::Term;
use crate::term::header::Header;
use crate::term::tag::object;

fn setup() -> (Process, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mem_size = 128 * 1024;
    let mem = MockVSpace::new(mem_size, base);

    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;

    let proc = Process::new(young_base, young_size, old_base, old_size);
    (proc, mem)
}

// ============================================================================
// String Tests
// ============================================================================

#[test]
fn alloc_term_string_empty() {
    let (mut proc, mut mem) = setup();

    let term = proc.alloc_term_string(&mut mem, "").unwrap();
    assert!(term.is_boxed());

    let s = proc.read_term_string(&mem, term).unwrap();
    assert_eq!(s, "");
}

#[test]
fn alloc_term_string_hello() {
    let (mut proc, mut mem) = setup();

    let term = proc.alloc_term_string(&mut mem, "hello").unwrap();
    assert!(term.is_boxed());

    let s = proc.read_term_string(&mem, term).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn alloc_term_string_unicode() {
    let (mut proc, mut mem) = setup();

    let term = proc.alloc_term_string(&mut mem, "こんにちは").unwrap();
    assert!(term.is_boxed());

    let s = proc.read_term_string(&mem, term).unwrap();
    assert_eq!(s, "こんにちは");
}

#[test]
fn read_term_string_wrong_type() {
    let (proc, mem) = setup();

    // Immediate value is not a string
    assert!(proc.read_term_string(&mem, Term::NIL).is_none());
    assert!(
        proc.read_term_string(&mem, Term::small_int(42).unwrap())
            .is_none()
    );
}

// ============================================================================
// Pair Tests
// ============================================================================

#[test]
fn alloc_term_pair_simple() {
    let (mut proc, mut mem) = setup();

    let head = Term::small_int(1).unwrap();
    let rest = Term::NIL;

    let term = proc.alloc_term_pair(&mut mem, head, rest).unwrap();
    assert!(term.is_list());

    let (h, r) = proc.read_term_pair(&mem, term).unwrap();
    assert_eq!(h.as_small_int(), Some(1));
    assert!(r.is_nil());
}

#[test]
fn alloc_term_pair_nested() {
    let (mut proc, mut mem) = setup();

    // Build list (1 2 3)
    let tail = Term::NIL;
    let pair3 = proc
        .alloc_term_pair(&mut mem, Term::small_int(3).unwrap(), tail)
        .unwrap();
    let pair2 = proc
        .alloc_term_pair(&mut mem, Term::small_int(2).unwrap(), pair3)
        .unwrap();
    let pair1 = proc
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), pair2)
        .unwrap();

    // Read first element
    let (h1, r1) = proc.read_term_pair(&mem, pair1).unwrap();
    assert_eq!(h1.as_small_int(), Some(1));

    // Read second element
    let (h2, r2) = proc.read_term_pair(&mem, r1).unwrap();
    assert_eq!(h2.as_small_int(), Some(2));

    // Read third element
    let (h3, r3) = proc.read_term_pair(&mem, r2).unwrap();
    assert_eq!(h3.as_small_int(), Some(3));
    assert!(r3.is_nil());
}

#[test]
fn read_term_pair_wrong_type() {
    let (proc, mem) = setup();

    assert!(proc.read_term_pair(&mem, Term::NIL).is_none());
    assert!(
        proc.read_term_pair(&mem, Term::small_int(42).unwrap())
            .is_none()
    );
}

// ============================================================================
// Tuple Tests
// ============================================================================

#[test]
fn alloc_term_tuple_empty() {
    let (mut proc, mut mem) = setup();

    let term = proc.alloc_term_tuple(&mut mem, &[]).unwrap();
    assert!(term.is_boxed());

    let len = proc.read_term_tuple_len(&mem, term).unwrap();
    assert_eq!(len, 0);
}

#[test]
fn alloc_term_tuple_with_elements() {
    let (mut proc, mut mem) = setup();

    let elements = [
        Term::small_int(1).unwrap(),
        Term::small_int(2).unwrap(),
        Term::small_int(3).unwrap(),
    ];

    let term = proc.alloc_term_tuple(&mut mem, &elements).unwrap();
    assert!(term.is_boxed());

    let len = proc.read_term_tuple_len(&mem, term).unwrap();
    assert_eq!(len, 3);

    assert_eq!(
        proc.read_term_tuple_element(&mem, term, 0)
            .unwrap()
            .as_small_int(),
        Some(1)
    );
    assert_eq!(
        proc.read_term_tuple_element(&mem, term, 1)
            .unwrap()
            .as_small_int(),
        Some(2)
    );
    assert_eq!(
        proc.read_term_tuple_element(&mem, term, 2)
            .unwrap()
            .as_small_int(),
        Some(3)
    );
}

#[test]
fn read_term_tuple_element_out_of_bounds() {
    let (mut proc, mut mem) = setup();

    let elements = [Term::small_int(1).unwrap()];
    let term = proc.alloc_term_tuple(&mut mem, &elements).unwrap();

    assert!(proc.read_term_tuple_element(&mem, term, 1).is_none());
    assert!(proc.read_term_tuple_element(&mem, term, 100).is_none());
}

#[test]
fn read_term_tuple_wrong_type() {
    let (proc, mem) = setup();

    assert!(proc.read_term_tuple_len(&mem, Term::NIL).is_none());
}

// ============================================================================
// Vector Tests
// ============================================================================

#[test]
fn alloc_term_vector_empty() {
    let (mut proc, mut mem) = setup();

    let term = proc.alloc_term_vector(&mut mem, &[]).unwrap();
    assert!(term.is_boxed());

    let len = proc.read_term_vector_len(&mem, term).unwrap();
    assert_eq!(len, 0);
}

#[test]
fn alloc_term_vector_with_elements() {
    let (mut proc, mut mem) = setup();

    let elements = [
        Term::small_int(10).unwrap(),
        Term::small_int(20).unwrap(),
        Term::small_int(30).unwrap(),
    ];

    let term = proc.alloc_term_vector(&mut mem, &elements).unwrap();
    assert!(term.is_boxed());

    let len = proc.read_term_vector_len(&mem, term).unwrap();
    assert_eq!(len, 3);

    assert_eq!(
        proc.read_term_vector_element(&mem, term, 0)
            .unwrap()
            .as_small_int(),
        Some(10)
    );
    assert_eq!(
        proc.read_term_vector_element(&mem, term, 1)
            .unwrap()
            .as_small_int(),
        Some(20)
    );
    assert_eq!(
        proc.read_term_vector_element(&mem, term, 2)
            .unwrap()
            .as_small_int(),
        Some(30)
    );
}

#[test]
fn read_term_vector_element_out_of_bounds() {
    let (mut proc, mut mem) = setup();

    let elements = [Term::small_int(1).unwrap()];
    let term = proc.alloc_term_vector(&mut mem, &elements).unwrap();

    assert!(proc.read_term_vector_element(&mem, term, 1).is_none());
}

// ============================================================================
// Float Tests
// ============================================================================

#[test]
fn alloc_term_float() {
    let (mut proc, mut mem) = setup();

    let term = proc.alloc_term_float(&mut mem, 42.5).unwrap();
    assert!(term.is_boxed());

    let value = proc.read_term_float(&mem, term).unwrap();
    assert!((value - 42.5).abs() < f64::EPSILON);
}

#[test]
fn read_term_float_wrong_type() {
    let (proc, mem) = setup();

    assert!(proc.read_term_float(&mem, Term::NIL).is_none());
    assert!(
        proc.read_term_float(&mem, Term::small_int(42).unwrap())
            .is_none()
    );
}

// ============================================================================
// Map Tests
// ============================================================================

#[test]
fn alloc_term_map_empty() {
    let (mut proc, mut mem) = setup();

    let term = proc.alloc_term_map(&mut mem, Term::NIL, 0).unwrap();
    assert!(term.is_boxed());

    let entries = proc.read_term_map_entries(&mem, term).unwrap();
    assert!(entries.is_nil());
}

#[test]
fn alloc_term_map_with_entries() {
    let (mut proc, mut mem) = setup();

    // Create a simple entry: [key, value]
    let key = Term::small_int(1).unwrap();
    let value = Term::small_int(100).unwrap();
    let entry = proc.alloc_term_tuple(&mut mem, &[key, value]).unwrap();

    // Create the pair chain
    let entries = proc.alloc_term_pair(&mut mem, entry, Term::NIL).unwrap();

    // Create the map
    let term = proc.alloc_term_map(&mut mem, entries, 1).unwrap();
    assert!(term.is_boxed());

    // Read entries back
    let read_entries = proc.read_term_map_entries(&mem, term).unwrap();
    assert!(read_entries.is_list());
}

// ============================================================================
// Closure Tests
// ============================================================================

#[test]
fn alloc_term_closure_no_captures() {
    let (mut proc, mut mem) = setup();

    // Use a placeholder for the function reference
    let function = Term::NIL; // In real code, this would be a function Term
    let captures: &[Term] = &[];

    let term = proc
        .alloc_term_closure(&mut mem, function, captures)
        .unwrap();
    assert!(term.is_boxed());

    // Verify it's a closure by checking the header
    let addr = Vaddr::new(term.as_raw() & !0b11);
    let header: Header = mem.read(addr);
    assert_eq!(header.object_tag(), object::CLOSURE);
    assert_eq!(header.arity(), 0); // No captures
}

#[test]
fn alloc_term_closure_with_captures() {
    let (mut proc, mut mem) = setup();

    let function = Term::NIL;
    let captures = [Term::small_int(1).unwrap(), Term::small_int(2).unwrap()];

    let term = proc
        .alloc_term_closure(&mut mem, function, &captures)
        .unwrap();
    assert!(term.is_boxed());

    let addr = Vaddr::new(term.as_raw() & !0b11);
    let header: Header = mem.read(addr);
    assert_eq!(header.object_tag(), object::CLOSURE);
    assert_eq!(header.arity(), 2); // 2 captures
}

// ============================================================================
// Object Tag Verification Tests
// ============================================================================

#[test]
fn verify_object_tags() {
    let (mut proc, mut mem) = setup();

    // String
    let string_term = proc.alloc_term_string(&mut mem, "test").unwrap();
    let string_addr = Vaddr::new(string_term.as_raw() & !0b11);
    let string_header: Header = mem.read(string_addr);
    assert_eq!(string_header.object_tag(), object::STRING);

    // Tuple
    let tuple_term = proc.alloc_term_tuple(&mut mem, &[Term::NIL]).unwrap();
    let tuple_addr = Vaddr::new(tuple_term.as_raw() & !0b11);
    let tuple_header: Header = mem.read(tuple_addr);
    assert_eq!(tuple_header.object_tag(), object::TUPLE);

    // Vector
    let vector_term = proc.alloc_term_vector(&mut mem, &[Term::NIL]).unwrap();
    let vector_addr = Vaddr::new(vector_term.as_raw() & !0b11);
    let vector_header: Header = mem.read(vector_addr);
    assert_eq!(vector_header.object_tag(), object::VECTOR);

    // Float
    let float_term = proc.alloc_term_float(&mut mem, 1.0).unwrap();
    let float_addr = Vaddr::new(float_term.as_raw() & !0b11);
    let float_header: Header = mem.read(float_addr);
    assert_eq!(float_header.object_tag(), object::FLOAT);

    // Map
    let map_term = proc.alloc_term_map(&mut mem, Term::NIL, 0).unwrap();
    let map_addr = Vaddr::new(map_term.as_raw() & !0b11);
    let map_header: Header = mem.read(map_addr);
    assert_eq!(map_header.object_tag(), object::MAP);
}
