//! Platform abstraction traits.

use crate::types::{Paddr, Vaddr};
use core::result::Result;

/// Abstraction over a virtual address space.
///
/// This trait allows VM code to read and write memory without knowing
/// whether it's operating on a real seL4 `VSpace` or a mock buffer.
pub trait MemorySpace {
    /// Read a value from a virtual address.
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T;

    /// Write a value to a virtual address.
    fn write<T>(&mut self, vaddr: Vaddr, value: T);

    /// Get a byte slice at a virtual address.
    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8];

    /// Get a mutable byte slice at a virtual address.
    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8];

    /// Zero out a range of memory.
    fn zero(&mut self, vaddr: Vaddr, len: usize) {
        let slice = self.slice_mut(vaddr, len);
        slice.fill(0);
    }

    /// Copy bytes from one location to another within the same `VSpace`.
    fn copy_within(&mut self, src: Vaddr, dst: Vaddr, len: usize);
}

/// Page permissions for memory mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagePerms {
    /// Allow reads.
    pub read: bool,
    /// Allow writes.
    pub write: bool,
    /// Allow execution.
    pub execute: bool,
}

impl PagePerms {
    /// Read-only permissions.
    pub const RO: Self = Self {
        read: true,
        write: false,
        execute: false,
    };

    /// Read-write permissions.
    pub const RW: Self = Self {
        read: true,
        write: true,
        execute: false,
    };

    /// Read-execute permissions.
    pub const RX: Self = Self {
        read: true,
        write: false,
        execute: true,
    };

    /// Read-write-execute permissions (rarely used).
    pub const RWX: Self = Self {
        read: true,
        write: true,
        execute: true,
    };
}

/// Cache attributes for memory mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheAttr {
    /// Normal cached memory.
    Cached,
    /// Uncached memory (for DMA descriptors).
    Uncached,
    /// Write-combining (for TX buffers, frame buffers).
    WriteCombine,
    /// Device memory (strongly ordered, for MMIO).
    Device,
}

/// Errors that can occur during page mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// Virtual address is already mapped.
    AlreadyMapped,
    /// Insufficient resources to create mapping.
    InsufficientResources,
    /// Address is not properly aligned.
    MisalignedAddress,
    /// Permission denied for this operation.
    PermissionDenied,
    /// Invalid capability.
    InvalidCapability,
}

impl core::fmt::Display for MapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AlreadyMapped => write!(f, "virtual address already mapped"),
            Self::InsufficientResources => write!(f, "insufficient resources"),
            Self::MisalignedAddress => write!(f, "address not properly aligned"),
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::InvalidCapability => write!(f, "invalid capability"),
        }
    }
}

/// Platform-specific operations.
///
/// This trait abstracts over the seL4 platform, allowing tests to run
/// without an actual seL4 kernel.
pub trait Platform {
    /// The memory space type for this platform.
    type VSpace: MemorySpace;

    /// Get the current time in nanoseconds.
    fn time_ns(&self) -> u64;

    /// Yield to the kernel scheduler.
    fn yield_cpu(&self);

    /// Map a page into the `VSpace`.
    ///
    /// # Errors
    ///
    /// Returns an error if the mapping cannot be created.
    fn map_page(
        &mut self,
        vspace: &mut Self::VSpace,
        vaddr: Vaddr,
        paddr: Paddr,
        perms: PagePerms,
        cache: CacheAttr,
    ) -> Result<(), MapError>;

    /// Unmap a page from the `VSpace`.
    ///
    /// # Errors
    ///
    /// Returns an error if the unmapping fails.
    fn unmap_page(&mut self, vspace: &mut Self::VSpace, vaddr: Vaddr) -> Result<(), MapError>;
}

// Compile-time verification of PagePerms constants
const _: () = {
    assert!(PagePerms::RO.read);
    assert!(!PagePerms::RO.write);
    assert!(!PagePerms::RO.execute);

    assert!(PagePerms::RW.read);
    assert!(PagePerms::RW.write);
    assert!(!PagePerms::RW.execute);

    assert!(PagePerms::RX.read);
    assert!(!PagePerms::RX.write);
    assert!(PagePerms::RX.execute);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_error_display() {
        let err = MapError::AlreadyMapped;
        assert_eq!(format!("{err}"), "virtual address already mapped");
    }
}
