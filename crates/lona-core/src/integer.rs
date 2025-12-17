// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Hybrid arbitrary-precision integer type for Lonala.
//!
//! Provides a hybrid integer representation that uses `i64` for small values
//! (the common case) and automatically promotes to arbitrary precision when
//! overflow would occur.
//!
//! # Design
//!
//! Most integers in typical programs fit in 64 bits, so we optimize for that
//! case while still supporting arbitrary precision arithmetic when needed.
//! The transition is transparent to users.

extern crate alloc;

use alloc::boxed::Box;

use core::cmp::Ordering;
use core::fmt::{self, Display};
use core::hash::{Hash, Hasher};
use core::ops::{Add, Div, Mul, Neg, Rem, Sub};

use num_bigint::BigInt;
use num_traits::{One, Signed as _, ToPrimitive as _, Zero};

/// A hybrid integer that uses `i64` for small values and `BigInt` for overflow.
///
/// This type provides transparent automatic promotion: operations that would
/// overflow an `i64` automatically produce a `Big` variant instead.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Integer {
    /// Small integer that fits in 64 bits.
    Small(i64),
    /// Arbitrary precision integer for values outside `i64` range.
    Big(Box<BigInt>),
}

impl Integer {
    /// Creates an `Integer` from an `i64`.
    #[inline]
    #[must_use]
    pub const fn from_i64(value: i64) -> Self {
        Self::Small(value)
    }

    /// Creates an `Integer` from a `BigInt`.
    ///
    /// If the value fits in an `i64`, it will be stored as `Small`.
    #[inline]
    #[must_use]
    pub fn from_bigint(value: BigInt) -> Self {
        // Try to downgrade to i64 if it fits
        value
            .to_i64()
            .map_or_else(|| Self::Big(Box::new(value)), Self::Small)
    }

    /// Returns `true` if this integer is positive.
    #[inline]
    #[must_use]
    pub fn is_positive(&self) -> bool {
        match *self {
            Self::Small(val) => val > 0,
            Self::Big(ref big) => big.is_positive(),
        }
    }

    /// Returns `true` if this integer is negative.
    #[inline]
    #[must_use]
    pub fn is_negative(&self) -> bool {
        match *self {
            Self::Small(val) => val < 0,
            Self::Big(ref big) => big.is_negative(),
        }
    }

    /// Attempts to convert to `i64`, returning `None` if the value doesn't fit.
    #[inline]
    #[must_use]
    pub fn to_i64(&self) -> Option<i64> {
        match *self {
            Self::Small(val) => Some(val),
            Self::Big(ref big) => big.to_i64(),
        }
    }

    /// Converts to a `BigInt`.
    #[inline]
    #[must_use]
    pub fn to_bigint(&self) -> BigInt {
        match *self {
            Self::Small(val) => BigInt::from(val),
            Self::Big(ref big) => (**big).clone(),
        }
    }

    /// Returns the absolute value.
    #[inline]
    #[must_use]
    pub fn abs(&self) -> Self {
        match *self {
            Self::Small(val) => {
                // i64::MIN.abs() overflows, so handle it specially
                if val == i64::MIN {
                    // |i64::MIN| = i64::MAX + 1, which doesn't fit in i64
                    Self::Big(Box::new(BigInt::from(val).abs()))
                } else {
                    Self::Small(val.abs())
                }
            }
            Self::Big(ref big) => Self::from_bigint(big.abs()),
        }
    }

    /// Computes the greatest common divisor.
    #[inline]
    #[must_use]
    pub fn gcd(&self, other: &Self) -> Self {
        // Use num-integer's GCD implementation via BigInt
        let left_big = self.to_bigint();
        let right_big = other.to_bigint();
        Self::from_bigint(num_integer::Integer::gcd(&left_big, &right_big))
    }

    /// Performs checked division, returning `None` if divisor is zero.
    #[inline]
    #[must_use]
    pub fn checked_div(&self, other: &Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }

