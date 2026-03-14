// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for GC utility functions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::gc::utils::{is_in_old_heap, is_in_young_heap, needs_tracing, object_size_from_header};
use crate::platform::MockVSpace;
use crate::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process};
use crate::term::Term;
use crate::term::header::Header;
use crate::term::pair::Pair;
use crate::term::tag::object;

/// Create a test process with mock memory.
fn setup_process() -> (Process, MockVSpace) {
    let young_base = Vaddr::new(0x1000);
    let old_base = Vaddr::new(0x0010_0000);
    let process = Process::new(
        young_base,
        INITIAL_YOUNG_HEAP_SIZE,
        old_base,
        INITIAL_OLD_HEAP_SIZE,
    );
    let mem = MockVSpace::new(1024 * 1024, Vaddr::new(0)); // 1 MB mock memory
    (process, mem)
}

// =============================================================================
// Object Size Tests
// =============================================================================

#[test]
fn object_size_tuple_empty() {
    let header = Header::tuple(0);
    assert_eq!(object_size_from_header(header), 8); // Just header
}

#[test]
fn object_size_tuple_with_elements() {
    let header = Header::tuple(3);
    assert_eq!(object_size_from_header(header), 8 + 3 * 8); // Header + 3 elements
}

#[test]
fn object_size_vector_empty() {
    let header = Header::vector(0);
    assert_eq!(object_size_from_header(header), 16); // Header + length field
}

#[test]
fn object_size_vector_with_capacity() {
    let header = Header::vector(5);
    assert_eq!(object_size_from_header(header), 16 + 5 * 8); // Header + length + capacity elements
}

#[test]
fn object_size_string_empty() {
    let header = Header::string(0);
    assert_eq!(object_size_from_header(header), 8); // Just header
}

#[test]
fn object_size_string_short() {
    let header = Header::string(5);
    assert_eq!(object_size_from_header(header), 8 + 8); // Header + aligned(5) = 8
}

#[test]
fn object_size_string_exactly_aligned() {
    let header = Header::string(8);
    assert_eq!(object_size_from_header(header), 8 + 8); // Header + 8 bytes
}

#[test]
fn object_size_string_needs_padding() {
    let header = Header::string(10);
    assert_eq!(object_size_from_header(header), 8 + 16); // Header + aligned(10) = 16
}

#[test]
fn object_size_map() {
    let header = Header::map(5);
    assert_eq!(object_size_from_header(header), 16); // Fixed size
}

#[test]
fn object_size_float() {
    let header = Header::float();
    assert_eq!(object_size_from_header(header), 16); // Header + f64
}

#[test]
fn object_size_closure_no_captures() {
    let header = Header::closure(0);
    assert_eq!(object_size_from_header(header), 16); // Header + function pointer
}

#[test]
fn object_size_closure_with_captures() {
    let header = Header::closure(3);
    assert_eq!(object_size_from_header(header), 16 + 3 * 8); // Header + fn + 3 captures
}

#[test]
fn object_size_fun() {
    // FUN stores total words in arity
    let total_words = 10u64;
    let header = Header::fun(total_words);
    assert_eq!(object_size_from_header(header), 10 * 8);
}

#[test]
fn object_size_bignum() {
    let header = Header::bignum(3);
    assert_eq!(object_size_from_header(header), 16 + 3 * 8); // Header + sign + 3 limbs
}

#[test]
fn object_size_pid() {
    let header = Header::pid();
    assert_eq!(object_size_from_header(header), 16);
}

#[test]
fn object_size_reference() {
    let header = Header::reference();
    assert_eq!(object_size_from_header(header), 16);
}

#[test]
fn object_size_procbin() {
    let header = Header::procbin();
    assert_eq!(object_size_from_header(header), 24);
}

#[test]
fn object_size_subbin() {
    let header = Header::subbin();
    assert_eq!(object_size_from_header(header), 24);
}

#[test]
fn object_size_namespace() {
    let header = Header::namespace();
    assert_eq!(object_size_from_header(header), 24);
}

#[test]
fn object_size_var() {
    let header = Header::var();
    assert_eq!(object_size_from_header(header), 32);
}

// =============================================================================
// Address Classification Tests
// =============================================================================

#[test]
fn is_in_young_heap_at_base() {
    let (process, _mem) = setup_process();
    assert!(is_in_young_heap(&process, process.heap));
}

#[test]
fn is_in_young_heap_in_middle() {
    let (process, _mem) = setup_process();
    let middle = Vaddr::new(process.heap.as_u64() + INITIAL_YOUNG_HEAP_SIZE as u64 / 2);
    assert!(is_in_young_heap(&process, middle));
}

