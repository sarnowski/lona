// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for header words.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::term::heap::{HeapFloat, HeapMap, HeapNamespace, HeapPid, HeapRef, HeapVar};
use crate::term::tag::object;

// ============================================================================
// Compile-Time Size Assertions
// ============================================================================
// These assertions verify that Header::object_size() returns the correct size
// for fixed-size heap objects. This prevents GC memory corruption.

const _: () = assert!(Header::namespace().object_size() == HeapNamespace::SIZE);
const _: () = assert!(Header::var().object_size() == HeapVar::SIZE);
const _: () = assert!(Header::map(0).object_size() == HeapMap::SIZE);
const _: () = assert!(Header::float().object_size() == HeapFloat::SIZE);
const _: () = assert!(Header::pid().object_size() == HeapPid::SIZE);
const _: () = assert!(Header::reference().object_size() == HeapRef::SIZE);

// ============================================================================
// Size and Layout Tests
// ============================================================================

#[test]
fn header_is_8_bytes() {
    assert_eq!(core::mem::size_of::<Header>(), 8);
}

// ============================================================================
// Header Encoding/Decoding Tests
// ============================================================================

#[test]
fn header_encoding_tuple() {
    let header = Header::tuple(5);
    assert_eq!(header.object_tag(), object::TUPLE);
    assert_eq!(header.arity(), 5);
}

#[test]
fn header_encoding_vector() {
    let header = Header::vector(10);
    assert_eq!(header.object_tag(), object::VECTOR);
    assert_eq!(header.arity(), 10);
}

#[test]
fn header_encoding_string() {
    let header = Header::string(42);
    assert_eq!(header.object_tag(), object::STRING);
    assert_eq!(header.arity(), 42);
}

#[test]
fn header_encoding_map() {
    let header = Header::map(3);
    assert_eq!(header.object_tag(), object::MAP);
    assert_eq!(header.arity(), 3);
}

#[test]
fn header_encoding_float() {
    let header = Header::float();
    assert_eq!(header.object_tag(), object::FLOAT);
    assert_eq!(header.arity(), 0);
}

#[test]
fn header_encoding_bignum() {
    let header = Header::bignum(4);
    assert_eq!(header.object_tag(), object::BIGNUM);
    assert_eq!(header.arity(), 4);
}

#[test]
fn header_encoding_fun() {
    let header = Header::fun(10);
    assert_eq!(header.object_tag(), object::FUN);
    assert_eq!(header.arity(), 10);
}

#[test]
fn header_encoding_closure() {
    let header = Header::closure(3);
    assert_eq!(header.object_tag(), object::CLOSURE);
    assert_eq!(header.arity(), 3);
}

#[test]
fn header_encoding_pid() {
    let header = Header::pid();
    assert_eq!(header.object_tag(), object::PID);
}

#[test]
fn header_encoding_reference() {
    let header = Header::reference();
    assert_eq!(header.object_tag(), object::REF);
}

// ============================================================================
// Object Tag Tests
// ============================================================================

#[test]
fn object_tag_extraction() {
    let test_cases = [
        (Header::tuple(0), object::TUPLE),
        (Header::vector(0), object::VECTOR),
        (Header::map(0), object::MAP),
        (Header::string(0), object::STRING),
        (Header::binary(0), object::BINARY),
        (Header::float(), object::FLOAT),
        (Header::bignum(0), object::BIGNUM),
        (Header::fun(0), object::FUN),
        (Header::closure(0), object::CLOSURE),
        (Header::pid(), object::PID),
        (Header::reference(), object::REF),
        (Header::procbin(), object::PROCBIN),
        (Header::subbin(), object::SUBBIN),
        (Header::namespace(), object::NAMESPACE),
        (Header::var(), object::VAR),
    ];

    for (header, expected_tag) in test_cases {
        assert_eq!(
            header.object_tag(),
            expected_tag,
            "Failed for header {header:?}"
        );
    }
}

// ============================================================================
// Arity Tests
// ============================================================================

#[test]
fn arity_zero() {
    let header = Header::tuple(0);
    assert_eq!(header.arity(), 0);
}

