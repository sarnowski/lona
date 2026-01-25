// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for heap object types.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::term::tag::object;

// ============================================================================
// Size and Layout Tests
// ============================================================================

#[test]
fn heap_tuple_header_is_8_bytes() {
    assert_eq!(core::mem::size_of::<HeapTuple>(), 8);
}

#[test]
fn heap_vector_prefix_is_16_bytes() {
    assert_eq!(core::mem::size_of::<HeapVector>(), 16);
}

#[test]
fn heap_string_header_is_8_bytes() {
    assert_eq!(core::mem::size_of::<HeapString>(), 8);
}

#[test]
fn heap_map_is_16_bytes() {
    assert_eq!(core::mem::size_of::<HeapMap>(), 16);
}

#[test]
fn heap_float_is_16_bytes() {
    assert_eq!(core::mem::size_of::<HeapFloat>(), 16);
}

#[test]
fn heap_fun_prefix_is_16_bytes() {
    assert_eq!(core::mem::size_of::<HeapFun>(), 16);
}

#[test]
fn heap_closure_prefix_is_16_bytes() {
    assert_eq!(core::mem::size_of::<HeapClosure>(), 16);
}

#[test]
fn heap_pid_is_16_bytes() {
    assert_eq!(core::mem::size_of::<HeapPid>(), 16);
}

#[test]
fn heap_ref_is_16_bytes() {
    assert_eq!(core::mem::size_of::<HeapRef>(), 16);
}

#[test]
fn heap_bignum_prefix_is_16_bytes() {
    assert_eq!(core::mem::size_of::<HeapBignum>(), 16);
}

#[test]
fn heap_var_is_32_bytes() {
    // HeapVar: header (8) + name (8) + namespace (8) + root (8) = 32 bytes
    assert_eq!(core::mem::size_of::<HeapVar>(), 32);
}

// ============================================================================
// Allocation Size Tests
// ============================================================================

#[test]
fn tuple_alloc_size() {
    assert_eq!(HeapTuple::alloc_size(0), 8); // Just header
    assert_eq!(HeapTuple::alloc_size(1), 16); // Header + 1 element
    assert_eq!(HeapTuple::alloc_size(3), 32); // Header + 3 elements
    assert_eq!(HeapTuple::alloc_size(10), 88); // Header + 10 elements
}

#[test]
fn vector_alloc_size() {
    assert_eq!(HeapVector::alloc_size(0), 16); // Prefix only
    assert_eq!(HeapVector::alloc_size(1), 24); // Prefix + 1 element
    assert_eq!(HeapVector::alloc_size(5), 56); // Prefix + 5 elements
}

#[test]
fn string_alloc_size() {
    assert_eq!(HeapString::alloc_size(0), 8); // Just header
    assert_eq!(HeapString::alloc_size(1), 16); // Header + align8(1)
    assert_eq!(HeapString::alloc_size(8), 16); // Header + align8(8)
    assert_eq!(HeapString::alloc_size(9), 24); // Header + align8(9)
    assert_eq!(HeapString::alloc_size(16), 24); // Header + align8(16)
}

#[test]
fn closure_alloc_size() {
    assert_eq!(HeapClosure::alloc_size(0), 16); // Prefix only
    assert_eq!(HeapClosure::alloc_size(1), 24); // Prefix + 1 capture
    assert_eq!(HeapClosure::alloc_size(3), 40); // Prefix + 3 captures
}

#[test]
fn bignum_alloc_size() {
    assert_eq!(HeapBignum::alloc_size(0), 16); // Prefix only
    assert_eq!(HeapBignum::alloc_size(1), 24); // Prefix + 1 limb
    assert_eq!(HeapBignum::alloc_size(4), 48); // Prefix + 4 limbs
}

// ============================================================================
// Header Creation Tests
// ============================================================================

#[test]
fn tuple_header_has_correct_tag() {
    let header = HeapTuple::make_header(5);
    assert_eq!(header.object_tag(), object::TUPLE);
    assert_eq!(header.arity(), 5);
}

