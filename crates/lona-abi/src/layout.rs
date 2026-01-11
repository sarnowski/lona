// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! `VSpace` layout constants and region definitions.
//!
//! This module defines the fixed virtual address layout for all realms.
//! Using fixed addresses ensures that pointers in inherited regions remain
//! valid across realm boundaries.
//!
//! # `VSpace` Layout (64-bit)
//!
//! ```text
//! 0x0000_0000_0000_0000  NULL guard (unmapped, 4 KB)
//! 0x0000_0000_1000_0000  Worker stacks region (256 MB)
//! 0x0000_0001_0000_0000  Shared code (4 GB)
//! 0x0000_0002_0000_0000  Inherited regions (64 GB)
//! 0x0000_0012_0000_0000  Realm-local data (4 GB)
//! 0x0000_0013_0000_0000  Realm binary heap (16 GB)
//! 0x0000_0020_0000_0000  Process pool (240 GB)
//! 0x0000_00F0_0000_0000  MMIO/Device region (16 GB)
//! 0xFFFF_8000_0000_0000  Kernel (unmapped in userspace)
//! ```

/// One gigabyte in bytes.
const GB: u64 = 1024 * 1024 * 1024;

/// One megabyte in bytes.
const MB: u64 = 1024 * 1024;

/// One kilobyte in bytes.
const KB: u64 = 1024;

/// Standard page size (4 KB).
pub const PAGE_SIZE: u64 = 4 * KB;

/// Large page size (2 MB).
pub const LARGE_PAGE_SIZE: u64 = 2 * MB;

/// Page size shift (log2 of `PAGE_SIZE`).
pub const PAGE_SHIFT: u32 = 12;

/// Large page size shift (log2 of `LARGE_PAGE_SIZE`).
pub const LARGE_PAGE_SHIFT: u32 = 21;

// =============================================================================
// VSpace Region Base Addresses
// =============================================================================

/// Base address of the NULL guard region.
///
/// This region is never mapped to catch null pointer dereferences.
pub const NULL_GUARD_BASE: u64 = 0x0000_0000_0000_0000;

/// Size of the NULL guard region (4 KB).
pub const NULL_GUARD_SIZE: u64 = PAGE_SIZE;

/// Base address of the worker stacks region.
///
/// Contains native stacks for Lona VM workers (TCBs). Each worker slot:
/// - Guard page (4 KB)
/// - Stack (256 KB, grows down from top)
/// - Guard page (4 KB)
/// - IPC buffer (4 KB, required by seL4)
pub const WORKER_STACKS_BASE: u64 = 0x0000_0000_1000_0000;

/// Size of the worker stacks region (256 MB).
pub const WORKER_STACKS_SIZE: u64 = 256 * MB;

/// Stack size per worker (256 KB).
pub const WORKER_STACK_SIZE: u64 = 256 * KB;

/// IPC buffer size per worker (4 KB, required by seL4).
pub const WORKER_IPC_BUFFER_SIZE: u64 = PAGE_SIZE;

/// Total size per worker slot (stack + IPC buffer + guards).
pub const WORKER_SLOT_SIZE: u64 = WORKER_STACK_SIZE + WORKER_IPC_BUFFER_SIZE + 2 * PAGE_SIZE;

/// Base address of the shared code region.
///
/// Contains Lona VM executable code and core library, mapped into all realms
/// from the same physical frames. Permissions: RX for code, RO for rodata.
pub const SHARED_CODE_BASE: u64 = 0x0000_0001_0000_0000;

/// Size of the shared code region (4 GB).
pub const SHARED_CODE_SIZE: u64 = 4 * GB;

/// Base address of inherited regions.
///
/// Each ancestor gets a fixed-size slot containing code and binary sub-regions.
/// Child realms map parent regions read-only at these addresses.
pub const INHERITED_BASE: u64 = 0x0000_0002_0000_0000;

/// Size of the inherited regions area (64 GB).
pub const INHERITED_SIZE: u64 = 64 * GB;

