// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the ratio implementation.

use alloc::string::ToString;

use crate::integer::Integer;

use super::Ratio;

// =============================================================================
// Construction Tests
// =============================================================================

#[test]
fn new_normalizes() {
    let ratio = Ratio::new(&Integer::from_i64(2), &Integer::from_i64(4));
    assert_eq!(ratio.numer(), &Integer::from_i64(1));
    assert_eq!(ratio.denom(), &Integer::from_i64(2));
}

#[test]
fn new_makes_denom_positive() {
    let ratio = Ratio::new(&Integer::from_i64(1), &Integer::from_i64(-2));
    assert_eq!(ratio.numer(), &Integer::from_i64(-1));
    assert_eq!(ratio.denom(), &Integer::from_i64(2));
}

#[test]
fn new_double_negative() {
    let ratio = Ratio::new(&Integer::from_i64(-1), &Integer::from_i64(-2));
    assert_eq!(ratio.numer(), &Integer::from_i64(1));
    assert_eq!(ratio.denom(), &Integer::from_i64(2));
}

#[test]
#[should_panic(expected = "ratio denominator cannot be zero")]
fn new_panics_on_zero_denom() {
    let _ratio = Ratio::new(&Integer::from_i64(1), &Integer::from_i64(0));
}

#[test]
fn from_integer() {
    let ratio = Ratio::from_integer(Integer::from_i64(42));
    assert_eq!(ratio.numer(), &Integer::from_i64(42));
    assert_eq!(ratio.denom(), &Integer::from_i64(1));
    assert!(ratio.is_integer());
}

#[test]
fn from_i64_pair() {
    let ratio = Ratio::from_i64(3, 4);
    assert_eq!(ratio.numer(), &Integer::from_i64(3));
    assert_eq!(ratio.denom(), &Integer::from_i64(4));
}

// =============================================================================
// Display Tests
// =============================================================================

#[test]
fn display_integer_ratio() {
    let ratio = Ratio::from_integer(Integer::from_i64(42));
    assert_eq!(ratio.to_string(), "42");
}

#[test]
fn display_fraction() {
    let ratio = Ratio::from_i64(1, 3);
    assert_eq!(ratio.to_string(), "1/3");
}

#[test]
fn display_negative_fraction() {
    let ratio = Ratio::from_i64(-1, 3);
    assert_eq!(ratio.to_string(), "-1/3");
}

// =============================================================================
// Arithmetic Tests
// =============================================================================

#[test]
fn add_same_denom() {
    let left = Ratio::from_i64(1, 3);
    let right = Ratio::from_i64(1, 3);
    let result = left + right;
    assert_eq!(result, Ratio::from_i64(2, 3));
}

#[test]
fn add_different_denom() {
    let left = Ratio::from_i64(1, 2);
    let right = Ratio::from_i64(1, 3);
    // 1/2 + 1/3 = 3/6 + 2/6 = 5/6
    let result = left + right;
    assert_eq!(result, Ratio::from_i64(5, 6));
}

#[test]
fn add_normalizes() {
    let left = Ratio::from_i64(1, 4);
    let right = Ratio::from_i64(1, 4);
    // 1/4 + 1/4 = 2/4 = 1/2
    let result = left + right;
    assert_eq!(result, Ratio::from_i64(1, 2));
}

#[test]
fn sub_basic() {
    let left = Ratio::from_i64(3, 4);
    let right = Ratio::from_i64(1, 4);
    let result = left - right;
    assert_eq!(result, Ratio::from_i64(1, 2));
}

#[test]
fn sub_different_denom() {
    let left = Ratio::from_i64(1, 2);
    let right = Ratio::from_i64(1, 3);
    // 1/2 - 1/3 = 3/6 - 2/6 = 1/6
    let result = left - right;
    assert_eq!(result, Ratio::from_i64(1, 6));
}

#[test]
fn mul_basic() {
    let left = Ratio::from_i64(2, 3);
    let right = Ratio::from_i64(3, 4);
    // (2/3) * (3/4) = 6/12 = 1/2
    let result = left * right;
    assert_eq!(result, Ratio::from_i64(1, 2));
}

#[test]
fn div_basic() {
    let left = Ratio::from_i64(1, 2);
    let right = Ratio::from_i64(3, 4);
    // (1/2) / (3/4) = (1/2) * (4/3) = 4/6 = 2/3
    let result = left / right;
    assert_eq!(result, Ratio::from_i64(2, 3));
}

#[test]
#[should_panic(expected = "division by zero")]
fn div_by_zero_panics() {
    let left = Ratio::from_i64(1, 2);
    let right = Ratio::from_integer(Integer::from_i64(0));
    let _result = left / right;
}

#[test]
fn neg_basic() {
    let ratio = Ratio::from_i64(1, 2);
    let result = -ratio;
    assert_eq!(result, Ratio::from_i64(-1, 2));
}

#[test]
fn neg_negative() {
    let ratio = Ratio::from_i64(-1, 2);
    let result = -ratio;
    assert_eq!(result, Ratio::from_i64(1, 2));
}

// =============================================================================
// Comparison Tests
// =============================================================================

#[test]
fn equality() {
    assert_eq!(Ratio::from_i64(1, 2), Ratio::from_i64(1, 2));
    assert_eq!(Ratio::from_i64(2, 4), Ratio::from_i64(1, 2)); // Normalized
    assert_ne!(Ratio::from_i64(1, 2), Ratio::from_i64(1, 3));
}

#[test]
fn ordering() {
    assert!(Ratio::from_i64(1, 3) < Ratio::from_i64(1, 2));
    assert!(Ratio::from_i64(1, 2) < Ratio::from_i64(2, 3));
    assert!(Ratio::from_i64(-1, 2) < Ratio::from_i64(1, 2));
}