#[test]
fn vector_header_has_correct_tag() {
    let header = HeapVector::make_header(10);
    assert_eq!(header.object_tag(), object::VECTOR);
    assert_eq!(header.arity(), 10);
}

#[test]
fn string_header_has_correct_tag() {
    let header = HeapString::make_header(42);
    assert_eq!(header.object_tag(), object::STRING);
    assert_eq!(header.arity(), 42);
}

#[test]
fn map_header_has_correct_tag() {
    let header = HeapMap::make_header(3);
    assert_eq!(header.object_tag(), object::MAP);
    assert_eq!(header.arity(), 3);
}

#[test]
fn float_header_has_correct_tag() {
    let header = HeapFloat::make_header();
    assert_eq!(header.object_tag(), object::FLOAT);
}

#[test]
fn fun_header_has_correct_tag() {
    let header = HeapFun::make_header(10, 5);
    assert_eq!(header.object_tag(), object::FUN);
    // arity stores total words, not function arity
}

#[test]
fn closure_header_has_correct_tag() {
    let header = HeapClosure::make_header(3);
    assert_eq!(header.object_tag(), object::CLOSURE);
    assert_eq!(header.arity(), 3);
}

// ============================================================================
// Tuple Tests (with in-memory simulation)
// ============================================================================

#[test]
fn tuple_len_and_is_empty() {
    // Simulate a tuple in memory
    #[repr(C)]
    struct TupleWithElements {
        tuple: HeapTuple,
        elements: [Term; 3],
    }

    let data = TupleWithElements {
        tuple: HeapTuple {
            header: HeapTuple::make_header(3),
        },
        elements: [
            Term::small_int(1).unwrap(),
            Term::small_int(2).unwrap(),
            Term::small_int(3).unwrap(),
        ],
    };

    assert_eq!(data.tuple.len(), 3);
    assert!(!data.tuple.is_empty());

    // Empty tuple
    let empty = HeapTuple {
        header: HeapTuple::make_header(0),
    };
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
}

#[test]
fn tuple_get_and_set() {
    #[repr(C)]
    struct TupleWithElements {
        tuple: HeapTuple,
        elements: [Term; 3],
    }

    let data = TupleWithElements {
        tuple: HeapTuple {
            header: HeapTuple::make_header(3),
        },
        elements: [
            Term::small_int(10).unwrap(),
            Term::small_int(20).unwrap(),
            Term::small_int(30).unwrap(),
        ],
    };

    unsafe {
        assert_eq!(data.tuple.get(0), Some(Term::small_int(10).unwrap()));
        assert_eq!(data.tuple.get(1), Some(Term::small_int(20).unwrap()));
        assert_eq!(data.tuple.get(2), Some(Term::small_int(30).unwrap()));
        assert_eq!(data.tuple.get(3), None); // Out of bounds
    }
}

#[test]
fn tuple_elements_slice() {
    #[repr(C)]
    struct TupleWithElements {
        tuple: HeapTuple,
        elements: [Term; 3],
    }

    let data = TupleWithElements {
        tuple: HeapTuple {
            header: HeapTuple::make_header(3),
        },
        elements: [
            Term::small_int(1).unwrap(),
            Term::small_int(2).unwrap(),
            Term::small_int(3).unwrap(),
        ],
    };

    unsafe {
        let elements = data.tuple.elements();
        assert_eq!(elements.len(), 3);
        assert_eq!(elements[0].as_small_int(), Some(1));
        assert_eq!(elements[1].as_small_int(), Some(2));
        assert_eq!(elements[2].as_small_int(), Some(3));
    }
}

// ============================================================================
// Vector Tests
// ============================================================================

#[test]
fn vector_len_and_capacity() {
    #[repr(C)]
    struct VectorWithElements {
        vector: HeapVector,
        elements: [Term; 5],
    }

    let data = VectorWithElements {
        vector: HeapVector {
            header: HeapVector::make_header(5), // capacity = 5
            length: 3,                          // length = 3
        },
        elements: [
            Term::small_int(1).unwrap(),
            Term::small_int(2).unwrap(),
            Term::small_int(3).unwrap(),
            Term::NIL, // unused capacity
            Term::NIL, // unused capacity
        ],
    };

    assert_eq!(data.vector.capacity(), 5);
    assert_eq!(data.vector.len(), 3);
    assert!(!data.vector.is_empty());
}