#[test]
fn arity_small() {
    let header = Header::tuple(5);
    assert_eq!(header.arity(), 5);
}

#[test]
fn arity_large() {
    // 54 bits available for arity
    let large_arity = 1u64 << 40;
    let header = Header::new(object::TUPLE, large_arity);
    assert_eq!(header.arity(), large_arity);
}

#[test]
fn arity_max() {
    // Maximum 54-bit value
    let max_arity = (1u64 << 54) - 1;
    let header = Header::new(object::TUPLE, max_arity);
    assert_eq!(header.arity(), max_arity);
}

// ============================================================================
// Forwarding Pointer Tests
// ============================================================================

#[test]
fn non_forward_header_is_not_forward() {
    assert!(!Header::tuple(5).is_forward());
    assert!(!Header::vector(10).is_forward());
    assert!(!Header::string(42).is_forward());
}

#[test]
fn forward_header_is_forward() {
    // Create a fake aligned address
    let addr = 0x1000_0000_0000_0000u64 as *const u8;
    let forward = Header::forward(addr);
    assert!(forward.is_forward());
    assert_eq!(forward.object_tag(), object::FORWARD);
}

#[test]
fn forward_address_round_trip() {
    // Test with various 8-byte aligned addresses that fit in the arity field.
    // Arity field is 54 bits, and we store address >> 3, so addresses up to
    // 54 + 3 = 57 bits are valid (max is about 0x01FF_FFFF_FFFF_FFF8).
    let test_addresses = [
        0x0000_0000_0000_0008u64,
        0x0000_0000_0001_0000u64,
        0x0000_1234_5678_0000u64,
        0x00FF_FFFF_FFFF_FFF8u64, // 56-bit address (fits in 54-bit arity after >> 3)
        0x01FF_FFFF_FFFF_FFF8u64, // 57-bit address (max valid)
    ];

    for &addr in &test_addresses {
        let ptr = addr as *const u8;
        let forward = Header::forward(ptr);
        assert!(forward.is_forward());
        let extracted = forward.forward_address();
        assert_eq!(
            extracted as u64, addr,
            "Round-trip failed for address {addr:#x}"
        );
    }
}

// ============================================================================
// Object Size Tests
// ============================================================================

#[test]
fn object_size_tuple() {
    assert_eq!(Header::tuple(0).object_size(), 8); // just header
    assert_eq!(Header::tuple(1).object_size(), 16); // header + 1 element
    assert_eq!(Header::tuple(3).object_size(), 32); // header + 3 elements
}

#[test]
fn object_size_vector() {
    // Vector: header (8) + length (8) + capacity * 8
    assert_eq!(Header::vector(0).object_size(), 16); // header + length
    assert_eq!(Header::vector(5).object_size(), 56); // 16 + 5*8
}

#[test]
fn object_size_string() {
    // String: header (8) + aligned byte length
    assert_eq!(Header::string(0).object_size(), 8); // just header
    assert_eq!(Header::string(1).object_size(), 16); // 8 + align8(1)
    assert_eq!(Header::string(8).object_size(), 16); // 8 + align8(8)
    assert_eq!(Header::string(9).object_size(), 24); // 8 + align8(9)
}

#[test]
fn object_size_map() {
    assert_eq!(Header::map(0).object_size(), 16);
    assert_eq!(Header::map(100).object_size(), 16); // arity doesn't affect size
}

#[test]
fn object_size_float() {
    assert_eq!(Header::float().object_size(), 16);
}

#[test]
fn object_size_bignum() {
    assert_eq!(Header::bignum(0).object_size(), 16); // header + sign/len
    assert_eq!(Header::bignum(2).object_size(), 32); // 16 + 2*8
}

#[test]
fn object_size_fun() {
    // FUN stores total size in words in arity
    assert_eq!(Header::fun(2).object_size(), 16); // 2 * 8
    assert_eq!(Header::fun(10).object_size(), 80); // 10 * 8
}

#[test]
fn object_size_closure() {
    // Closure: header (8) + fun ptr (8) + captures
    assert_eq!(Header::closure(0).object_size(), 16);
    assert_eq!(Header::closure(3).object_size(), 40); // 16 + 3*8
}

