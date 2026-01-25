// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Platform abstraction traits.

use crate::types::{Paddr, Vaddr};
use core::result::Result;

/// Abstraction over a virtual address space.
///
/// This trait allows VM code to read and write memory without knowing
/// whether it's operating on a real seL4 `VSpace` or a mock buffer.
///
/// # Safety Contract for Implementors
///
/// Implementors MUST ensure:
///
/// - **Memory validity**: All addresses passed to read/write/slice methods are
///   within the allocated memory region. Out-of-bounds access is undefined behavior.
///
/// - **Alignment**: Implementors should handle unaligned access appropriately for
///   the target platform (either supporting it or panicking with a clear error).
///
/// - **No overlapping mutable borrows**: The `slice_mut` and `write` methods must
///   not be called concurrently with other accesses to overlapping regions.
///
/// - **Initialization**: Memory returned by `slice` for types that require
///   initialization (e.g., `Term`) must be properly initialized.
///
/// While this trait is not marked `unsafe trait` for ergonomic reasons, violations
/// of these requirements can cause undefined behavior in the VM.
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

    /// Atomically read a u64 with Acquire ordering.
    ///
    /// Used for reading var slot content pointers to ensure we see
    /// all writes that happened before the corresponding Release store.
    fn read_u64_acquire(&self, vaddr: Vaddr) -> u64;

    /// Atomically write a u64 with Release ordering.
    ///
    /// Used for updating var slot content pointers to ensure all our
    /// writes are visible to readers before the pointer is published.
    fn write_u64_release(&self, vaddr: Vaddr, value: u64);
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

/// Real VSpace that interprets addresses directly as pointers.
///
/// This implementation is used on seL4 where virtual addresses are
/// directly accessible via pointer operations.
#[cfg(not(any(test, feature = "std")))]
#[derive(Default)]
pub struct Sel4VSpace;

#[cfg(not(any(test, feature = "std")))]
impl MemorySpace for Sel4VSpace {
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T {
        let addr = vaddr.as_u64();
        let align = core::mem::align_of::<T>() as u64;
        let size = core::mem::size_of::<T>();

        // Diagnostic assertion: catch misalignment before Rust's precondition check
        if addr == 0 {
            sel4::debug_println!(
                "DIAGNOSTIC: Null pointer in read<{}>",
                core::any::type_name::<T>()
            );
        }
        if addr % align != 0 {
            sel4::debug_println!(
                "DIAGNOSTIC: Misaligned read<{}> at {:#x} (need {}-byte alignment, size={})",
                core::any::type_name::<T>(),
                addr,
                align,
                size
            );
        }

        // SAFETY: Caller ensures vaddr is valid, mapped, and properly aligned.
        unsafe { vaddr.as_ptr::<T>().read() }
    }

    fn write<T>(&mut self, vaddr: Vaddr, value: T) {
        let addr = vaddr.as_u64();
        let align = core::mem::align_of::<T>() as u64;
        let size = core::mem::size_of::<T>();

        // Diagnostic assertion: catch misalignment before Rust's precondition check
        if addr == 0 {
            sel4::debug_println!(
                "DIAGNOSTIC: Null pointer in write<{}>",
                core::any::type_name::<T>()
            );
        }
        if addr % align != 0 {
            sel4::debug_println!(
                "DIAGNOSTIC: Misaligned write<{}> at {:#x} (need {}-byte alignment, size={})",
                core::any::type_name::<T>(),
                addr,
                align,
                size
            );
        }

        // SAFETY: Caller ensures vaddr is valid, mapped, and properly aligned.
        unsafe {
            vaddr.as_mut_ptr::<T>().write(value);
        }
    }

    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8] {
        // SAFETY: Caller ensures range is valid and mapped
        unsafe { core::slice::from_raw_parts(vaddr.as_ptr::<u8>(), len) }
    }

    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8] {
        // SAFETY: Caller ensures range is valid and mapped
        unsafe { core::slice::from_raw_parts_mut(vaddr.as_mut_ptr::<u8>(), len) }
    }

    fn copy_within(&mut self, src: Vaddr, dst: Vaddr, len: usize) {
        // SAFETY: Caller ensures ranges are valid and don't illegally overlap
        unsafe {
            core::ptr::copy(src.as_ptr::<u8>(), dst.as_mut_ptr::<u8>(), len);
        }
    }

    fn read_u64_acquire(&self, vaddr: Vaddr) -> u64 {
        // SAFETY: Caller ensures vaddr is valid, mapped, and 8-byte aligned.
        // Uses AtomicU64 for proper Acquire semantics.
        unsafe {
            let atomic = &*(vaddr.as_ptr::<u64>() as *const core::sync::atomic::AtomicU64);
            atomic.load(core::sync::atomic::Ordering::Acquire)
        }
    }

    fn write_u64_release(&self, vaddr: Vaddr, value: u64) {
        // SAFETY: Caller ensures vaddr is valid, mapped, and 8-byte aligned.
        // Uses AtomicU64 for proper Release semantics.
        // Note: Takes &self because atomic operations are inherently thread-safe.
        unsafe {
            let atomic = &*(vaddr.as_ptr::<u64>() as *const core::sync::atomic::AtomicU64);
            atomic.store(value, core::sync::atomic::Ordering::Release);
        }
    }
}
