// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the Term type.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// ============================================================================
// Size and Layout Tests
// ============================================================================

#[test]
fn term_is_8_bytes() {
    assert_eq!(core::mem::size_of::<Term>(), 8);
}

#[test]
fn term_has_8_byte_alignment() {
    assert_eq!(core::mem::align_of::<Term>(), 8);
}

// ============================================================================
// Special Value Tests
// ============================================================================

#[test]
fn nil_is_correct() {
    assert!(Term::NIL.is_nil());
    assert!(Term::NIL.is_special());
    assert!(Term::NIL.is_immediate());
    assert!(!Term::NIL.is_truthy());
    assert_eq!(Term::NIL.type_name(), "nil");
}

#[test]
fn true_is_correct() {
    assert!(Term::TRUE.is_true());
    assert!(Term::TRUE.is_boolean());
    assert!(Term::TRUE.is_special());
    assert!(Term::TRUE.is_immediate());
    assert!(Term::TRUE.is_truthy());
    assert_eq!(Term::TRUE.as_bool(), Some(true));
    assert_eq!(Term::TRUE.type_name(), "boolean");
}

#[test]
fn false_is_correct() {
    assert!(Term::FALSE.is_false());
    assert!(Term::FALSE.is_boolean());
    assert!(Term::FALSE.is_special());
    assert!(Term::FALSE.is_immediate());
    assert!(!Term::FALSE.is_truthy());
    assert_eq!(Term::FALSE.as_bool(), Some(false));
    assert_eq!(Term::FALSE.type_name(), "boolean");
}

#[test]
fn unbound_is_correct() {
    assert!(Term::UNBOUND.is_unbound());
    assert!(Term::UNBOUND.is_special());
    assert!(Term::UNBOUND.is_immediate());
    assert!(Term::UNBOUND.is_truthy()); // Unbound is truthy (not nil or false)
    assert_eq!(Term::UNBOUND.type_name(), "unbound");
}

#[test]
fn bool_constructor() {
    assert_eq!(Term::bool(true), Term::TRUE);
    assert_eq!(Term::bool(false), Term::FALSE);
}

// ============================================================================
// Small Integer Tests
// ============================================================================

#[test]
fn small_int_zero() {
    let term = Term::small_int(0).unwrap();
    assert!(term.is_small_int());
    assert!(term.is_immediate());
    assert!(term.is_truthy());
    assert_eq!(term.as_small_int(), Some(0));
    assert_eq!(term.type_name(), "integer");
}

#[test]
fn small_int_positive() {
    let term = Term::small_int(42).unwrap();
    assert!(term.is_small_int());
    assert_eq!(term.as_small_int(), Some(42));
}

#[test]
fn small_int_negative() {
    let term = Term::small_int(-42).unwrap();
    assert!(term.is_small_int());
    assert_eq!(term.as_small_int(), Some(-42));
}

#[test]
fn small_int_one() {
    let term = Term::small_int(1).unwrap();
    assert_eq!(term.as_small_int(), Some(1));
}

#[test]
fn small_int_negative_one() {
    let term = Term::small_int(-1).unwrap();
    assert_eq!(term.as_small_int(), Some(-1));
}

#[test]
fn small_int_max_positive() {
    // Maximum positive value that fits in 60 bits signed: 2^59 - 1
    let max = (1i64 << 59) - 1;
    let term = Term::small_int(max).unwrap();
    assert_eq!(term.as_small_int(), Some(max));
}

#[test]
fn small_int_min_negative() {
    // Minimum negative value that fits in 60 bits signed: -2^59
    let min = -(1i64 << 59);
    let term = Term::small_int(min).unwrap();
    assert_eq!(term.as_small_int(), Some(min));
}

#[test]
fn small_int_overflow_positive() {
    // 2^59 is too large
    let overflow = 1i64 << 59;
    assert!(Term::small_int(overflow).is_none());
}

#[test]
fn small_int_overflow_negative() {
    // -2^59 - 1 is too small
    let overflow = -(1i64 << 59) - 1;
    assert!(Term::small_int(overflow).is_none());
}

#[test]
fn small_int_round_trip_various() {
    let test_values = [
        0,
        1,
        -1,
        42,
        -42,
        1000,
        -1000,
        i64::from(i32::MAX),
        i64::from(i32::MIN),
        (1i64 << 59) - 1,
        -(1i64 << 59),
    ];

    for &value in &test_values {
        let term = Term::small_int(value).unwrap();
        assert_eq!(
            term.as_small_int(),
            Some(value),
            "Round-trip failed for {value}"
        );
    }
}

// ============================================================================
// Symbol Tests
// ============================================================================

#[test]
fn symbol_zero_index() {
    let term = Term::symbol(0);
    assert!(term.is_symbol());
    assert!(term.is_immediate());
    assert!(term.is_truthy());
    assert_eq!(term.as_symbol_index(), Some(0));
    assert_eq!(term.type_name(), "symbol");
}

#[test]
fn symbol_nonzero_index() {
    let term = Term::symbol(42);
    assert!(term.is_symbol());
    assert_eq!(term.as_symbol_index(), Some(42));
}

#[test]
fn symbol_max_index() {
    let term = Term::symbol(u32::MAX);
    assert!(term.is_symbol());
    assert_eq!(term.as_symbol_index(), Some(u32::MAX));
}

#[test]
fn symbol_not_keyword() {
    let term = Term::symbol(42);
    assert!(!term.is_keyword());
    assert!(term.as_keyword_index().is_none());
}

// ============================================================================
// Keyword Tests
// ============================================================================

