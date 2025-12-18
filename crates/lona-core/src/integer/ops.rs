// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Arithmetic trait implementations for Integer.

use alloc::boxed::Box;

use core::ops::{Add, Div, Mul, Neg, Rem, Sub};

use num_bigint::BigInt;

use super::Integer;

// =============================================================================
// Addition
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

// =============================================================================
// Subtraction
// =============================================================================

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

// =============================================================================
// Multiplication
// =============================================================================

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

// =============================================================================
// Division
// =============================================================================

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

// =============================================================================
// Remainder
// =============================================================================

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

// =============================================================================
// Negation
// =============================================================================

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
