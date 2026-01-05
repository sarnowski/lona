// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Core type definitions for realm, process, and capability identifiers.
//!
//! These newtypes prevent accidentally mixing different ID types at compile time.

use core::fmt;

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
}
