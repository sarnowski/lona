// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for arithmetic and comparison intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::similar_names)]

use super::*;
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::term::Term;

/// Create a test environment with `x_regs`, process, memory, and realm.
pub(super) fn setup() -> (XRegs, Process, MockVSpace, Realm) {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let proc = Process::new(young_base, young_size, old_base, old_size);
    let realm_base = base.add(128 * 1024);
    let realm = Realm::new(realm_base, 64 * 1024);
    let x_regs = [Term::NIL; X_REG_COUNT];
    (x_regs, proc, mem, realm)
}

/// Helper to create a small integer Term, panicking if out of range.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

// --- Arithmetic tests ---

#[test]
fn add_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(2);
    x_regs[2] = int(3);

    call_intrinsic(id::ADD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(5));
}

#[test]
fn add_negative() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(-10);
    x_regs[2] = int(7);

    call_intrinsic(id::ADD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(-3));
}

#[test]
fn sub_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(10);
    x_regs[2] = int(3);

    call_intrinsic(id::SUB, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(7));
}

#[test]
fn mul_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(6);
    x_regs[2] = int(7);

    call_intrinsic(id::MUL, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(42));
}

#[test]
fn div_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(20);
    x_regs[2] = int(4);

    call_intrinsic(id::DIV, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(5));
}

#[test]
fn div_by_zero() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(10);
    x_regs[2] = int(0);

    let result = call_intrinsic(id::DIV, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn mod_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(17);
    x_regs[2] = int(5);

    call_intrinsic(id::MOD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(2));
}

#[test]
fn mod_by_zero() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(10);
    x_regs[2] = int(0);

    let result = call_intrinsic(id::MOD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn mod_negative_dividend() {
    // Modulus: result has same sign as divisor
    // (-7) mod 3 = 2 (NOT -1 which is remainder)
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(-7);
    x_regs[2] = int(3);

    call_intrinsic(id::MOD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(2));
}

#[test]
fn mod_negative_divisor() {
    // Modulus: result has same sign as divisor
    // 7 mod (-3) = -2 (NOT 1 which is remainder)
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(7);
    x_regs[2] = int(-3);

    call_intrinsic(id::MOD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(-2));
}

#[test]
fn mod_both_negative() {
    // (-7) mod (-3) = -1 (sign follows divisor which is negative)
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = int(-7);
    x_regs[2] = int(-3);

    call_intrinsic(id::MOD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(-1));
}

#[test]
fn arithmetic_type_error() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();
    x_regs[1] = Term::TRUE; // Wrong type
    x_regs[2] = int(5);

    let result = call_intrinsic(id::ADD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert!(matches!(result, Err(IntrinsicError::TypeError { .. })));
}

// --- Overflow tests ---

#[test]
fn add_overflow_positive() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Two large positive numbers whose sum exceeds small_int max (2^59 - 1)
    // (1 << 58) + (1 << 58) = 2^59, which doesn't fit in 60-bit signed range
    let large = 1i64 << 58;
    x_regs[1] = int(large);
    x_regs[2] = int(large);

    let result = call_intrinsic(id::ADD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::Overflow));
}

#[test]
fn add_overflow_negative() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Two large negative numbers whose sum is less than small_int min (-2^59)
    // -(1 << 58) + -(1 << 58) - 1 = -2^59 - 1, which doesn't fit
    // We use the min small_int value and add -1 to it
    let min_small_int = -(1i64 << 59);
    x_regs[1] = int(min_small_int);
    x_regs[2] = int(-1);

    let result = call_intrinsic(id::ADD, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::Overflow));
}

#[test]
fn sub_overflow() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Subtracting a positive from min_small_int causes underflow
    // min_small_int - 1 = -2^59 - 1, which doesn't fit
    let min_small_int = -(1i64 << 59);
    x_regs[1] = int(min_small_int);
    x_regs[2] = int(1);

    let result = call_intrinsic(id::SUB, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::Overflow));
}

