// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! ID types for realms, processes, and workers.

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