        if let (&Self::Small(left), &Self::Small(right)) = (self, other) {
            // Handle i64::MIN / -1 which overflows
            if left == i64::MIN && right == -1 {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "[approved] BigInt division is safe and cannot overflow"
                )]
                let result = BigInt::from(left) / BigInt::from(right);
                Some(Self::Big(Box::new(result)))
            } else {
                Some(Self::Small(left.checked_div(right)?))
            }
        } else {
            let left_big = self.to_bigint();
            let right_big = other.to_bigint();
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "[approved] BigInt division is safe and cannot overflow"
            )]
            let result = left_big / right_big;
            Some(Self::from_bigint(result))
        }
    }

    /// Performs checked remainder, returning `None` if divisor is zero.
    #[inline]
    #[must_use]
    pub fn checked_rem(&self, other: &Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }

        if let (&Self::Small(left), &Self::Small(right)) = (self, other) {
            // Handle i64::MIN % -1 which can overflow in some implementations
            if left == i64::MIN && right == -1 {
                Some(Self::Small(0))
            } else {
                Some(Self::Small(left.checked_rem(right)?))
            }
        } else {
            let left_big = self.to_bigint();
            let right_big = other.to_bigint();
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "[approved] BigInt remainder is safe and cannot overflow"
            )]
            let result = left_big % right_big;
            Some(Self::from_bigint(result))
        }
    }

    /// Helper to promote small operation results to Big when they overflow.
    fn add_small(left: i64, right: i64) -> Self {
        left.checked_add(right).map_or_else(
            || {
                // Overflow - promote to BigInt
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "[approved] BigInt addition is safe and cannot overflow"
                )]
                let result = BigInt::from(left) + BigInt::from(right);
                Self::from_bigint(result)
            },
            Self::Small,
        )
    }

    /// Helper for subtraction with overflow promotion.
    fn sub_small(left: i64, right: i64) -> Self {
        left.checked_sub(right).map_or_else(
            || {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "[approved] BigInt subtraction is safe and cannot overflow"
                )]
                let result = BigInt::from(left) - BigInt::from(right);
                Self::from_bigint(result)
            },
            Self::Small,
        )
    }

    /// Helper for multiplication with overflow promotion.
    fn mul_small(left: i64, right: i64) -> Self {
        left.checked_mul(right).map_or_else(
            || {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "[approved] BigInt multiplication is safe and cannot overflow"
                )]
                let result = BigInt::from(left) * BigInt::from(right);
                Self::from_bigint(result)
            },
            Self::Small,
        )
    }
}

impl Default for Integer {
    #[inline]
    fn default() -> Self {
        Self::Small(0)
    }
}

impl Display for Integer {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Small(val) => write!(f, "{val}"),
            Self::Big(ref big) => write!(f, "{big}"),
        }
    }
}

impl PartialEq for Integer {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&Self::Small(left), &Self::Small(right)) => left == right,
            (&Self::Big(ref left), &Self::Big(ref right)) => left == right,
            // Cross-variant comparison: convert to same representation
            (&Self::Small(left), &Self::Big(ref right)) => BigInt::from(left) == **right,
            (&Self::Big(ref left), &Self::Small(right)) => **left == BigInt::from(right),
        }
    }
}

impl Eq for Integer {}

impl PartialOrd for Integer {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Integer {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (&Self::Small(left), &Self::Small(right)) => left.cmp(&right),
            (&Self::Big(ref left), &Self::Big(ref right)) => left.cmp(right),
            // Cross-variant comparison
            (&Self::Small(left), &Self::Big(ref right)) => BigInt::from(left).cmp(right),
            (&Self::Big(ref left), &Self::Small(right)) => (**left).cmp(&BigInt::from(right)),
        }
    }
}

