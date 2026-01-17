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

/// Initial heap size for init realm (32 KB).
///
/// This is intentionally smaller than what the VM needs, forcing it to
/// request additional pages from the Lona Memory Manager via IPC.
pub const INIT_HEAP_SIZE: u64 = 32 * KB;

// =============================================================================
// Fault Region Classification
// =============================================================================

/// Result of classifying a faulting address.
///
/// Used by the Lona Memory Manager to determine what kind of error occurred
/// when a fault happens. Note: Most regions do NOT use fault-based allocation.
/// Only inherited regions use lazy mapping; all other faults indicate errors.
///
/// | Region | On Fault |
/// |--------|----------|
/// | ProcessPool | ERROR - should use explicit IPC |
/// | RealmBinary | ERROR - should use explicit IPC |
/// | RealmLocal | ERROR - should use explicit IPC |
/// | WorkerStack | ERROR - stacks are pre-mapped |
/// | Invalid | ERROR - invalid memory access |
/// | (Inherited) | Not in enum - see `is_inherited_region()` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultRegion {
    /// Address is in the process pool region.
    ProcessPool,
    /// Address is in the realm binary heap region.
    RealmBinary,
    /// Address is in the realm-local data region.
    RealmLocal,
    /// Address is in a worker stack region. Contains the worker index.
    WorkerStack(u16),
    /// Address is invalid (null guard, kernel space, etc).
    Invalid,
}

impl FaultRegion {
    /// Determine which region a faulting address belongs to.
    ///
    /// This is used by the fault handler to decide whether to map a page
    /// or reject the fault.
    #[must_use]
    pub const fn from_addr(addr: u64) -> Self {
        // Null guard page - never map
        if addr < PAGE_SIZE {
            return Self::Invalid;
        }

        // Worker stacks region
        if addr >= WORKER_STACKS_BASE && addr < WORKER_STACKS_BASE + WORKER_STACKS_SIZE {
            let offset = addr - WORKER_STACKS_BASE;
            let worker = (offset / WORKER_SLOT_SIZE) as u16;
            return Self::WorkerStack(worker);
        }

        // Process pool region
        if addr >= PROCESS_POOL_BASE && addr < PROCESS_POOL_BASE + PROCESS_POOL_SIZE {
            return Self::ProcessPool;
        }

        // Realm binary heap region
        if addr >= REALM_BINARY_BASE && addr < REALM_BINARY_BASE + REALM_BINARY_SIZE {
            return Self::RealmBinary;
        }

        // Realm-local data region
        if addr >= REALM_LOCAL_BASE && addr < REALM_LOCAL_BASE + REALM_LOCAL_SIZE {
            return Self::RealmLocal;
        }

        // Everything else is invalid (shared code should be pre-mapped,
        // inherited regions are RO, MMIO is device-specific)
        Self::Invalid
    }

    /// Check if this region classification indicates a potentially valid access.
    ///
    /// Returns `true` if the address is in a known region (for error reporting).
    /// Returns `false` if the address is completely invalid (null, kernel, etc).
    ///
    /// NOTE: This does NOT mean the fault should be handled by mapping a page.
    /// For allocation decisions, check `is_inherited_region()` first.
    #[inline]
    #[must_use]
    pub const fn is_mappable(&self) -> bool {
        !matches!(self, Self::Invalid)
    }
}

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