#[test]
fn is_in_young_heap_before_end() {
    let (process, _mem) = setup_process();
    let before_end = Vaddr::new(process.hend.as_u64() - 8);
    assert!(is_in_young_heap(&process, before_end));
}

#[test]
fn is_in_young_heap_at_end_exclusive() {
    let (process, _mem) = setup_process();
    // End is exclusive
    assert!(!is_in_young_heap(&process, process.hend));
}

#[test]
fn is_in_young_heap_before_base() {
    let (process, _mem) = setup_process();
    let before_base = Vaddr::new(process.heap.as_u64() - 8);
    assert!(!is_in_young_heap(&process, before_base));
}

#[test]
fn is_in_young_heap_way_outside() {
    let (process, _mem) = setup_process();
    assert!(!is_in_young_heap(&process, Vaddr::new(0)));
    assert!(!is_in_young_heap(&process, Vaddr::new(0xFFFF_FFFF)));
}

#[test]
fn is_in_old_heap_at_base() {
    let (process, _mem) = setup_process();
    assert!(is_in_old_heap(&process, process.old_heap));
}

#[test]
fn is_in_old_heap_in_middle() {
    let (process, _mem) = setup_process();
    let middle = Vaddr::new(process.old_heap.as_u64() + INITIAL_OLD_HEAP_SIZE as u64 / 2);
    assert!(is_in_old_heap(&process, middle));
}

#[test]
fn is_in_old_heap_at_end_exclusive() {
    let (process, _mem) = setup_process();
    assert!(!is_in_old_heap(&process, process.old_hend));
}

#[test]
fn is_in_old_heap_not_in_young() {
    let (process, _mem) = setup_process();
    // Young heap address should not be in old heap
    assert!(!is_in_old_heap(&process, process.heap));
}

// =============================================================================
// Needs Tracing Tests
// =============================================================================

#[test]
fn needs_tracing_nil() {
    assert!(!needs_tracing(Term::NIL));
}

#[test]
fn needs_tracing_true() {
    assert!(!needs_tracing(Term::TRUE));
}

#[test]
fn needs_tracing_false() {
    assert!(!needs_tracing(Term::FALSE));
}

#[test]
fn needs_tracing_small_int() {
    let term = Term::small_int(42).unwrap();
    assert!(!needs_tracing(term));
}

#[test]
fn needs_tracing_symbol() {
    let term = Term::symbol(0);
    assert!(!needs_tracing(term));
}

#[test]
fn needs_tracing_keyword() {
    let term = Term::keyword(0);
    assert!(!needs_tracing(term));
}

#[test]
fn needs_tracing_list_pointer() {
    // List pointer should need tracing
    let ptr = 0x1000 as *const Pair;
    let term = Term::list(ptr);
    assert!(needs_tracing(term));
}

#[test]
fn needs_tracing_boxed_pointer() {
    // Boxed pointer should need tracing
    let ptr = 0x1000 as *const Header;
    let term = Term::boxed(ptr);
    assert!(needs_tracing(term));
}

// =============================================================================
// Forwarding Pointer Tests
// =============================================================================

#[test]
fn forwarding_header_creation() {
    let new_addr = 0x2000 as *const u8;
    let forward_header = Header::forward(new_addr);

    assert!(forward_header.is_forward());
    assert_eq!(forward_header.object_tag(), object::FORWARD);
    assert_eq!(forward_header.forward_address(), new_addr);
}

#[test]
fn forwarding_header_preserves_address() {
    // Test various aligned addresses
    for addr in &[0x1000_u64, 0x2000, 0x0001_0000, 0x0010_0000, 0x1_0000_0000] {
        let ptr = (*addr) as *const u8;
        let forward = Header::forward(ptr);
        assert_eq!(forward.forward_address(), ptr);
    }
}

#[test]
fn pair_forwarding() {
    let mut pair = Pair::new(Term::NIL, Term::NIL);

    // Before forwarding
    assert!(!pair.is_forwarded());

    // Set forward
    let new_addr = 0x2000 as *const Pair;
    // SAFETY: Test with mock address
    unsafe { pair.set_forward(new_addr) };

    // After forwarding
    assert!(pair.is_forwarded());
    assert_eq!(pair.forward_address(), new_addr);
}

#[test]
fn pair_forward_marker_is_header() {
    let mut pair = Pair::new(Term::NIL, Term::NIL);
    let new_addr = 0x2000 as *const Pair;

    // SAFETY: Test with mock address
    unsafe { pair.set_forward(new_addr) };

    // The marker in head position should have HEADER primary tag
    assert!(pair.head.is_header());
}