impl Hash for Integer {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // For consistent hashing, always convert to BigInt representation.
        // This ensures Small(42) and Big(42) hash the same, maintaining
        // the hash invariant: a == b implies hash(a) == hash(b).
        let big = match *self {
            Self::Small(val) => BigInt::from(val),
            Self::Big(ref big) => (**big).clone(),
        };
        let (sign, digits) = big.to_u64_digits();
        sign.hash(state);
        digits.hash(state);
    }
}

// =============================================================================
// Arithmetic Operations
// =============================================================================

impl Add for Integer {
    type Output = Self;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] BigInt arithmetic is safe and cannot overflow"
    )]
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Small(left), Self::Small(right)) => Self::add_small(left, right),
            (Self::Big(left), Self::Big(right)) => Self::from_bigint(*left + *right),
            (Self::Small(left), Self::Big(right)) => Self::from_bigint(BigInt::from(left) + *right),
            (Self::Big(left), Self::Small(right)) => Self::from_bigint(*left + BigInt::from(right)),
        }
    }
}

impl Add for &Integer {
    type Output = Integer;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] BigInt arithmetic is safe and cannot overflow"
    )]
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (&Integer::Small(left), &Integer::Small(right)) => Integer::add_small(left, right),
            (&Integer::Big(ref left), &Integer::Big(ref right)) => {
                Integer::from_bigint((**left).clone() + (**right).clone())
            }
            (&Integer::Small(left), &Integer::Big(ref right)) => {
                Integer::from_bigint(BigInt::from(left) + (**right).clone())
            }
            (&Integer::Big(ref left), &Integer::Small(right)) => {
                Integer::from_bigint((**left).clone() + BigInt::from(right))
            }
        }
    }
}

impl Sub for Integer {
    type Output = Self;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] BigInt arithmetic is safe and cannot overflow"
    )]
    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Small(left), Self::Small(right)) => Self::sub_small(left, right),
            (Self::Big(left), Self::Big(right)) => Self::from_bigint(*left - *right),
            (Self::Small(left), Self::Big(right)) => Self::from_bigint(BigInt::from(left) - *right),
            (Self::Big(left), Self::Small(right)) => Self::from_bigint(*left - BigInt::from(right)),
        }
    }
}

impl Sub for &Integer {
    type Output = Integer;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] BigInt arithmetic is safe and cannot overflow"
    )]
    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (&Integer::Small(left), &Integer::Small(right)) => Integer::sub_small(left, right),
            (&Integer::Big(ref left), &Integer::Big(ref right)) => {
                Integer::from_bigint((**left).clone() - (**right).clone())
            }
            (&Integer::Small(left), &Integer::Big(ref right)) => {
                Integer::from_bigint(BigInt::from(left) - (**right).clone())
            }
            (&Integer::Big(ref left), &Integer::Small(right)) => {
                Integer::from_bigint((**left).clone() - BigInt::from(right))
            }
        }
    }
}

impl Mul for Integer {
    type Output = Self;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] BigInt arithmetic is safe and cannot overflow"
    )]
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Small(left), Self::Small(right)) => Self::mul_small(left, right),
            (Self::Big(left), Self::Big(right)) => Self::from_bigint(*left * *right),
            (Self::Small(left), Self::Big(right)) => Self::from_bigint(BigInt::from(left) * *right),
            (Self::Big(left), Self::Small(right)) => Self::from_bigint(*left * BigInt::from(right)),
        }
    }
}

impl Mul for &Integer {
    type Output = Integer;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] BigInt arithmetic is safe and cannot overflow"
    )]
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (&Integer::Small(left), &Integer::Small(right)) => Integer::mul_small(left, right),
            (&Integer::Big(ref left), &Integer::Big(ref right)) => {
                Integer::from_bigint((**left).clone() * (**right).clone())
            }
            (&Integer::Small(left), &Integer::Big(ref right)) => {
                Integer::from_bigint(BigInt::from(left) * (**right).clone())
            }
            (&Integer::Big(ref left), &Integer::Small(right)) => {
                Integer::from_bigint((**left).clone() * BigInt::from(right))
            }
        }
    }
}

