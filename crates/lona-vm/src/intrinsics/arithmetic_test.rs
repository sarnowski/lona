// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for arithmetic and comparison intrinsics.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::similar_names)]

use super::*;
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::value::Value;

/// Create a test environment with process, memory, and realm.
pub(super) fn setup() -> (Process, MockVSpace, Realm) {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let proc = Process::new(1, young_base, young_size, old_base, old_size);
    let realm_base = base.add(128 * 1024);
    let realm = Realm::new(realm_base, 64 * 1024);
    (proc, mem, realm)
}

// --- Arithmetic tests ---

#[test]
fn add_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(3);

    call_intrinsic(id::ADD, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(5));
}

#[test]
fn add_negative() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(-10);
    proc.x_regs[2] = Value::int(7);

    call_intrinsic(id::ADD, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(-3));
}

#[test]
fn sub_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(10);
    proc.x_regs[2] = Value::int(3);

    call_intrinsic(id::SUB, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(7));
}

#[test]
fn mul_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(6);
    proc.x_regs[2] = Value::int(7);

    call_intrinsic(id::MUL, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(42));
}

#[test]
fn div_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(20);
    proc.x_regs[2] = Value::int(4);

    call_intrinsic(id::DIV, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(5));
}

#[test]
fn div_by_zero() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(10);
    proc.x_regs[2] = Value::int(0);

    let result = call_intrinsic(id::DIV, 2, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn mod_basic() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(17);
    proc.x_regs[2] = Value::int(5);

    call_intrinsic(id::MOD, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(2));
}

#[test]
fn mod_by_zero() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(10);
    proc.x_regs[2] = Value::int(0);

    let result = call_intrinsic(id::MOD, 2, &mut proc, &mut mem, &mut realm);
    assert_eq!(result, Err(IntrinsicError::DivisionByZero));
}

#[test]
fn mod_negative_dividend() {
    // Modulus: result has same sign as divisor
    // (-7) mod 3 = 2 (NOT -1 which is remainder)
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(-7);
    proc.x_regs[2] = Value::int(3);

    call_intrinsic(id::MOD, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(2));
}

#[test]
fn mod_negative_divisor() {
    // Modulus: result has same sign as divisor
    // 7 mod (-3) = -2 (NOT 1 which is remainder)
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(7);
    proc.x_regs[2] = Value::int(-3);

    call_intrinsic(id::MOD, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(-2));
}

#[test]
fn mod_both_negative() {
    // (-7) mod (-3) = -1 (sign follows divisor which is negative)
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::int(-7);
    proc.x_regs[2] = Value::int(-3);

    call_intrinsic(id::MOD, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(-1));
}

#[test]
fn arithmetic_type_error() {
    let (mut proc, mut mem, mut realm) = setup();
    proc.x_regs[1] = Value::bool(true); // Wrong type
    proc.x_regs[2] = Value::int(5);

    let result = call_intrinsic(id::ADD, 2, &mut proc, &mut mem, &mut realm);
    assert!(matches!(result, Err(IntrinsicError::TypeError { .. })));
}

// --- Comparison tests ---

