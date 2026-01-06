// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Core type definitions for realm, process, capability identifiers, and addresses.
//!
//! These newtypes prevent accidentally mixing different ID types at compile time.

use core::convert::From;
use core::default::Default;
use core::fmt;
use core::ops::{Add, Sub};
use core::option::Option::{self, None, Some};

// ============================================================================
// Address Types
// ============================================================================

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

// ============================================================================
// ID Types
// ============================================================================

/// Unique identifier for a realm.
///
/// Realms are the primary security boundary in Lona. Each realm has its own
/// `VSpace`, `CSpace`, and CPU budget. Realm IDs are assigned by the Lona Memory
/// Manager when a realm is created.
///
/// The init realm always has ID 1. ID 0 is reserved/invalid.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct RealmId(u64);

impl RealmId {
    /// The invalid/null realm ID.
    pub const NULL: Self = Self(0);

    /// The init realm ID (first user realm).
    pub const INIT: Self = Self(1);

    /// Creates a new realm ID.
    #[inline]
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw ID value.
    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Checks if this is the null/invalid realm ID.
    #[inline]
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for RealmId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RealmId({})", self.0)
    }
}

impl fmt::Display for RealmId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "realm:{}", self.0)
    }
}

/// Unique identifier for a process within a realm.
///
/// Processes are lightweight execution units (like Erlang processes). They are
/// managed entirely in userspace by the Lona VM - no kernel objects are created.
///
/// Process IDs are unique within a realm but may be reused after process exit.
/// ID 0 is reserved/invalid.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct ProcessId(u64);

impl ProcessId {
    /// The invalid/null process ID.
    pub const NULL: Self = Self(0);

    /// The init process ID (first process in a realm).
    pub const INIT: Self = Self(1);

    /// Creates a new process ID.
    #[inline]
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw ID value.
    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Checks if this is the null/invalid process ID.
    #[inline]
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProcessId({})", self.0)
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pid:{}", self.0)
    }
}

/// Worker thread index within a realm.
///
/// Workers are kernel threads (TCBs) that run the Lona VM. Each worker can
/// execute Lonala processes. A realm may have multiple workers for parallel
/// execution on multiple CPUs.
///
/// Worker IDs are small integers (0-255) representing the worker's index
/// within its realm.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct WorkerId(u16);

impl WorkerId {
    /// The first (and often only) worker in a realm.
    pub const FIRST: Self = Self(0);

    /// Maximum number of workers per realm.
    pub const MAX_WORKERS: u16 = 256;

    /// Creates a new worker ID.
    ///
    /// Returns `None` if the ID exceeds `MAX_WORKERS`.
    #[inline]
    #[must_use]
    pub const fn new(id: u16) -> Option<Self> {
        if id < Self::MAX_WORKERS {
            Some(Self(id))
        } else {
            None
        }
    }

    /// Creates a worker ID without bounds checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure `id < MAX_WORKERS`.
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(id: u16) -> Self {
        Self(id)
    }

    /// Returns the raw worker index.
    #[inline]
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Returns the worker index as usize (for array indexing).
    #[inline]
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Debug for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WorkerId({})", self.0)
    }
}

impl fmt::Display for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "worker:{}", self.0)
    }
}

/// Capability slot index in a realm's `CSpace`.
///
/// seL4 capabilities are stored in slots within a `CSpace`. This type represents
/// a slot index. Certain slots have fixed assignments (see constants below).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct CapSlot(u64);

impl CapSlot {
    // Fixed slot assignments for all realms

    /// Null capability slot (always empty).
    pub const NULL: Self = Self(0);

    /// `CSpace` root capability.
    pub const CSPACE: Self = Self(1);

    /// `VSpace` root capability.
    pub const VSPACE: Self = Self(2);

    /// TCB capability for the current worker.
    pub const TCB_SELF: Self = Self(3);

    /// Endpoint to Lona Memory Manager for IPC requests.
    pub const LMM_ENDPOINT: Self = Self(4);

    /// IPC buffer frame capability.
    pub const IPC_BUFFER: Self = Self(5);

