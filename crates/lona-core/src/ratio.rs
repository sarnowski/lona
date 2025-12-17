// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Rational number type for Lonala.
//!
//! Provides exact rational arithmetic using the hybrid [`Integer`] type for
//! both numerator and denominator. Ratios are automatically normalized
//! (reduced to lowest terms) on construction.
//!
//! # Design
//!
//! Ratios maintain the invariant that:
//! - The denominator is always positive
//! - The numerator and denominator are coprime (GCD = 1)
//! - Zero is represented as 0/1

extern crate alloc;

use core::cmp::Ordering;
use core::fmt::{self, Display};
use core::hash::{Hash, Hasher};
use core::ops::{Add, Div, Mul, Neg, Sub};

use num_traits::{ToPrimitive as _, Zero as _};

use crate::integer::Integer;

/// A rational number (fraction) with arbitrary precision.
///
/// Ratios are always stored in normalized form with a positive denominator.
/// This ensures consistent equality and hashing.
#[derive(Debug, Clone)]
pub struct Ratio {
    /// The numerator (can be negative, zero, or positive).
    numer: Integer,
    /// The denominator (always positive, never zero).
    denom: Integer,
}

impl Ratio {
    /// Creates a new ratio, normalizing it automatically.
    ///
    /// # Panics
    ///
    /// Panics if `denom` is zero.
    #[inline]
    #[must_use]
    pub fn new(numer: &Integer, denom: &Integer) -> Self {
        assert!(!denom.is_zero(), "ratio denominator cannot be zero");
        Self::normalize(numer, denom)
    }

    /// Creates a ratio from an integer (denominator = 1).
    #[inline]
    #[must_use]
    pub const fn from_integer(numer: Integer) -> Self {
        Self {
            numer,
            denom: Integer::Small(1),
        }
    }

    /// Creates a ratio from two `i64` values.
    ///
    /// # Panics
    ///
    /// Panics if `denom` is zero.
    #[inline]
    #[must_use]
    pub fn from_i64(numer: i64, denom: i64) -> Self {
        Self::new(&Integer::from_i64(numer), &Integer::from_i64(denom))
    }

    /// Returns the numerator.
    #[inline]
    #[must_use]
    pub const fn numer(&self) -> &Integer {
        &self.numer
    }

    /// Returns the denominator.
    #[inline]
    #[must_use]
    pub const fn denom(&self) -> &Integer {
        &self.denom
    }

    /// Returns `true` if this ratio represents an integer (denominator = 1).
    #[inline]
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.denom == Integer::from_i64(1)
    }

    /// Converts to an integer if the denominator is 1.
    #[inline]
    #[must_use]
    pub fn to_integer(&self) -> Option<Integer> {
        self.is_integer().then(|| self.numer.clone())
    }

    /// Returns `true` if this ratio is zero.
    #[inline]
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.numer.is_zero()
    }

    /// Returns `true` if this ratio is positive.
    #[inline]
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.numer.is_positive()
    }

    /// Returns `true` if this ratio is negative.
    #[inline]
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.numer.is_negative()
    }

    /// Returns the absolute value.
    #[inline]
    #[must_use]
    pub fn abs(&self) -> Self {
        Self {
            numer: self.numer.abs(),
            denom: self.denom.clone(),
        }
    }

    /// Returns the reciprocal (1/self).
    ///
    /// # Panics
    ///
    /// Panics if self is zero.
    #[inline]
    #[must_use]
    pub fn recip(&self) -> Self {
        assert!(!self.is_zero(), "reciprocal of zero");
        Self::normalize(&self.denom, &self.numer)
    }

    /// Normalizes a ratio to lowest terms with positive denominator.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn normalize(numer: &Integer, denom: &Integer) -> Self {
        if numer.is_zero() {
            return Self {
                numer: Integer::from_i64(0),
                denom: Integer::from_i64(1),
            };
        }

        let gcd = numer.gcd(denom);

        // Divide both by GCD
        let normalized_numer = numer.checked_div(&gcd).unwrap_or_default();
        let normalized_denom = denom
            .checked_div(&gcd)
            .unwrap_or_else(|| Integer::from_i64(1));

        // Ensure denominator is positive
        if normalized_denom.is_negative() {
            Self {
                numer: -normalized_numer,
                denom: -normalized_denom,
            }
        } else {
            Self {
                numer: normalized_numer,
                denom: normalized_denom,
            }
        }
    }

    /// Converts to f64 (may lose precision).
    ///
    /// Returns `None` if the values are too large to convert to f64.
    #[inline]
    #[must_use]
    #[expect(
        clippy::float_arithmetic,
        reason = "[approved] Float division is needed for ratio to f64 conversion"
    )]
    pub fn to_f64(&self) -> Option<f64> {
        // Use BigInt's ToPrimitive implementation for the conversion
        let numer_big = self.numer.to_bigint();
        let denom_big = self.denom.to_bigint();
        let numer_f64 = numer_big.to_f64()?;
        let denom_f64 = denom_big.to_f64()?;
        Some(numer_f64 / denom_f64)
    }
}

impl Default for Ratio {
    #[inline]
    fn default() -> Self {
        Self::from_integer(Integer::from_i64(0))
    }
}

impl Display for Ratio {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_integer() {
            write!(f, "{}", self.numer)
        } else {
            write!(f, "{}/{}", self.numer, self.denom)
        }
    }
}