/// Size per ancestor slot (5 GB: 1 GB code + 4 GB binary).
pub const ANCESTOR_SLOT_SIZE: u64 = 5 * GB;

/// Code sub-region size per ancestor (1 GB).
pub const ANCESTOR_CODE_SIZE: u64 = GB;

/// Binary sub-region size per ancestor (4 GB).
pub const ANCESTOR_BINARY_SIZE: u64 = 4 * GB;

/// Maximum supported ancestor depth.
pub const MAX_ANCESTORS: u64 = INHERITED_SIZE / ANCESTOR_SLOT_SIZE;

/// Base address of realm-local data.
///
/// Contains scheduler state, process table, local namespaces, atom table.
/// Each realm has its own data here. Permissions: RW.
pub const REALM_LOCAL_BASE: u64 = 0x0000_0012_0000_0000;

/// Size of the realm-local data region (4 GB).
pub const REALM_LOCAL_SIZE: u64 = 4 * GB;

/// Base address of the realm binary heap.
///
/// Contains large binaries (≥64 bytes) with reference counting.
/// Separate from code region for efficient large data handling.
pub const REALM_BINARY_BASE: u64 = 0x0000_0013_0000_0000;

/// Size of the realm binary heap region (16 GB).
pub const REALM_BINARY_SIZE: u64 = 16 * GB;

/// Base address of the process pool.
///
/// Dynamic segments for process heaps, stacks, and mailboxes.
/// Allocated on demand as processes are created.
pub const PROCESS_POOL_BASE: u64 = 0x0000_0020_0000_0000;

/// Size of the process pool region (240 GB).
pub const PROCESS_POOL_SIZE: u64 = 240 * GB;

/// Base address of the MMIO/device region.
///
/// Only mapped in driver realms. Contains memory-mapped device registers.
pub const MMIO_BASE: u64 = 0x0000_00F0_0000_0000;

/// Size of the MMIO region (16 GB).
pub const MMIO_SIZE: u64 = 16 * GB;

/// Virtual address where UART is mapped (for init realm).
pub const UART_VADDR: u64 = MMIO_BASE;

/// Default initial heap size for init realm (128 KB).
/// This is sufficient for realm code region + REPL process heaps.
pub const INIT_HEAP_SIZE: u64 = 128 * KB;

// =============================================================================
// Region Types and Permissions
// =============================================================================

/// Type of a memory region in a realm's `VSpace`.
///
/// The Lona Memory Manager uses this to determine how to handle page faults
/// and what permissions to apply when mapping frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RegionType {
    /// Guard region - never mapped, access terminates realm.
    Guard = 0,

    /// Shared VM code - RX, same frames for all realms.
    SharedCode = 1,

    /// Shared VM rodata - RO, same frames for all realms.
    SharedData = 2,

    /// Inherited region from ancestor - RO, lazy-mapped.
    /// The associated `u8` is the ancestor level (0 = root, 1 = init, etc.).
    Inherited = 3,

    /// Realm-local data - RW, allocated fresh.
    RealmLocal = 4,

    /// Realm binary heap - RW, allocated fresh.
    RealmBinary = 5,

    /// Process pool - RW, allocated on demand.
    ProcessPool = 6,

    /// Worker stack - RW, pre-allocated per worker.
    WorkerStack = 7,

    /// MMIO region - RW uncached, device memory.
    Mmio = 8,
}

/// Memory access permissions for a region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Permissions {
    /// No access (guard pages).
    None = 0,

    /// Read-only.
    ReadOnly = 1,

    /// Read-write.
    ReadWrite = 2,

    /// Read-execute (code).
    ReadExecute = 3,
}