#[test]
fn mul_overflow() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Multiplying two moderately large numbers
    let large = 1i64 << 40;
    x_regs[1] = int(large);
    x_regs[2] = int(large);

    let result = call_intrinsic(id::MUL, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::Overflow));
}

#[test]
fn div_no_overflow_normally() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Normal division doesn't overflow
    x_regs[1] = int(1000);
    x_regs[2] = int(10);

    call_intrinsic(id::DIV, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], int(100));
}

/// Division can overflow in one edge case: `MIN / -1 = MAX + 1` (out of range).
/// For small integers, `MIN_SMALL_INT / -1` would exceed `MAX_SMALL_INT`.
#[test]
fn div_overflow_min_by_minus_one() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // MIN_SMALL_INT is approximately -2^59
    // MIN_SMALL_INT / -1 = 2^59, which exceeds MAX_SMALL_INT (2^59 - 1)
    let min_small_int = -(1i64 << 59);
    x_regs[1] = int(min_small_int);
    x_regs[2] = int(-1);

    let result = call_intrinsic(id::DIV, 2, &mut x_regs, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::Overflow));
}

// --- Comparison tests ---

#[test]
fn eq_integers() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(42);
    x_regs[2] = int(42);
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    x_regs[1] = int(1);
    x_regs[2] = int(2);
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn eq_strings() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let s1 = proc.alloc_term_string(&mut mem, "hello").unwrap();
    let s2 = proc.alloc_term_string(&mut mem, "hello").unwrap();
    let s3 = proc.alloc_term_string(&mut mem, "world").unwrap();

    // Same content = equal
    x_regs[1] = s1;
    x_regs[2] = s2;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    // Different content = not equal
    x_regs[1] = s1;
    x_regs[2] = s3;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn eq_different_types() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(1);
    x_regs[2] = Term::TRUE;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn lt_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(1);
    x_regs[2] = int(2);
    call_intrinsic(id::LT, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    x_regs[1] = int(2);
    x_regs[2] = int(1);
    call_intrinsic(id::LT, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);

    x_regs[1] = int(2);
    x_regs[2] = int(2);
    call_intrinsic(id::LT, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn le_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(1);
    x_regs[2] = int(2);
    call_intrinsic(id::LE, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    x_regs[1] = int(2);
    x_regs[2] = int(2);
    call_intrinsic(id::LE, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    x_regs[1] = int(3);
    x_regs[2] = int(2);
    call_intrinsic(id::LE, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn gt_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(5);
    x_regs[2] = int(3);
    call_intrinsic(id::GT, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);
}

#[test]
fn ge_basic() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    x_regs[1] = int(5);
    x_regs[2] = int(5);
    call_intrinsic(id::GE, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    x_regs[1] = int(5);
    x_regs[2] = int(6);
    call_intrinsic(id::GE, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

// --- Structural equality tests ---

#[test]
fn eq_tuples_structural() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Create two tuples with same content
    let t1 = proc
        .alloc_term_tuple(&mut mem, &[int(1), int(2), int(3)])
        .unwrap();
    let t2 = proc
        .alloc_term_tuple(&mut mem, &[int(1), int(2), int(3)])
        .unwrap();
    let t3 = proc
        .alloc_term_tuple(&mut mem, &[int(1), int(2), int(4)])
        .unwrap();

    // Same content = equal
    x_regs[1] = t1;
    x_regs[2] = t2;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    // Different content = not equal
    x_regs[1] = t1;
    x_regs[2] = t3;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn eq_tuples_different_length() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    let t1 = proc.alloc_term_tuple(&mut mem, &[int(1), int(2)]).unwrap();
    let t2 = proc
        .alloc_term_tuple(&mut mem, &[int(1), int(2), int(3)])
        .unwrap();

    x_regs[1] = t1;
    x_regs[2] = t2;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn eq_nested_tuples() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // [[1 2] [3 4]] == [[1 2] [3 4]]
    let inner1a = proc.alloc_term_tuple(&mut mem, &[int(1), int(2)]).unwrap();
    let inner1b = proc.alloc_term_tuple(&mut mem, &[int(3), int(4)]).unwrap();
    let t1 = proc
        .alloc_term_tuple(&mut mem, &[inner1a, inner1b])
        .unwrap();

    let inner2a = proc.alloc_term_tuple(&mut mem, &[int(1), int(2)]).unwrap();
    let inner2b = proc.alloc_term_tuple(&mut mem, &[int(3), int(4)]).unwrap();
    let t2 = proc
        .alloc_term_tuple(&mut mem, &[inner2a, inner2b])
        .unwrap();

    x_regs[1] = t1;
    x_regs[2] = t2;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    // [[1 2] [3 5]] - different inner element
    let inner3b = proc.alloc_term_tuple(&mut mem, &[int(3), int(5)]).unwrap();
    let t3 = proc
        .alloc_term_tuple(&mut mem, &[inner2a, inner3b])
        .unwrap();

    x_regs[1] = t1;
    x_regs[2] = t3;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn eq_pairs_structural() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Create two lists (1 2 3) with same content
    let p1c = proc.alloc_term_pair(&mut mem, int(3), Term::NIL).unwrap();
    let p1b = proc.alloc_term_pair(&mut mem, int(2), p1c).unwrap();
    let p1a = proc.alloc_term_pair(&mut mem, int(1), p1b).unwrap();

    let p2c = proc.alloc_term_pair(&mut mem, int(3), Term::NIL).unwrap();
    let p2b = proc.alloc_term_pair(&mut mem, int(2), p2c).unwrap();
    let p2a = proc.alloc_term_pair(&mut mem, int(1), p2b).unwrap();

    // Same content = equal
    x_regs[1] = p1a;
    x_regs[2] = p2a;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    // (1 2 4) - different last element
    let p3c = proc.alloc_term_pair(&mut mem, int(4), Term::NIL).unwrap();
    let p3b = proc.alloc_term_pair(&mut mem, int(2), p3c).unwrap();
    let p3a = proc.alloc_term_pair(&mut mem, int(1), p3b).unwrap();

    x_regs[1] = p1a;
    x_regs[2] = p3a;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn eq_maps_structural() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Create %{:a 1 :b 2} - using strings as keys during transition
    // (keywords aren't fully implemented in Term yet)
    let ka = proc.alloc_term_string(&mut mem, "a").unwrap();
    let kb = proc.alloc_term_string(&mut mem, "b").unwrap();

    let entry1a = proc.alloc_term_tuple(&mut mem, &[ka, int(1)]).unwrap();
    let entry1b = proc.alloc_term_tuple(&mut mem, &[kb, int(2)]).unwrap();
    let list1b = proc.alloc_term_pair(&mut mem, entry1b, Term::NIL).unwrap();
    let list1a = proc.alloc_term_pair(&mut mem, entry1a, list1b).unwrap();
    let m1 = proc.alloc_term_map(&mut mem, list1a, 2).unwrap();

    // Create another %{:a 1 :b 2}
    let ka2 = proc.alloc_term_string(&mut mem, "a").unwrap();
    let kb2 = proc.alloc_term_string(&mut mem, "b").unwrap();
    let entry2a = proc.alloc_term_tuple(&mut mem, &[ka2, int(1)]).unwrap();
    let entry2b = proc.alloc_term_tuple(&mut mem, &[kb2, int(2)]).unwrap();
    let list2b = proc.alloc_term_pair(&mut mem, entry2b, Term::NIL).unwrap();
    let list2a = proc.alloc_term_pair(&mut mem, entry2a, list2b).unwrap();
    let m2 = proc.alloc_term_map(&mut mem, list2a, 2).unwrap();

    // Same content = equal
    x_regs[1] = m1;
    x_regs[2] = m2;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    // Create %{:a 1 :b 3} - different value
    let entry3b = proc.alloc_term_tuple(&mut mem, &[kb2, int(3)]).unwrap();
    let list3b = proc.alloc_term_pair(&mut mem, entry3b, Term::NIL).unwrap();
    let list3a = proc.alloc_term_pair(&mut mem, entry2a, list3b).unwrap();
    let m3 = proc.alloc_term_map(&mut mem, list3a, 2).unwrap();

    x_regs[1] = m1;
    x_regs[2] = m3;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

#[test]
fn eq_same_address_fast_path() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Same address = equal (fast path)
    let t = proc.alloc_term_tuple(&mut mem, &[int(1)]).unwrap();

    x_regs[1] = t;
    x_regs[2] = t;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);
}

#[test]
fn eq_maps_with_nil_values() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Create two maps %{:a nil} - using strings as keys
    let ka = proc.alloc_term_string(&mut mem, "a").unwrap();

    let entry1 = proc.alloc_term_tuple(&mut mem, &[ka, Term::NIL]).unwrap();
    let list1 = proc.alloc_term_pair(&mut mem, entry1, Term::NIL).unwrap();
    let m1 = proc.alloc_term_map(&mut mem, list1, 1).unwrap();

    let ka2 = proc.alloc_term_string(&mut mem, "a").unwrap();
    let entry2 = proc.alloc_term_tuple(&mut mem, &[ka2, Term::NIL]).unwrap();
    let list2 = proc.alloc_term_pair(&mut mem, entry2, Term::NIL).unwrap();
    let m2 = proc.alloc_term_map(&mut mem, list2, 1).unwrap();

    // Maps with nil values should be equal
    x_regs[1] = m1;
    x_regs[2] = m2;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    // Create %{:a 1} - should NOT equal %{:a nil}
    let entry3 = proc.alloc_term_tuple(&mut mem, &[ka2, int(1)]).unwrap();
    let list3 = proc.alloc_term_pair(&mut mem, entry3, Term::NIL).unwrap();
    let m3 = proc.alloc_term_map(&mut mem, list3, 1).unwrap();

    x_regs[1] = m1;
    x_regs[2] = m3;
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}

// --- Reference identity tests ---

#[test]
fn identical_same_address() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Same tuple address = identical
    let t = proc.alloc_term_tuple(&mut mem, &[int(1)]).unwrap();

    x_regs[1] = t;
    x_regs[2] = t;
    call_intrinsic(
        id::IDENTICAL,
        2,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::TRUE);
}

#[test]
fn identical_different_address_same_content() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Different addresses with same content = NOT identical
    let t1 = proc.alloc_term_tuple(&mut mem, &[int(1)]).unwrap();
    let t2 = proc.alloc_term_tuple(&mut mem, &[int(1)]).unwrap();

    x_regs[1] = t1;
    x_regs[2] = t2;
    call_intrinsic(
        id::IDENTICAL,
        2,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::FALSE);

    // But they ARE equal (structural equality)
    call_intrinsic(id::EQ, 2, &mut x_regs, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(x_regs[0], Term::TRUE);
}

#[test]
fn identical_immediates() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Integers - same value = identical
    x_regs[1] = int(42);
    x_regs[2] = int(42);
    call_intrinsic(
        id::IDENTICAL,
        2,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    // Integers - different value = not identical
    x_regs[1] = int(42);
    x_regs[2] = int(43);
    call_intrinsic(
        id::IDENTICAL,
        2,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::FALSE);

    // Nil = identical to nil
    x_regs[1] = Term::NIL;
    x_regs[2] = Term::NIL;
    call_intrinsic(
        id::IDENTICAL,
        2,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::TRUE);

    // Booleans
    x_regs[1] = Term::TRUE;
    x_regs[2] = Term::TRUE;
    call_intrinsic(
        id::IDENTICAL,
        2,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::TRUE);
}

#[test]
fn identical_different_types() {
    let (mut x_regs, mut proc, mut mem, mut realm) = setup();

    // Different types = not identical
    x_regs[1] = int(1);
    x_regs[2] = Term::TRUE;
    call_intrinsic(
        id::IDENTICAL,
        2,
        &mut x_regs,
        &mut proc,
        &mut mem,
        &mut realm,
    )
    .unwrap();
    assert_eq!(x_regs[0], Term::FALSE);
}
