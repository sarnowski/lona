// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Ratio values.

use super::*;
use alloc::string::ToString;

#[test]
fn ratio_equality() {
    let r1 = Value::Ratio(Ratio::from_i64(1, 2));
    let r2 = Value::Ratio(Ratio::from_i64(1, 2));
    let r3 = Value::Ratio(Ratio::from_i64(1, 3));

    assert_eq!(r1, r2);
    assert_ne!(r1, r3);
}

#[test]
fn ratio_equality_normalized() {
    // 2/4 should equal 1/2 after normalization
    let r1 = Value::Ratio(Ratio::from_i64(2, 4));
    let r2 = Value::Ratio(Ratio::from_i64(1, 2));
    assert_eq!(r1, r2);
}

#[test]
fn display_ratio() {
    let ratio = Value::Ratio(Ratio::from_i64(1, 3));
    assert_eq!(ratio.to_string(), "1/3");
}

#[test]
fn display_ratio_integer() {
    // Ratio that equals an integer displays as integer
    let ratio = Value::Ratio(Ratio::from_i64(4, 2));
    assert_eq!(ratio.to_string(), "2");
}

#[test]
fn is_ratio() {
    assert!(Value::Ratio(Ratio::from_i64(1, 2)).is_ratio());
    assert!(!Value::Nil.is_ratio());
    assert!(!int(42).is_ratio());
}

#[test]
fn as_ratio() {
    let ratio = Ratio::from_i64(1, 2);
    let value = Value::Ratio(ratio.clone());
    assert_eq!(value.as_ratio(), Some(&ratio));
    assert_eq!(Value::Nil.as_ratio(), None);
}

#[test]
fn from_ratio() {
    let ratio = Ratio::from_i64(1, 2);
    let value = Value::from(ratio.clone());
    assert_eq!(value, Value::Ratio(ratio));
}

#[test]
fn ratio_is_truthy() {
    // All ratios are truthy, including zero
    assert!(Value::Ratio(Ratio::from_i64(0, 1)).is_truthy());
    assert!(Value::Ratio(Ratio::from_i64(1, 2)).is_truthy());
}

#[test]
fn ratio_not_equal_to_integer() {
    // Even though 2/1 = 2, they are different types
    let ratio = Value::Ratio(Ratio::from_i64(2, 1));
    let integer = int(2);
    assert_ne!(ratio, integer);
}

#[test]
fn display_ratio_with_interner() {
    let interner = Interner::new();
    let ratio = Value::Ratio(Ratio::from_i64(1, 3));
    assert_eq!(ratio.display(&interner).to_string(), "1/3");
}