/// Check if an address is in the inherited regions area.
///
/// Inherited regions contain code and data from ancestor realms. They are
/// the ONLY regions that use fault-based lazy mapping. All other regions
/// use explicit IPC allocation or are pre-mapped at realm creation.
///
/// Lazy mapping for inherited regions is required because:
/// 1. Parent realms don't know their descendants (can't push updates)
/// 2. Live code updates must propagate to children automatically
/// 3. Pre-mapping all ancestor pages would be expensive and incomplete
///    (new parent pages allocated after child creation need lazy mapping)
#[inline]
#[must_use]
pub const fn is_inherited_region(addr: u64) -> bool {
    addr >= INHERITED_BASE && addr < INHERITED_BASE + INHERITED_SIZE
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

    // =========================================================================
    // FaultRegion Tests
    // =========================================================================

    #[test]
    fn fault_region_null_guard() {
        // Address 0 should be invalid (null guard)
        assert_eq!(FaultRegion::from_addr(0), FaultRegion::Invalid);
        assert!(!FaultRegion::from_addr(0).is_mappable());

        // Anything in the first page is null guard
        assert_eq!(FaultRegion::from_addr(0x100), FaultRegion::Invalid);
        assert_eq!(FaultRegion::from_addr(PAGE_SIZE - 1), FaultRegion::Invalid);
    }

    #[test]
    fn fault_region_process_pool() {
        // Base of process pool
        let base = FaultRegion::from_addr(PROCESS_POOL_BASE);
        assert_eq!(base, FaultRegion::ProcessPool);
        assert!(base.is_mappable());

        // Middle of process pool
        let mid = FaultRegion::from_addr(PROCESS_POOL_BASE + 0x1_0000_0000);
        assert_eq!(mid, FaultRegion::ProcessPool);
        assert!(mid.is_mappable());

        // Just before end of process pool
        let before_end = FaultRegion::from_addr(PROCESS_POOL_BASE + PROCESS_POOL_SIZE - 1);
        assert_eq!(before_end, FaultRegion::ProcessPool);
    }

    #[test]
    fn fault_region_worker_stack() {
        // Worker 0 stack
        let w0 = FaultRegion::from_addr(worker_stack_base(0) + 0x100);
        assert_eq!(w0, FaultRegion::WorkerStack(0));
        assert!(w0.is_mappable());

        // Worker 1 stack
        let w1 = FaultRegion::from_addr(worker_stack_base(1) + 0x100);
        assert_eq!(w1, FaultRegion::WorkerStack(1));

        // Worker 10 stack
        let w10 = FaultRegion::from_addr(worker_stack_base(10) + 0x100);
        assert_eq!(w10, FaultRegion::WorkerStack(10));
    }

    #[test]
    fn fault_region_realm_binary() {
        let base = FaultRegion::from_addr(REALM_BINARY_BASE);
        assert_eq!(base, FaultRegion::RealmBinary);
        assert!(base.is_mappable());

        let mid = FaultRegion::from_addr(REALM_BINARY_BASE + 0x1000_0000);
        assert_eq!(mid, FaultRegion::RealmBinary);
    }

    #[test]
    fn fault_region_realm_local() {
        let base = FaultRegion::from_addr(REALM_LOCAL_BASE);
        assert_eq!(base, FaultRegion::RealmLocal);
        assert!(base.is_mappable());

        let mid = FaultRegion::from_addr(REALM_LOCAL_BASE + 0x1000);
        assert_eq!(mid, FaultRegion::RealmLocal);
    }

    #[test]
    fn fault_region_boundaries() {
        // Just before process pool should be invalid (it's in realm binary region)
        let before_pool = FaultRegion::from_addr(PROCESS_POOL_BASE - 1);
        // This is still in REALM_BINARY (0x0000_0013... + 16GB ends before 0x0000_0020...)
        // Actually check: REALM_BINARY_BASE + REALM_BINARY_SIZE = 0x0000_0013_0000_0000 + 16GB
        // = 0x0000_0017_0000_0000, which is less than PROCESS_POOL_BASE (0x0000_0020_0000_0000)
        // So the address just before PROCESS_POOL_BASE is in invalid territory
        assert_eq!(before_pool, FaultRegion::Invalid);

        // Just after process pool should be invalid
        let after_pool = FaultRegion::from_addr(PROCESS_POOL_BASE + PROCESS_POOL_SIZE);
        assert_eq!(after_pool, FaultRegion::Invalid);

        // Shared code region should be invalid (pre-mapped, not demand-paged)
        let shared = FaultRegion::from_addr(SHARED_CODE_BASE + 0x1000);
        assert_eq!(shared, FaultRegion::Invalid);

        // MMIO region should be invalid (device-specific)
        let mmio = FaultRegion::from_addr(MMIO_BASE + 0x1000);
        assert_eq!(mmio, FaultRegion::Invalid);
    }

    // =========================================================================
    // is_inherited_region Tests
    // =========================================================================

    #[test]
    fn is_inherited_region_tests() {
        // Before inherited region
        assert!(!is_inherited_region(INHERITED_BASE - 1));

        // Start of inherited region
        assert!(is_inherited_region(INHERITED_BASE));

        // Middle of inherited region
        assert!(is_inherited_region(INHERITED_BASE + INHERITED_SIZE / 2));

        // Just before end of inherited region
        assert!(is_inherited_region(INHERITED_BASE + INHERITED_SIZE - 1));

        // End of inherited region (exclusive)
        assert!(!is_inherited_region(INHERITED_BASE + INHERITED_SIZE));

        // Well after inherited region
        assert!(!is_inherited_region(PROCESS_POOL_BASE));
    }
}