impl Permissions {
    /// Returns true if reading is allowed.
    #[inline]
    #[must_use]
    pub const fn can_read(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Returns true if writing is allowed.
    #[inline]
    #[must_use]
    pub const fn can_write(self) -> bool {
        matches!(self, Self::ReadWrite)
    }

    /// Returns true if execution is allowed.
    #[inline]
    #[must_use]
    pub const fn can_execute(self) -> bool {
        matches!(self, Self::ReadExecute)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Calculates the virtual address for an ancestor's code sub-region.
///
/// # Arguments
///
/// * `ancestor_level` - Ancestor index (0 = root realm, 1 = init realm, etc.)
#[inline]
#[must_use]
pub const fn ancestor_code_base(ancestor_level: u8) -> u64 {
    INHERITED_BASE + (ancestor_level as u64) * ANCESTOR_SLOT_SIZE
}

/// Calculates the virtual address for an ancestor's binary sub-region.
///
/// # Arguments
///
/// * `ancestor_level` - Ancestor index (0 = root realm, 1 = init realm, etc.)
#[inline]
#[must_use]
pub const fn ancestor_binary_base(ancestor_level: u8) -> u64 {
    ancestor_code_base(ancestor_level) + ANCESTOR_CODE_SIZE
}

/// Calculates the stack base address for a worker.
///
/// # Arguments
///
/// * `worker_index` - Worker index within the realm (0-255)
#[inline]
#[must_use]
pub const fn worker_stack_base(worker_index: u16) -> u64 {
    WORKER_STACKS_BASE + (worker_index as u64) * WORKER_SLOT_SIZE + PAGE_SIZE
}

/// Calculates the IPC buffer address for a worker.
///
/// # Arguments
///
/// * `worker_index` - Worker index within the realm (0-255)
#[inline]
#[must_use]
pub const fn worker_ipc_buffer(worker_index: u16) -> u64 {
    WORKER_STACKS_BASE
        + (worker_index as u64) * WORKER_SLOT_SIZE
        + PAGE_SIZE
        + WORKER_STACK_SIZE
        + PAGE_SIZE
}

// Compile-time verification that regions do not overlap
const _: () = {
    assert!(NULL_GUARD_BASE + NULL_GUARD_SIZE <= WORKER_STACKS_BASE);
    assert!(WORKER_STACKS_BASE + WORKER_STACKS_SIZE <= SHARED_CODE_BASE);
    assert!(SHARED_CODE_BASE + SHARED_CODE_SIZE <= INHERITED_BASE);
    assert!(INHERITED_BASE + INHERITED_SIZE <= REALM_LOCAL_BASE);
    assert!(REALM_LOCAL_BASE + REALM_LOCAL_SIZE <= REALM_BINARY_BASE);
    assert!(REALM_BINARY_BASE + REALM_BINARY_SIZE <= PROCESS_POOL_BASE);
    assert!(PROCESS_POOL_BASE + PROCESS_POOL_SIZE <= MMIO_BASE);
    assert!(MAX_ANCESTORS >= 12); // Support at least 12 ancestors
};

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn page_size_is_power_of_two() {
        assert!(PAGE_SIZE.is_power_of_two());
        assert!(LARGE_PAGE_SIZE.is_power_of_two());
    }

    #[test]
    fn ancestor_addresses_are_ordered() {
        let a0_code = ancestor_code_base(0);
        let a0_binary = ancestor_binary_base(0);
        let a1_code = ancestor_code_base(1);

        assert!(a0_code < a0_binary);
        assert!(a0_binary < a1_code);
    }

    #[test]
    fn worker_addresses_are_ordered() {
        let w0_stack = worker_stack_base(0);
        let w0_ipc = worker_ipc_buffer(0);
        let w1_stack = worker_stack_base(1);

        assert!(w0_stack < w0_ipc);
        assert!(w0_ipc < w1_stack);
    }

    #[test]
    fn permissions_checks() {
        assert!(!Permissions::None.can_read());
        assert!(!Permissions::None.can_write());
        assert!(!Permissions::None.can_execute());

        assert!(Permissions::ReadOnly.can_read());
        assert!(!Permissions::ReadOnly.can_write());

        assert!(Permissions::ReadWrite.can_read());
        assert!(Permissions::ReadWrite.can_write());

        assert!(Permissions::ReadExecute.can_read());
        assert!(Permissions::ReadExecute.can_execute());
    }
}