#[test]
fn vector_get() {
    #[repr(C)]
    struct VectorWithElements {
        vector: HeapVector,
        elements: [Term; 3],
    }

    let data = VectorWithElements {
        vector: HeapVector {
            header: HeapVector::make_header(3),
            length: 2, // Only 2 elements are valid
        },
        elements: [
            Term::small_int(10).unwrap(),
            Term::small_int(20).unwrap(),
            Term::NIL,
        ],
    };

    unsafe {
        assert_eq!(data.vector.get(0), Some(Term::small_int(10).unwrap()));
        assert_eq!(data.vector.get(1), Some(Term::small_int(20).unwrap()));
        assert_eq!(data.vector.get(2), None); // Beyond length
    }
}

// ============================================================================
// String Tests
// ============================================================================

#[test]
fn string_len_and_is_empty() {
    let empty = HeapString {
        header: HeapString::make_header(0),
    };
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());

    let non_empty = HeapString {
        header: HeapString::make_header(5),
    };
    assert_eq!(non_empty.len(), 5);
    assert!(!non_empty.is_empty());
}

#[test]
fn string_as_bytes() {
    #[repr(C)]
    struct StringWithData {
        string: HeapString,
        data: [u8; 8], // Padded to 8 bytes
    }

    let data = StringWithData {
        string: HeapString {
            header: HeapString::make_header(5),
        },
        data: *b"hello\0\0\0", // 5 bytes + padding
    };

    unsafe {
        let bytes = data.string.as_bytes();
        assert_eq!(bytes, b"hello");
    }
}

#[test]
fn string_as_str() {
    #[repr(C)]
    struct StringWithData {
        string: HeapString,
        data: [u8; 8],
    }

    let data = StringWithData {
        string: HeapString {
            header: HeapString::make_header(5),
        },
        data: *b"world\0\0\0",
    };

    unsafe {
        let s = data.string.as_str();
        assert_eq!(s, Some("world"));
    }
}

// ============================================================================
// Map Tests
// ============================================================================

#[test]
fn map_is_empty() {
    let empty_map = HeapMap {
        header: HeapMap::make_header(0),
        entries: Term::NIL,
    };
    assert!(empty_map.is_empty());
    assert_eq!(empty_map.entry_count(), 0);
}

// ============================================================================
// Float Tests
// ============================================================================

#[test]
fn float_get_value() {
    let f = HeapFloat {
        header: HeapFloat::make_header(),
        value: 42.5,
    };
    assert!((f.get() - 42.5).abs() < f64::EPSILON);
}

// ============================================================================
// Function Tests
// ============================================================================

#[test]
fn fun_total_words() {
    // 16 bytes prefix + 10 bytes code + 2 constants (16 bytes)
    // = 42 bytes total, rounded up to 6 words (48 bytes)
    let words = HeapFun::total_words(10, 2);
    assert_eq!(words, 6);
}

#[test]
fn fun_alloc_size() {
    // alloc_size includes padding to align constants at 8-byte boundary
    assert_eq!(HeapFun::alloc_size(0, 0), 16); // Just prefix (already aligned)
    assert_eq!(HeapFun::alloc_size(10, 0), 32); // Prefix + 10 bytes code + padding
    assert_eq!(HeapFun::alloc_size(0, 2), 32); // Prefix + 2 constants (16 bytes)
    assert_eq!(HeapFun::alloc_size(10, 2), 48); // Prefix + code + padding + constants

    // Test alignment calculation
    assert_eq!(HeapFun::constants_offset(0), 16); // No code, starts at prefix end
    assert_eq!(HeapFun::constants_offset(4), 24); // 16+4=20, rounded up to 24
    assert_eq!(HeapFun::constants_offset(8), 24); // 16+8=24, already aligned
    assert_eq!(HeapFun::constants_offset(10), 32); // 16+10=26, rounded up to 32
}