    /// `IOPort` capability for UART (`x86_64` only).
    ///
    /// On `x86_64`, this slot contains an `IOPort` capability for COM1 (0x3F8-0x3FF).
    /// On `aarch64`, this slot is unused (UART uses MMIO at `UART_VADDR`).
    pub const IOPORT_UART: Self = Self(6);

    /// First slot available for dynamic allocation.
    pub const FIRST_FREE: Self = Self(16);

    /// Creates a new capability slot index.
    #[inline]
    #[must_use]
    pub const fn new(slot: u64) -> Self {
        Self(slot)
    }

    /// Returns the raw slot index.
    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Checks if this is the null slot.
    #[inline]
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for CapSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CapSlot({})", self.0)
    }
}

impl fmt::Display for CapSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slot:{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn realm_id_constants() {
        assert!(RealmId::NULL.is_null());
        assert!(!RealmId::INIT.is_null());
        assert_eq!(RealmId::INIT.as_u64(), 1);
    }

    #[test]
    fn process_id_constants() {
        assert!(ProcessId::NULL.is_null());
        assert!(!ProcessId::INIT.is_null());
        assert_eq!(ProcessId::INIT.as_u64(), 1);
    }

    #[test]
    fn worker_id_bounds() {
        assert!(WorkerId::new(0).is_some());
        assert!(WorkerId::new(255).is_some());
        assert!(WorkerId::new(256).is_none());
    }

    #[test]
    fn cap_slot_constants() {
        assert!(CapSlot::NULL.is_null());
        assert!(!CapSlot::CSPACE.is_null());
        assert!(CapSlot::FIRST_FREE.as_u64() >= 16);
    }

    // Address type tests

    #[test]
    fn paddr_basic() {
        let addr = Paddr::new(0x1000);
        assert_eq!(addr.as_u64(), 0x1000);
        assert!(!addr.is_null());
        assert!(Paddr::null().is_null());
    }

    #[test]
    fn paddr_arithmetic() {
        let addr = Paddr::new(0x1000);
        assert_eq!(addr.add(0x100).as_u64(), 0x1100);
        assert_eq!(addr.sub(0x100).as_u64(), 0x0F00);
        assert_eq!((addr + 0x100).as_u64(), 0x1100);
        assert_eq!((addr - 0x100).as_u64(), 0x0F00);
    }

    #[test]
    fn paddr_alignment() {
        let addr = Paddr::new(0x1234);
        assert_eq!(addr.align_up(0x1000).map(Paddr::as_u64), Some(0x2000));
        assert_eq!(addr.align_down(0x1000).map(Paddr::as_u64), Some(0x1000));
        assert_eq!(addr.is_aligned(0x1000), Some(false));
        assert_eq!(Paddr::new(0x2000).is_aligned(0x1000), Some(true));
        assert_eq!(addr.align_up(0), None);
        assert_eq!(addr.align_up(3), None);
    }

    #[test]
    fn vaddr_basic() {
        let addr = Vaddr::new(0x4000_0000);
        assert_eq!(addr.as_u64(), 0x4000_0000);
        assert!(!addr.is_null());
        assert!(Vaddr::null().is_null());
    }

    #[test]
    fn vaddr_arithmetic() {
        let addr = Vaddr::new(0x4000_0000);
        assert_eq!(addr.add(0x1000).as_u64(), 0x4000_1000);
        assert_eq!(addr.sub(0x1000).as_u64(), 0x3FFF_F000);
    }

    #[test]
    fn vaddr_alignment() {
        let addr = Vaddr::new(0x4000_1234);
        assert_eq!(addr.align_up(0x1000).map(Vaddr::as_u64), Some(0x4000_2000));
        assert_eq!(
            addr.align_down(0x1000).map(Vaddr::as_u64),
            Some(0x4000_1000)
        );
    }

    #[test]
    fn vaddr_diff() {
        let a = Vaddr::new(0x5000);
        let b = Vaddr::new(0x3000);
        assert_eq!(a.diff(b), 0x2000);
    }

    #[test]
    fn address_debug_format() {
        let paddr = Paddr::new(0x1234);
        let vaddr = Vaddr::new(0x5678);
        assert_eq!(format!("{paddr:?}"), "Paddr(0x1234)");
        assert_eq!(format!("{vaddr:?}"), "Vaddr(0x5678)");
    }
}
