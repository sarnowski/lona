// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Physical and virtual address types.
//!
//! These newtypes prevent accidentally mixing address types at compile time.

use core::fmt;
use core::ops::{Add, Sub};

/// A physical memory address (hardware/DMA visible).
///
/// Physical addresses are what the hardware sees. They're used for:
/// - DMA buffer addresses
/// - Page table entries
/// - Device MMIO regions
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Paddr(u64);

impl Paddr {
    /// Create a new physical address.
    #[inline]
    #[must_use]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Create a null (zero) physical address.
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        Self(0)
    }

    /// Check if this is a null address.
    #[inline]
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Get the raw address value.
    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Add an offset to this address.
    #[inline]
    #[must_use]
    pub const fn add(self, offset: u64) -> Self {
        Self(self.0.wrapping_add(offset))
    }

    /// Subtract an offset from this address.
    #[inline]
    #[must_use]
    pub const fn sub(self, offset: u64) -> Self {
        Self(self.0.wrapping_sub(offset))
    }

    /// Calculate the difference between two addresses.
    #[inline]
    #[must_use]
    pub const fn diff(self, other: Self) -> u64 {
        self.0.wrapping_sub(other.0)
    }

    /// Align this address up to the given alignment.
    ///
    /// Returns `None` if alignment is zero or not a power of two.
    #[inline]
    #[must_use]
    pub const fn align_up(self, alignment: u64) -> Option<Self> {
        if !alignment.is_power_of_two() {
            return None;
        }
        let mask = alignment - 1;
        Some(Self((self.0.wrapping_add(mask)) & !mask))
    }

    /// Align this address down to the given alignment.
    ///
    /// Returns `None` if alignment is zero or not a power of two.
    #[inline]
    #[must_use]
    pub const fn align_down(self, alignment: u64) -> Option<Self> {
        if !alignment.is_power_of_two() {
            return None;
        }
        let mask = alignment - 1;
        Some(Self(self.0 & !mask))
    }

    /// Check if this address is aligned to the given alignment.
    ///
    /// Returns `None` if alignment is zero or not a power of two.
    #[inline]
    #[must_use]
    pub const fn is_aligned(self, alignment: u64) -> Option<bool> {
        if !alignment.is_power_of_two() {
            return None;
        }
        Some((self.0 & (alignment - 1)) == 0)
    }
}

impl fmt::Debug for Paddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Paddr({:#x})", self.0)
    }
}

impl fmt::Display for Paddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl From<u64> for Paddr {
    fn from(addr: u64) -> Self {
        Self(addr)
    }
}

impl Add<u64> for Paddr {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        self.add(rhs)
    }
}

impl Sub<u64> for Paddr {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        self.sub(rhs)
    }
}

/// A virtual memory address (CPU visible).
///
/// Virtual addresses are what the CPU sees after MMU translation. They're used for:
/// - Process heap and stack pointers
/// - Code addresses
/// - All normal memory access
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Vaddr(u64);

impl Vaddr {
    /// Create a new virtual address.
    #[inline]
    #[must_use]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Create a null (zero) virtual address.
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        Self(0)
    }

    /// Check if this is a null address.
    #[inline]
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Get the raw address value.
    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Convert to a raw pointer (for use in unsafe code).
    #[inline]
    #[must_use]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Convert to a raw mutable pointer (for use in unsafe code).
    #[inline]
    #[must_use]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// Add an offset to this address.
    #[inline]
    #[must_use]
    pub const fn add(self, offset: u64) -> Self {
        Self(self.0.wrapping_add(offset))
    }

    /// Subtract an offset from this address.
    #[inline]
    #[must_use]
    pub const fn sub(self, offset: u64) -> Self {
        Self(self.0.wrapping_sub(offset))
    }

    /// Calculate the difference between two addresses.
    #[inline]
    #[must_use]
    pub const fn diff(self, other: Self) -> u64 {
        self.0.wrapping_sub(other.0)
    }

    /// Align this address up to the given alignment.
    ///
    /// Returns `None` if alignment is zero or not a power of two.
    #[inline]
    #[must_use]
    pub const fn align_up(self, alignment: u64) -> Option<Self> {
        if !alignment.is_power_of_two() {
            return None;
        }
        let mask = alignment - 1;
        Some(Self((self.0.wrapping_add(mask)) & !mask))
    }

    /// Align this address down to the given alignment.
    ///
    /// Returns `None` if alignment is zero or not a power of two.
    #[inline]
    #[must_use]
    pub const fn align_down(self, alignment: u64) -> Option<Self> {
        if !alignment.is_power_of_two() {
            return None;
        }
        let mask = alignment - 1;
        Some(Self(self.0 & !mask))
    }

    /// Check if this address is aligned to the given alignment.
    ///
    /// Returns `None` if alignment is zero or not a power of two.
    #[inline]
    #[must_use]
    pub const fn is_aligned(self, alignment: u64) -> Option<bool> {
        if !alignment.is_power_of_two() {
            return None;
        }
        Some((self.0 & (alignment - 1)) == 0)
    }
}

impl fmt::Debug for Vaddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vaddr({:#x})", self.0)
    }
}

impl fmt::Display for Vaddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl From<u64> for Vaddr {
    fn from(addr: u64) -> Self {
        Self(addr)
    }
}

impl Add<u64> for Vaddr {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        self.add(rhs)
    }
}

impl Sub<u64> for Vaddr {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        self.sub(rhs)
    }
}