#[test]
fn object_size_pid() {
    assert_eq!(Header::pid().object_size(), 16);
}

#[test]
fn object_size_ref() {
    assert_eq!(Header::reference().object_size(), 16);
}

#[test]
fn object_size_procbin() {
    assert_eq!(Header::procbin().object_size(), 24);
}

#[test]
fn object_size_subbin() {
    assert_eq!(Header::subbin().object_size(), 24);
}

#[test]
fn object_size_namespace() {
    // HeapNamespace: header (8) + name (8) + mappings (8) = 24
    assert_eq!(Header::namespace().object_size(), 24);
}

#[test]
fn object_size_var() {
    // HeapVar: header (8) + name (8) + namespace (8) + root (8) = 32
    assert_eq!(Header::var().object_size(), 32);
}

#[test]
fn object_size_forward() {
    let forward = Header::forward(0x1000 as *const u8);
    assert_eq!(forward.object_size(), 8);
}

// ============================================================================
// Term Extension Tests
// ============================================================================

#[test]
fn boxed_pointer_round_trip() {
    let header = Header::tuple(5);
    let ptr = &raw const header;

    let term = Term::boxed(ptr);

    assert!(term.is_boxed());
    assert!(!term.is_list());
    assert!(!term.is_immediate());

    let extracted = term.as_header_ptr().unwrap();
    assert_eq!(extracted, ptr);
}

#[test]
fn boxed_type_name() {
    let header = Header::tuple(3);
    let term = Term::boxed(&raw const header);
    assert_eq!(term.type_name(), "boxed");
}

#[test]
fn non_boxed_returns_none_for_as_header_ptr() {
    assert!(Term::NIL.as_header_ptr().is_none());
    assert!(Term::TRUE.as_header_ptr().is_none());
    assert!(Term::small_int(42).unwrap().as_header_ptr().is_none());
}

#[test]
fn boxed_header_extraction() {
    let header = Header::vector(10);
    let term = Term::boxed(&raw const header);

    unsafe {
        let extracted = term.header().unwrap();
        assert_eq!(extracted.object_tag(), object::VECTOR);
        assert_eq!(extracted.arity(), 10);
    }
}

#[test]
fn boxed_object_tag_extraction() {
    let header = Header::string(100);
    let term = Term::boxed(&raw const header);

    unsafe {
        assert_eq!(term.object_tag(), Some(object::STRING));
    }
}

// ============================================================================
// Type Checking Helper Tests
// ============================================================================

#[test]
fn type_checking_helpers() {
    let tuple_header = Header::tuple(3);
    let vector_header = Header::vector(5);
    let string_header = Header::string(10);
    let map_header = Header::map(2);
    let float_header = Header::float();

    let tuple_term = Term::boxed(&raw const tuple_header);
    let vector_term = Term::boxed(&raw const vector_header);
    let string_term = Term::boxed(&raw const string_header);
    let map_term = Term::boxed(&raw const map_header);
    let float_term = Term::boxed(&raw const float_header);

    unsafe {
        assert!(tuple_term.is_tuple());
        assert!(!tuple_term.is_vector());

        assert!(vector_term.is_vector());
        assert!(!vector_term.is_tuple());

        assert!(string_term.is_string());
        assert!(!string_term.is_map());

        assert!(map_term.is_map());
        assert!(!map_term.is_string());

        assert!(float_term.is_float());
        assert!(!float_term.is_bignum());
    }
}

#[test]
fn non_boxed_type_checks_return_false() {
    unsafe {
        assert!(!Term::NIL.is_tuple());
        assert!(!Term::small_int(42).unwrap().is_vector());
        assert!(!Term::symbol(0).is_string());
    }
}

// ============================================================================
// Debug Format Tests
// ============================================================================

#[test]
fn debug_format_tuple() {
    let header = Header::tuple(5);
    let debug = format!("{header:?}");
    assert!(debug.contains("TUPLE"));
    assert!(debug.contains("arity=5"));
}

#[test]
fn debug_format_forward() {
    let forward = Header::forward(0x1000 as *const u8);
    let debug = format!("{forward:?}");
    assert!(debug.contains("FORWARD"));
}