// =============================================================================
// Utility Method Tests
// =============================================================================

#[test]
fn is_integer_true() {
    let ratio = Ratio::from_integer(Integer::from_i64(42));
    assert!(ratio.is_integer());
}

#[test]
fn is_integer_false() {
    let ratio = Ratio::from_i64(1, 2);
    assert!(!ratio.is_integer());
}

#[test]
fn to_integer_some() {
    let ratio = Ratio::from_integer(Integer::from_i64(42));
    assert_eq!(ratio.to_integer(), Some(Integer::from_i64(42)));
}

#[test]
fn to_integer_none() {
    let ratio = Ratio::from_i64(1, 2);
    assert_eq!(ratio.to_integer(), None);
}

#[test]
fn is_zero_true() {
    let ratio = Ratio::from_integer(Integer::from_i64(0));
    assert!(ratio.is_zero());
}

#[test]
fn is_zero_false() {
    let ratio = Ratio::from_i64(1, 2);
    assert!(!ratio.is_zero());
}

#[test]
fn is_positive_negative() {
    let pos = Ratio::from_i64(1, 2);
    let neg = Ratio::from_i64(-1, 2);
    let zero = Ratio::from_integer(Integer::from_i64(0));

    assert!(pos.is_positive());
    assert!(!neg.is_positive());
    assert!(!zero.is_positive());

    assert!(!pos.is_negative());
    assert!(neg.is_negative());
    assert!(!zero.is_negative());
}

#[test]
fn abs_positive() {
    let ratio = Ratio::from_i64(1, 2);
    assert_eq!(ratio.abs(), Ratio::from_i64(1, 2));
}

#[test]
fn abs_negative() {
    let ratio = Ratio::from_i64(-1, 2);
    assert_eq!(ratio.abs(), Ratio::from_i64(1, 2));
}

#[test]
fn recip_basic() {
    let ratio = Ratio::from_i64(2, 3);
    let result = ratio.recip();
    assert_eq!(result, Ratio::from_i64(3, 2));
}

#[test]
fn recip_negative() {
    let ratio = Ratio::from_i64(-2, 3);
    let result = ratio.recip();
    // -2/3 -> 3/-2 -> -3/2
    assert_eq!(result, Ratio::from_i64(-3, 2));
}

#[test]
#[should_panic(expected = "reciprocal of zero")]
fn recip_zero_panics() {
    let ratio = Ratio::from_integer(Integer::from_i64(0));
    let _result = ratio.recip();
}

#[test]
fn to_f64_basic() {
    let ratio = Ratio::from_i64(1, 2);
    assert_eq!(ratio.to_f64(), Some(0.5));
}

#[test]
fn to_f64_integer() {
    let ratio = Ratio::from_integer(Integer::from_i64(3));
    assert_eq!(ratio.to_f64(), Some(3.0));
}

// =============================================================================
// Reference Operation Tests
// =============================================================================

#[test]
fn add_by_reference() {
    let left = Ratio::from_i64(1, 3);
    let right = Ratio::from_i64(1, 3);
    let result = &left + &right;
    assert_eq!(result, Ratio::from_i64(2, 3));
    // Originals still usable
    assert_eq!(left, Ratio::from_i64(1, 3));
}

#[test]
fn sub_by_reference() {
    let left = Ratio::from_i64(2, 3);
    let right = Ratio::from_i64(1, 3);
    let result = &left - &right;
    assert_eq!(result, Ratio::from_i64(1, 3));
}

#[test]
fn mul_by_reference() {
    let left = Ratio::from_i64(2, 3);
    let right = Ratio::from_i64(3, 4);
    let result = &left * &right;
    assert_eq!(result, Ratio::from_i64(1, 2));
}

#[test]
fn div_by_reference() {
    let left = Ratio::from_i64(1, 2);
    let right = Ratio::from_i64(3, 4);
    let result = &left / &right;
    assert_eq!(result, Ratio::from_i64(2, 3));
}

#[test]
fn neg_by_reference() {
    let ratio = Ratio::from_i64(1, 2);
    let result = -&ratio;
    assert_eq!(result, Ratio::from_i64(-1, 2));
    // Original still usable
    assert_eq!(ratio, Ratio::from_i64(1, 2));
}

// =============================================================================
// Conversion Tests
// =============================================================================

#[test]
fn from_integer_trait() {
    let ratio: Ratio = Integer::from_i64(5).into();
    assert_eq!(ratio, Ratio::from_integer(Integer::from_i64(5)));
}

#[test]
fn from_i64_trait() {
    let ratio: Ratio = 42_i64.into();
    assert_eq!(ratio, Ratio::from_integer(Integer::from_i64(42)));
}

#[test]
fn from_i32_trait() {
    let ratio: Ratio = 42_i32.into();
    assert_eq!(ratio, Ratio::from_integer(Integer::from_i64(42)));
}

// =============================================================================
// Default Test
// =============================================================================

#[test]
fn default_is_zero() {
    let ratio: Ratio = Ratio::default();
    assert!(ratio.is_zero());
    assert_eq!(ratio, Ratio::from_integer(Integer::from_i64(0)));
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn zero_normalized() {
    // 0/5 should normalize to 0/1
    let ratio = Ratio::new(&Integer::from_i64(0), &Integer::from_i64(5));
    assert_eq!(ratio.numer(), &Integer::from_i64(0));
    assert_eq!(ratio.denom(), &Integer::from_i64(1));
}

#[test]
fn large_gcd_normalization() {
    // 100/200 = 1/2
    let ratio = Ratio::new(&Integer::from_i64(100), &Integer::from_i64(200));
    assert_eq!(ratio.numer(), &Integer::from_i64(1));
    assert_eq!(ratio.denom(), &Integer::from_i64(2));
}
