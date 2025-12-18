// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Integer type.

use alloc::boxed::Box;
use alloc::string::ToString;

use num_bigint::BigInt;
use num_traits::{One, Zero};

use super::Integer;

// =============================================================================
// Construction Tests
// =============================================================================

#[test]
fn from_i64_creates_small() {
    let int = Integer::from_i64(42);
    assert!(matches!(int, Integer::Small(42)));
}

#[test]
fn from_bigint_downgrades_when_possible() {
    let big = BigInt::from(42_i64);
    let int = Integer::from_bigint(big);
    assert!(matches!(int, Integer::Small(42)));
}

#[test]
fn from_bigint_stays_big_when_needed() {
    // Create a value larger than i64::MAX
    let big = BigInt::from(i64::MAX) + BigInt::from(1_i64);
    let int = Integer::from_bigint(big);
    assert!(matches!(int, Integer::Big(_)));
}

// =============================================================================
// Display Tests
// =============================================================================

#[test]
fn display_small() {
    let int = Integer::from_i64(42);
    assert_eq!(int.to_string(), "42");
}

#[test]
fn display_negative() {
    let int = Integer::from_i64(-123);
    assert_eq!(int.to_string(), "-123");
}

#[test]
fn display_big() {
    let big = BigInt::from(i64::MAX) + BigInt::from(1_i64);
    let int = Integer::from_bigint(big);
    assert_eq!(int.to_string(), "9223372036854775808");
}

// =============================================================================
// Arithmetic Tests
// =============================================================================

#[test]
fn add_small_no_overflow() {
    let result = Integer::from_i64(1) + Integer::from_i64(2);
    assert_eq!(result, Integer::from_i64(3));
}

#[test]
fn add_small_with_overflow() {
    let result = Integer::from_i64(i64::MAX) + Integer::from_i64(1);
    // Should be Big since it overflows
    assert!(matches!(result, Integer::Big(_)));
    // Verify the value is correct
    let expected = BigInt::from(i64::MAX) + BigInt::from(1_i64);
    assert_eq!(result, Integer::from_bigint(expected));
}

#[test]
fn sub_small_no_overflow() {
    let result = Integer::from_i64(5) - Integer::from_i64(3);
    assert_eq!(result, Integer::from_i64(2));
}

#[test]
fn sub_small_with_overflow() {
    let result = Integer::from_i64(i64::MIN) - Integer::from_i64(1);
    assert!(matches!(result, Integer::Big(_)));
}

#[test]
fn mul_small_no_overflow() {
    let result = Integer::from_i64(6) * Integer::from_i64(7);
    assert_eq!(result, Integer::from_i64(42));
}

#[test]
fn mul_small_with_overflow() {
    let result = Integer::from_i64(i64::MAX) * Integer::from_i64(2);
    assert!(matches!(result, Integer::Big(_)));
}

#[test]
fn div_small() {
    let result = Integer::from_i64(10) / Integer::from_i64(3);
    assert_eq!(result, Integer::from_i64(3));
}

#[test]
fn div_min_by_neg_one() {
    // This case overflows in i64 (MIN / -1 = MAX + 1)
    let result = Integer::from_i64(i64::MIN) / Integer::from_i64(-1);
    assert!(matches!(result, Integer::Big(_)));
}

#[test]
fn checked_div_by_zero_returns_none() {
    let result = Integer::from_i64(42).checked_div(&Integer::from_i64(0));
    assert!(result.is_none());
}

#[test]
fn rem_small() {
    let result = Integer::from_i64(10) % Integer::from_i64(3);
    assert_eq!(result, Integer::from_i64(1));
}

#[test]
fn checked_rem_by_zero_returns_none() {
    let result = Integer::from_i64(42).checked_rem(&Integer::from_i64(0));
    assert!(result.is_none());
}

#[test]
fn neg_small() {
    let result = -Integer::from_i64(42);
    assert_eq!(result, Integer::from_i64(-42));
}

#[test]
fn neg_min_overflows_to_big() {
    let result = -Integer::from_i64(i64::MIN);
    assert!(matches!(result, Integer::Big(_)));
    // Verify the value
    let expected = BigInt::from(i64::MAX) + BigInt::from(1_i64);
    assert_eq!(result, Integer::from_bigint(expected));
}

// =============================================================================
// Comparison Tests
// =============================================================================

#[test]
fn equality_small() {
    assert_eq!(Integer::from_i64(42), Integer::from_i64(42));
    assert_ne!(Integer::from_i64(42), Integer::from_i64(43));
}

#[test]
fn equality_cross_variant() {
    // Create a Big that equals a representable i64
    let big = Integer::Big(Box::new(BigInt::from(42_i64)));
    let small = Integer::from_i64(42);
    assert_eq!(big, small);
    assert_eq!(small, big);
}

#[test]
fn ordering_small() {
    assert!(Integer::from_i64(1) < Integer::from_i64(2));
    assert!(Integer::from_i64(2) > Integer::from_i64(1));
}

