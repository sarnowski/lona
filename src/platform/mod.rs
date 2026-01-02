//! Platform abstraction layer.
//!
//! This module provides traits that abstract over platform-specific operations,
//! allowing the VM to run on seL4 in production while using mock implementations
//! for testing on the development host.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    VM Logic                              │
//! │  (GC, Scheduler, Interpreter, Mailbox, etc.)            │
//! └─────────────────────────────────────────────────────────┘
//!                           │
//!                           ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │              Platform Traits                             │
//! │  (`MemorySpace`, `Platform`)                             │
//! └─────────────────────────────────────────────────────────┘
//!            │                              │
//!            ▼                              ▼
//! ┌─────────────────────┐      ┌─────────────────────────────┐
//! │   `MockPlatform`    │      │     `Sel4Platform`          │
//! │   (for testing)     │      │     (for production)        │
//! └─────────────────────┘      └─────────────────────────────┘
//! ```

#[cfg(any(test, feature = "std"))]
mod mock;
mod traits;

#[cfg(any(test, feature = "std"))]
pub use mock::MockVSpace;
pub use traits::{CacheAttr, MapError, MemorySpace, PagePerms, Platform};

/// `VSpace` layout constants matching `concept.md` Section 14.
pub mod vspace_layout {
    use crate::Vaddr;

    /// Unmapped region for null pointer traps.
    pub const NULL_GUARD: Vaddr = Vaddr::new(0x0000_0000_0000);

    /// Global control structures (namespace epoch, sequence lock, config).
    pub const GLOBAL_CONTROL: Vaddr = Vaddr::new(0x0000_0010_0000);

    /// Per-scheduler state (run queues, current process, stacks).
    pub const SCHEDULER_STATE: Vaddr = Vaddr::new(0x0000_0020_0000);

    /// Read-only namespace registry from ancestors.
    pub const NAMESPACE_RO: Vaddr = Vaddr::new(0x0000_0100_0000);

    /// Read-write local namespace registry.
    pub const NAMESPACE_RW: Vaddr = Vaddr::new(0x0000_0200_0000);

    /// Immutable namespace object snapshots.
    pub const NAMESPACE_OBJECTS: Vaddr = Vaddr::new(0x0000_1000_0000);

    /// Read-only ancestor code pages.
    pub const ANCESTOR_CODE: Vaddr = Vaddr::new(0x0000_2000_0000);

    /// Read-write local code pages.
    pub const LOCAL_CODE: Vaddr = Vaddr::new(0x0000_3000_0000);

    /// Process heap regions.
    pub const PROCESS_HEAPS: Vaddr = Vaddr::new(0x0000_4000_0000);

    /// Shared binary heap (reference counted large binaries).
    pub const SHARED_BINARY: Vaddr = Vaddr::new(0x0000_8000_0000);

    /// Cross-realm shared memory regions.
    pub const CROSS_REALM_SHARED: Vaddr = Vaddr::new(0x0001_0000_0000);

    /// Device MMIO mappings (driver realms only).
    pub const DEVICE_MAPPINGS: Vaddr = Vaddr::new(0x00F0_0000_0000);

    /// Start of kernel reserved region.
    pub const KERNEL_RESERVED: Vaddr = Vaddr::new(0xFFFF_8000_0000);

    /// Standard page size (4 KiB).
    pub const PAGE_SIZE: u64 = 4096;

    /// Large page size (2 MiB).
    pub const LARGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;

    /// Huge page size (1 GiB).
    pub const HUGE_PAGE_SIZE: u64 = 1024 * 1024 * 1024;
}