#[test]
fn eq_integers() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(42);
    proc.x_regs[2] = Value::int(42);
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_strings() {
    let (mut proc, mut mem, mut realm) = setup();

    let s1 = proc.alloc_string(&mut mem, "hello").unwrap();
    let s2 = proc.alloc_string(&mut mem, "hello").unwrap();
    let s3 = proc.alloc_string(&mut mem, "world").unwrap();

    // Same content = equal
    proc.x_regs[1] = s1;
    proc.x_regs[2] = s2;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // Different content = not equal
    proc.x_regs[1] = s1;
    proc.x_regs[2] = s3;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_different_types() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::bool(true);
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn lt_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LT, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(1);
    call_intrinsic(id::LT, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LT, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn le_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(2);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(3);
    proc.x_regs[2] = Value::int(2);
    call_intrinsic(id::LE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn gt_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(5);
    proc.x_regs[2] = Value::int(3);
    call_intrinsic(id::GT, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn ge_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(5);
    proc.x_regs[2] = Value::int(5);
    call_intrinsic(id::GE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    proc.x_regs[1] = Value::int(5);
    proc.x_regs[2] = Value::int(6);
    call_intrinsic(id::GE, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

// --- Structural equality tests ---

#[test]
fn eq_tuples_structural() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create two tuples with same content
    let t1 = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2), Value::int(3)])
        .unwrap();
    let t2 = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2), Value::int(3)])
        .unwrap();
    let t3 = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2), Value::int(4)])
        .unwrap();

    // Same content = equal
    proc.x_regs[1] = t1;
    proc.x_regs[2] = t2;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // Different content = not equal
    proc.x_regs[1] = t1;
    proc.x_regs[2] = t3;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_tuples_different_length() {
    let (mut proc, mut mem, mut realm) = setup();

    let t1 = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2)])
        .unwrap();
    let t2 = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2), Value::int(3)])
        .unwrap();

    proc.x_regs[1] = t1;
    proc.x_regs[2] = t2;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_nested_tuples() {
    let (mut proc, mut mem, mut realm) = setup();

    // [[1 2] [3 4]] == [[1 2] [3 4]]
    let inner1a = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2)])
        .unwrap();
    let inner1b = proc
        .alloc_tuple(&mut mem, &[Value::int(3), Value::int(4)])
        .unwrap();
    let t1 = proc.alloc_tuple(&mut mem, &[inner1a, inner1b]).unwrap();

    let inner2a = proc
        .alloc_tuple(&mut mem, &[Value::int(1), Value::int(2)])
        .unwrap();
    let inner2b = proc
        .alloc_tuple(&mut mem, &[Value::int(3), Value::int(4)])
        .unwrap();
    let t2 = proc.alloc_tuple(&mut mem, &[inner2a, inner2b]).unwrap();

    proc.x_regs[1] = t1;
    proc.x_regs[2] = t2;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // [[1 2] [3 5]] - different inner element
    let inner3b = proc
        .alloc_tuple(&mut mem, &[Value::int(3), Value::int(5)])
        .unwrap();
    let t3 = proc.alloc_tuple(&mut mem, &[inner2a, inner3b]).unwrap();

    proc.x_regs[1] = t1;
    proc.x_regs[2] = t3;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_pairs_structural() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create two lists (1 2 3) with same content
    let p1c = proc
        .alloc_pair(&mut mem, Value::int(3), Value::Nil)
        .unwrap();
    let p1b = proc.alloc_pair(&mut mem, Value::int(2), p1c).unwrap();
    let p1a = proc.alloc_pair(&mut mem, Value::int(1), p1b).unwrap();

    let p2c = proc
        .alloc_pair(&mut mem, Value::int(3), Value::Nil)
        .unwrap();
    let p2b = proc.alloc_pair(&mut mem, Value::int(2), p2c).unwrap();
    let p2a = proc.alloc_pair(&mut mem, Value::int(1), p2b).unwrap();

    // Same content = equal
    proc.x_regs[1] = p1a;
    proc.x_regs[2] = p2a;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // (1 2 4) - different last element
    let p3c = proc
        .alloc_pair(&mut mem, Value::int(4), Value::Nil)
        .unwrap();
    let p3b = proc.alloc_pair(&mut mem, Value::int(2), p3c).unwrap();
    let p3a = proc.alloc_pair(&mut mem, Value::int(1), p3b).unwrap();

    proc.x_regs[1] = p1a;
    proc.x_regs[2] = p3a;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_maps_structural() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create %{:a 1 :b 2}
    let ka = proc.alloc_keyword(&mut mem, "a").unwrap();
    let kb = proc.alloc_keyword(&mut mem, "b").unwrap();

    let entry1a = proc.alloc_tuple(&mut mem, &[ka, Value::int(1)]).unwrap();
    let entry1b = proc.alloc_tuple(&mut mem, &[kb, Value::int(2)]).unwrap();
    let list1b = proc.alloc_pair(&mut mem, entry1b, Value::Nil).unwrap();
    let list1a = proc.alloc_pair(&mut mem, entry1a, list1b).unwrap();
    let m1 = proc.alloc_map(&mut mem, list1a).unwrap();

    // Create another %{:a 1 :b 2}
    let entry2a = proc.alloc_tuple(&mut mem, &[ka, Value::int(1)]).unwrap();
    let entry2b = proc.alloc_tuple(&mut mem, &[kb, Value::int(2)]).unwrap();
    let list2b = proc.alloc_pair(&mut mem, entry2b, Value::Nil).unwrap();
    let list2a = proc.alloc_pair(&mut mem, entry2a, list2b).unwrap();
    let m2 = proc.alloc_map(&mut mem, list2a).unwrap();

    // Same content = equal
    proc.x_regs[1] = m1;
    proc.x_regs[2] = m2;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // Create %{:a 1 :b 3} - different value
    let entry3b = proc.alloc_tuple(&mut mem, &[kb, Value::int(3)]).unwrap();
    let list3b = proc.alloc_pair(&mut mem, entry3b, Value::Nil).unwrap();
    let list3a = proc.alloc_pair(&mut mem, entry2a, list3b).unwrap();
    let m3 = proc.alloc_map(&mut mem, list3a).unwrap();

    proc.x_regs[1] = m1;
    proc.x_regs[2] = m3;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn eq_same_address_fast_path() {
    let (mut proc, mut mem, mut realm) = setup();

    // Same address = equal (fast path)
    let t = proc.alloc_tuple(&mut mem, &[Value::int(1)]).unwrap();

    proc.x_regs[1] = t;
    proc.x_regs[2] = t;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn eq_maps_with_nil_values() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create two maps %{:a nil}
    let ka = proc.alloc_keyword(&mut mem, "a").unwrap();

    let entry1 = proc.alloc_tuple(&mut mem, &[ka, Value::Nil]).unwrap();
    let list1 = proc.alloc_pair(&mut mem, entry1, Value::Nil).unwrap();
    let m1 = proc.alloc_map(&mut mem, list1).unwrap();

    let entry2 = proc.alloc_tuple(&mut mem, &[ka, Value::Nil]).unwrap();
    let list2 = proc.alloc_pair(&mut mem, entry2, Value::Nil).unwrap();
    let m2 = proc.alloc_map(&mut mem, list2).unwrap();

    // Maps with nil values should be equal
    proc.x_regs[1] = m1;
    proc.x_regs[2] = m2;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // Create %{:a 1} - should NOT equal %{:a nil}
    let entry3 = proc.alloc_tuple(&mut mem, &[ka, Value::int(1)]).unwrap();
    let list3 = proc.alloc_pair(&mut mem, entry3, Value::Nil).unwrap();
    let m3 = proc.alloc_map(&mut mem, list3).unwrap();

    proc.x_regs[1] = m1;
    proc.x_regs[2] = m3;
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

// --- Reference identity tests ---

#[test]
fn identical_same_address() {
    let (mut proc, mut mem, mut realm) = setup();

    // Same tuple address = identical
    let t = proc.alloc_tuple(&mut mem, &[Value::int(1)]).unwrap();

    proc.x_regs[1] = t;
    proc.x_regs[2] = t;
    call_intrinsic(id::IDENTICAL, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn identical_different_address_same_content() {
    let (mut proc, mut mem, mut realm) = setup();

    // Different addresses with same content = NOT identical
    let t1 = proc.alloc_tuple(&mut mem, &[Value::int(1)]).unwrap();
    let t2 = proc.alloc_tuple(&mut mem, &[Value::int(1)]).unwrap();

    proc.x_regs[1] = t1;
    proc.x_regs[2] = t2;
    call_intrinsic(id::IDENTICAL, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    // But they ARE equal (structural equality)
    call_intrinsic(id::EQ, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn identical_immediates() {
    let (mut proc, mut mem, mut realm) = setup();

    // Integers - same value = identical
    proc.x_regs[1] = Value::int(42);
    proc.x_regs[2] = Value::int(42);
    call_intrinsic(id::IDENTICAL, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // Integers - different value = not identical
    proc.x_regs[1] = Value::int(42);
    proc.x_regs[2] = Value::int(43);
    call_intrinsic(id::IDENTICAL, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    // Nil = identical to nil
    proc.x_regs[1] = Value::Nil;
    proc.x_regs[2] = Value::Nil;
    call_intrinsic(id::IDENTICAL, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));

    // Booleans
    proc.x_regs[1] = Value::bool(true);
    proc.x_regs[2] = Value::bool(true);
    call_intrinsic(id::IDENTICAL, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn identical_different_types() {
    let (mut proc, mut mem, mut realm) = setup();

    // Different types = not identical
    proc.x_regs[1] = Value::int(1);
    proc.x_regs[2] = Value::bool(true);
    call_intrinsic(id::IDENTICAL, 2, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}