impl PartialEq for Ratio {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Since ratios are normalized, we can compare directly
        self.numer == other.numer && self.denom == other.denom
    }
}

impl Eq for Ratio {}

impl PartialOrd for Ratio {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ratio {
    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare a/b vs c/d by comparing a*d vs c*b
        let left_product = &self.numer * &other.denom;
        let right_product = &other.numer * &self.denom;
        left_product.cmp(&right_product)
    }
}

impl Hash for Ratio {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Since ratios are normalized, we can hash components directly
        self.numer.hash(state);
        self.denom.hash(state);
    }
}

// =============================================================================
// Arithmetic Operations
// =============================================================================

impl Add for Ratio {
    type Output = Self;

    /// Adds two ratios: a/b + c/d = (a*d + c*b) / (b*d)
    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn add(self, rhs: Self) -> Self::Output {
        let numer = self.numer * rhs.denom.clone() + rhs.numer * self.denom.clone();
        let denom = self.denom * rhs.denom;
        Self::normalize(&numer, &denom)
    }
}

impl Add for &Ratio {
    type Output = Ratio;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn add(self, rhs: Self) -> Self::Output {
        let numer = &self.numer * &rhs.denom + &rhs.numer * &self.denom;
        let denom = &self.denom * &rhs.denom;
        Ratio::normalize(&numer, &denom)
    }
}

impl Sub for Ratio {
    type Output = Self;

    /// Subtracts two ratios: a/b - c/d = (a*d - c*b) / (b*d)
    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn sub(self, rhs: Self) -> Self::Output {
        let numer = self.numer * rhs.denom.clone() - rhs.numer * self.denom.clone();
        let denom = self.denom * rhs.denom;
        Self::normalize(&numer, &denom)
    }
}

impl Sub for &Ratio {
    type Output = Ratio;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn sub(self, rhs: Self) -> Self::Output {
        let numer = &self.numer * &rhs.denom - &rhs.numer * &self.denom;
        let denom = &self.denom * &rhs.denom;
        Ratio::normalize(&numer, &denom)
    }
}

impl Mul for Ratio {
    type Output = Self;

    /// Multiplies two ratios: (a/b) * (c/d) = (a*c) / (b*d)
    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn mul(self, rhs: Self) -> Self::Output {
        let numer = self.numer * rhs.numer;
        let denom = self.denom * rhs.denom;
        Self::normalize(&numer, &denom)
    }
}

impl Mul for &Ratio {
    type Output = Ratio;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn mul(self, rhs: Self) -> Self::Output {
        let numer = &self.numer * &rhs.numer;
        let denom = &self.denom * &rhs.denom;
        Ratio::normalize(&numer, &denom)
    }
}

impl Div for Ratio {
    type Output = Self;

    /// Divides two ratios: (a/b) / (c/d) = (a*d) / (b*c)
    ///
    /// # Panics
    ///
    /// Panics if rhs is zero.
    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn div(self, rhs: Self) -> Self::Output {
        assert!(!rhs.is_zero(), "division by zero");
        let numer = self.numer * rhs.denom;
        let denom = self.denom * rhs.numer;
        Self::normalize(&numer, &denom)
    }
}

impl Div for &Ratio {
    type Output = Ratio;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer arithmetic is safe with automatic BigInt promotion"
    )]
    fn div(self, rhs: Self) -> Self::Output {
        assert!(!rhs.is_zero(), "division by zero");
        let numer = &self.numer * &rhs.denom;
        let denom = &self.denom * &rhs.numer;
        Ratio::normalize(&numer, &denom)
    }
}

impl Neg for Ratio {
    type Output = Self;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer negation is safe with automatic BigInt promotion"
    )]
    fn neg(self) -> Self::Output {
        Self {
            numer: -self.numer,
            denom: self.denom,
        }
    }
}

impl Neg for &Ratio {
    type Output = Ratio;

    #[inline]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer negation is safe with automatic BigInt promotion"
    )]
    fn neg(self) -> Self::Output {
        Ratio {
            numer: -&self.numer,
            denom: self.denom.clone(),
        }
    }
}

// =============================================================================
// Conversions
// =============================================================================

impl From<Integer> for Ratio {
    #[inline]
    fn from(value: Integer) -> Self {
        Self::from_integer(value)
    }
}

impl From<i64> for Ratio {
    #[inline]
    fn from(value: i64) -> Self {
        Self::from_integer(Integer::from_i64(value))
    }
}

impl From<i32> for Ratio {
    #[inline]
    fn from(value: i32) -> Self {
        Self::from_integer(Integer::from(value))
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

    // =========================================================================
    // Display Tests
    // =========================================================================

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

    // =========================================================================
    // Arithmetic Tests
    // =========================================================================

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

    // =========================================================================
    // Comparison Tests
    // =========================================================================

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

    // =========================================================================
    // Utility Method Tests
    // =========================================================================

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

    // =========================================================================
    // Reference Operation Tests
    // =========================================================================

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

    // =========================================================================
    // Conversion Tests
    // =========================================================================

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

    // =========================================================================
    // Default Test
    // =========================================================================

    #[test]
    fn default_is_zero() {
        let ratio: Ratio = Ratio::default();
        assert!(ratio.is_zero());
        assert_eq!(ratio, Ratio::from_integer(Integer::from_i64(0)));
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

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
}
