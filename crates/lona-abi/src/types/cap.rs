// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Capability slot type.

use core::fmt;

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