// ============================================================================
// Closure Tests
// ============================================================================

#[test]
fn closure_capture_count() {
    #[repr(C)]
    struct ClosureWithCaptures {
        closure: HeapClosure,
        captures: [Term; 2],
    }

    let data = ClosureWithCaptures {
        closure: HeapClosure {
            header: HeapClosure::make_header(2),
            function: Term::NIL, // Placeholder
        },
        captures: [Term::small_int(1).unwrap(), Term::small_int(2).unwrap()],
    };

    assert_eq!(data.closure.capture_count(), 2);

    unsafe {
        assert_eq!(
            data.closure.get_capture(0),
            Some(Term::small_int(1).unwrap())
        );
        assert_eq!(
            data.closure.get_capture(1),
            Some(Term::small_int(2).unwrap())
        );
        assert_eq!(data.closure.get_capture(2), None);
    }
}

// ============================================================================
// Bignum Tests
// ============================================================================

#[test]
fn bignum_sign_and_limbs() {
    #[repr(C)]
    struct BignumWithLimbs {
        bignum: HeapBignum,
        limbs: [u64; 2],
    }

    let positive = BignumWithLimbs {
        bignum: HeapBignum {
            header: HeapBignum::make_header(2),
            sign: 0,
        },
        limbs: [0x1234_5678_9ABC_DEF0, 0xFEDC_BA98_7654_3210],
    };

    let negative = BignumWithLimbs {
        bignum: HeapBignum {
            header: HeapBignum::make_header(2),
            sign: 1,
        },
        limbs: [0x1234_5678_9ABC_DEF0, 0xFEDC_BA98_7654_3210],
    };

    assert!(!positive.bignum.is_negative());
    assert!(negative.bignum.is_negative());

    assert_eq!(positive.bignum.limb_count(), 2);

    unsafe {
        let limbs = positive.bignum.limbs();
        assert_eq!(limbs.len(), 2);
        assert_eq!(limbs[0], 0x1234_5678_9ABC_DEF0);
        assert_eq!(limbs[1], 0xFEDC_BA98_7654_3210);
    }
}

// ============================================================================
// Object Size Consistency Tests
// ============================================================================

#[test]
fn object_sizes_match_header_calculation() {
    // Verify that our alloc_size functions match the Header::object_size calculation

    // Tuple
    for len in [0, 1, 3, 10] {
        let header = HeapTuple::make_header(len);
        assert_eq!(
            header.object_size(),
            HeapTuple::alloc_size(len),
            "Tuple size mismatch for len={len}"
        );
    }

    // Vector
    for cap in [0, 1, 5, 10] {
        let header = HeapVector::make_header(cap);
        assert_eq!(
            header.object_size(),
            HeapVector::alloc_size(cap),
            "Vector size mismatch for capacity={cap}"
        );
    }

    // String
    for len in [0, 1, 8, 9, 16] {
        let header = HeapString::make_header(len);
        assert_eq!(
            header.object_size(),
            HeapString::alloc_size(len),
            "String size mismatch for len={len}"
        );
    }

    // Fixed-size types
    assert_eq!(HeapMap::make_header(0).object_size(), HeapMap::SIZE);
    assert_eq!(HeapFloat::make_header().object_size(), HeapFloat::SIZE);
    assert_eq!(HeapPid::make_header().object_size(), HeapPid::SIZE);
    assert_eq!(HeapRef::make_header().object_size(), HeapRef::SIZE);
    assert_eq!(HeapVar::make_header().object_size(), HeapVar::SIZE);

    // Closure
    for cap in [0, 1, 3] {
        let header = HeapClosure::make_header(cap);
        assert_eq!(
            header.object_size(),
            HeapClosure::alloc_size(cap),
            "Closure size mismatch for captures={cap}"
        );
    }

    // Bignum
    for limbs in [0, 1, 4] {
        let header = HeapBignum::make_header(limbs);
        assert_eq!(
            header.object_size(),
            HeapBignum::alloc_size(limbs),
            "Bignum size mismatch for limbs={limbs}"
        );
    }
}