impl Div for Integer {
    type Output = Self;

    /// Performs integer division. Panics if divisor is zero.
    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "[approved] Div trait panic on zero is standard Rust behavior; use checked_div for non-panicking"
    )]
    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(&rhs).expect("division by zero")
    }
}

impl Div for &Integer {
    type Output = Integer;

    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "[approved] Div trait panic on zero is standard Rust behavior; use checked_div for non-panicking"
    )]
    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).expect("division by zero")
    }
}

impl Rem for Integer {
    type Output = Self;

    /// Performs remainder operation. Panics if divisor is zero.
    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "[approved] Rem trait panic on zero is standard Rust behavior; use checked_rem for non-panicking"
    )]
    fn rem(self, rhs: Self) -> Self::Output {
        self.checked_rem(&rhs).expect("division by zero")
    }
}

impl Rem for &Integer {
    type Output = Integer;

    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "[approved] Rem trait panic on zero is standard Rust behavior; use checked_rem for non-panicking"
    )]
    fn rem(self, rhs: Self) -> Self::Output {
        self.checked_rem(rhs).expect("division by zero")
    }
}

impl Neg for Integer {
    type Output = Self;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] BigInt negation is safe and cannot overflow"
    )]
    fn neg(self) -> Self::Output {
        match self {
            Self::Small(val) => {
                // i64::MIN negated doesn't fit in i64
                if val == i64::MIN {
                    Self::Big(Box::new(-BigInt::from(val)))
                } else {
                    Self::Small(-val)
                }
            }
            Self::Big(big) => Self::from_bigint(-*big),
        }
    }
}

impl Neg for &Integer {
    type Output = Integer;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] BigInt negation is safe and cannot overflow"
    )]
    fn neg(self) -> Self::Output {
        match *self {
            Integer::Small(val) => {
                if val == i64::MIN {
                    Integer::Big(Box::new(-BigInt::from(val)))
                } else {
                    Integer::Small(-val)
                }
            }
            Integer::Big(ref big) => Integer::from_bigint(-(**big).clone()),
        }
    }
}

// =============================================================================
// Conversions
// =============================================================================

impl From<i64> for Integer {
    #[inline]
    fn from(value: i64) -> Self {
        Self::Small(value)
    }
}

impl From<i32> for Integer {
    #[inline]
    fn from(value: i32) -> Self {
        Self::Small(i64::from(value))
    }
}

impl From<u32> for Integer {
    #[inline]
    fn from(value: u32) -> Self {
        Self::Small(i64::from(value))
    }
}

impl From<BigInt> for Integer {
    #[inline]
    fn from(value: BigInt) -> Self {
        Self::from_bigint(value)
    }
}

impl Zero for Integer {
    #[inline]
    fn zero() -> Self {
        Self::Small(0)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        match *self {
            Self::Small(val) => val == 0,
            Self::Big(ref big) => big.is_zero(),
        }
    }
}

impl One for Integer {
    #[inline]
    fn one() -> Self {
        Self::Small(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    // =========================================================================
    // Construction Tests
    // =========================================================================

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

    // =========================================================================
    // Display Tests
    // =========================================================================

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

    // =========================================================================
    // Arithmetic Tests
    // =========================================================================

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

    // =========================================================================
    // Comparison Tests
    // =========================================================================

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

    // =========================================================================
    // Utility Method Tests
    // =========================================================================

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

    // =========================================================================
    // Big Integer Operation Tests
    // =========================================================================

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

    // =========================================================================
    // Reference Operation Tests
    // =========================================================================

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

    // =========================================================================
    // Zero and One Tests
    // =========================================================================

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

    // =========================================================================
    // Default Test
    // =========================================================================

    #[test]
    fn default_is_zero() {
        let def: Integer = Integer::default();
        assert!(def.is_zero());
    }
}
