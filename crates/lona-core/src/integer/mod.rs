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

use num_bigint::BigInt;
use num_traits::{One, Signed as _, ToPrimitive as _, Zero};

mod ops;

#[cfg(test)]
mod tests;

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
    pub(crate) fn add_small(left: i64, right: i64) -> Self {
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
    pub(crate) fn sub_small(left: i64, right: i64) -> Self {
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
    pub(crate) fn mul_small(left: i64, right: i64) -> Self {
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