#[test]
fn ordering_cross_variant() {
    let big = Integer::Big(Box::new(BigInt::from(i64::MAX) + BigInt::from(1_i64)));
    let small = Integer::from_i64(i64::MAX);
    assert!(big > small);
    assert!(small < big);
}

// =============================================================================
// Utility Method Tests
// =============================================================================

#[test]
fn is_zero_small() {
    assert!(Integer::from_i64(0).is_zero());
    assert!(!Integer::from_i64(1).is_zero());
}

#[test]
fn is_positive_negative() {
    assert!(Integer::from_i64(42).is_positive());
    assert!(!Integer::from_i64(0).is_positive());
    assert!(!Integer::from_i64(-42).is_positive());

    assert!(Integer::from_i64(-42).is_negative());
    assert!(!Integer::from_i64(0).is_negative());
    assert!(!Integer::from_i64(42).is_negative());
}

#[test]
fn to_i64_small() {
    assert_eq!(Integer::from_i64(42).to_i64(), Some(42));
}

#[test]
fn to_i64_big_that_fits() {
    let big = Integer::Big(Box::new(BigInt::from(42_i64)));
    assert_eq!(big.to_i64(), Some(42));
}

#[test]
fn to_i64_big_that_doesnt_fit() {
    let big = Integer::from_bigint(BigInt::from(i64::MAX) + BigInt::from(1_i64));
    assert_eq!(big.to_i64(), None);
}

#[test]
fn abs_positive() {
    assert_eq!(Integer::from_i64(42).abs(), Integer::from_i64(42));
}

#[test]
fn abs_negative() {
    assert_eq!(Integer::from_i64(-42).abs(), Integer::from_i64(42));
}

#[test]
fn abs_min_overflows_to_big() {
    let result = Integer::from_i64(i64::MIN).abs();
    assert!(matches!(result, Integer::Big(_)));
}

#[test]
fn gcd_small() {
    let result = Integer::from_i64(12).gcd(&Integer::from_i64(8));
    assert_eq!(result, Integer::from_i64(4));
}

// =============================================================================
// Big Integer Operation Tests
// =============================================================================

#[test]
fn big_times_big() {
    let big_left = Integer::from_bigint(BigInt::from(i64::MAX) + BigInt::from(1_i64));
    let big_right = Integer::from_bigint(BigInt::from(2_i64));
    let result = big_left * big_right;
    let expected =
        Integer::from_bigint((BigInt::from(i64::MAX) + BigInt::from(1_i64)) * BigInt::from(2));
    assert_eq!(result, expected);
}

#[test]
fn mixed_small_and_big() {
    let small = Integer::from_i64(1_000_000_000);
    let big = Integer::from_bigint(BigInt::from(1_000_000_000_000_000_i64));
    let result = small * big;
    let expected = Integer::from_bigint(
        BigInt::from(1_000_000_000_i64) * BigInt::from(1_000_000_000_000_000_i64),
    );
    assert_eq!(result, expected);
}

// =============================================================================
// Reference Operation Tests
// =============================================================================

#[test]
fn add_by_reference() {
    let left = Integer::from_i64(1);
    let right = Integer::from_i64(2);
    let result = &left + &right;
    assert_eq!(result, Integer::from_i64(3));
    // Original values should still be usable
    assert_eq!(left, Integer::from_i64(1));
    assert_eq!(right, Integer::from_i64(2));
}

#[test]
fn sub_by_reference() {
    let left = Integer::from_i64(5);
    let right = Integer::from_i64(3);
    let result = &left - &right;
    assert_eq!(result, Integer::from_i64(2));
}

#[test]
fn mul_by_reference() {
    let left = Integer::from_i64(6);
    let right = Integer::from_i64(7);
    let result = &left * &right;
    assert_eq!(result, Integer::from_i64(42));
}

#[test]
fn div_by_reference() {
    let left = Integer::from_i64(10);
    let right = Integer::from_i64(3);
    let result = &left / &right;
    assert_eq!(result, Integer::from_i64(3));
}

#[test]
fn rem_by_reference() {
    let left = Integer::from_i64(10);
    let right = Integer::from_i64(3);
    let result = &left % &right;
    assert_eq!(result, Integer::from_i64(1));
}

#[test]
fn neg_by_reference() {
    let int = Integer::from_i64(42);
    let result = -&int;
    assert_eq!(result, Integer::from_i64(-42));
    // Original should still be usable
    assert_eq!(int, Integer::from_i64(42));
}

// =============================================================================
// Zero and One Tests
// =============================================================================

#[test]
fn zero_trait() {
    let zero: Integer = Zero::zero();
    assert!(zero.is_zero());
    assert_eq!(zero, Integer::from_i64(0));
}

#[test]
fn one_trait() {
    let one: Integer = One::one();
    assert_eq!(one, Integer::from_i64(1));
}

// =============================================================================
// Default Test
// =============================================================================

#[test]
fn default_is_zero() {
    let def: Integer = Integer::default();
    assert!(def.is_zero());
}