#[test]
fn keyword_zero_index() {
    let term = Term::keyword(0);
    assert!(term.is_keyword());
    assert!(term.is_immediate());
    assert!(term.is_truthy());
    assert_eq!(term.as_keyword_index(), Some(0));
    assert_eq!(term.type_name(), "keyword");
}

#[test]
fn keyword_nonzero_index() {
    let term = Term::keyword(42);
    assert!(term.is_keyword());
    assert_eq!(term.as_keyword_index(), Some(42));
}

#[test]
fn keyword_max_index() {
    let term = Term::keyword(u32::MAX);
    assert!(term.is_keyword());
    assert_eq!(term.as_keyword_index(), Some(u32::MAX));
}

#[test]
fn keyword_not_symbol() {
    let term = Term::keyword(42);
    assert!(!term.is_symbol());
    assert!(term.as_symbol_index().is_none());
}

// ============================================================================
// Primary Tag Tests
// ============================================================================

#[test]
fn primary_tag_immediate() {
    assert_eq!(Term::NIL.primary_tag(), tag::primary::IMMEDIATE);
    assert_eq!(Term::TRUE.primary_tag(), tag::primary::IMMEDIATE);
    assert_eq!(
        Term::small_int(42).unwrap().primary_tag(),
        tag::primary::IMMEDIATE
    );
    assert_eq!(Term::symbol(0).primary_tag(), tag::primary::IMMEDIATE);
    assert_eq!(Term::keyword(0).primary_tag(), tag::primary::IMMEDIATE);
}

// ============================================================================
// Pointer Masking Tests
// ============================================================================

#[test]
fn pointer_masking_preserves_alignment() {
    // Create a fake aligned pointer (8-byte aligned)
    let fake_ptr: u64 = 0x1000_0000_0000_0008;

    // Tag it as LIST
    // SAFETY: Test with known valid encoding
    let term = unsafe { Term::from_raw(fake_ptr | tag::primary::LIST) };

    // Extract pointer - should mask off low 2 bits
    let extracted = term.to_ptr() as u64;

    // The extracted pointer should be the original (which was already aligned)
    // but masked to ensure low bits are zero
    assert_eq!(extracted & 0b11, 0, "Extracted pointer not aligned");
    assert_eq!(extracted, fake_ptr, "Pointer not preserved");
}

#[test]
fn list_tag_detection() {
    let fake_ptr: u64 = 0x1000_0000_0000_0010;
    // SAFETY: Test with known valid encoding
    let term = unsafe { Term::from_raw(fake_ptr | tag::primary::LIST) };

    assert!(term.is_list());
    assert!(!term.is_boxed());
    assert!(!term.is_immediate());
    assert!(!term.is_header());
}

#[test]
fn boxed_tag_detection() {
    let fake_ptr: u64 = 0x1000_0000_0000_0010;
    // SAFETY: Test with known valid encoding
    let term = unsafe { Term::from_raw(fake_ptr | tag::primary::BOXED) };

    assert!(term.is_boxed());
    assert!(!term.is_list());
    assert!(!term.is_immediate());
    assert!(!term.is_header());
}

#[test]
fn header_tag_detection() {
    // Headers only have primary tag 00
    // SAFETY: Test with known valid encoding
    let term = unsafe { Term::from_raw(tag::primary::HEADER) };

    assert!(term.is_header());
    assert!(!term.is_list());
    assert!(!term.is_boxed());
    assert!(!term.is_immediate());
}

// ============================================================================
// Debug and Display Tests
// ============================================================================

#[test]
fn debug_format_nil() {
    let debug = format!("{:?}", Term::NIL);
    assert!(debug.contains("NIL"));
}

#[test]
fn debug_format_true() {
    let debug = format!("{:?}", Term::TRUE);
    assert!(debug.contains("TRUE"));
}

#[test]
fn debug_format_small_int() {
    let debug = format!("{:?}", Term::small_int(42).unwrap());
    assert!(debug.contains("42"));
}

#[test]
fn display_format_nil() {
    assert_eq!(format!("{}", Term::NIL), "nil");
}

#[test]
fn display_format_true() {
    assert_eq!(format!("{}", Term::TRUE), "true");
}

#[test]
fn display_format_false() {
    assert_eq!(format!("{}", Term::FALSE), "false");
}

#[test]
fn display_format_int() {
    assert_eq!(format!("{}", Term::small_int(42).unwrap()), "42");
    assert_eq!(format!("{}", Term::small_int(-7).unwrap()), "-7");
}

// ============================================================================
// Cross-Type Exclusivity Tests
// ============================================================================

#[test]
fn types_are_mutually_exclusive() {
    let terms = [
        Term::NIL,
        Term::TRUE,
        Term::FALSE,
        Term::UNBOUND,
        Term::small_int(42).unwrap(),
        Term::symbol(0),
        Term::keyword(0),
    ];

    for term in &terms {
        let mut type_count = 0;
        if term.is_nil() {
            type_count += 1;
        }
        if term.is_true() {
            type_count += 1;
        }
        if term.is_false() {
            type_count += 1;
        }
        if term.is_unbound() {
            type_count += 1;
        }
        if term.is_small_int() {
            type_count += 1;
        }
        if term.is_symbol() {
            type_count += 1;
        }
        if term.is_keyword() {
            type_count += 1;
        }

        assert_eq!(
            type_count, 1,
            "Term {term:?} matched {type_count} types instead of exactly 1"
        );
    }
}

// ============================================================================
// Default Tests
// ============================================================================

#[test]
fn default_is_nil() {
    // Term::default() should be NIL, not Term(0) which would be invalid (HEADER tag)
    let default = Term::default();
    assert!(default.is_nil());
    assert_eq!(default, Term::NIL);
    assert!(!default.is_header());
}
