//! Mock platform implementation for testing.
//!
//! This module provides a mock `VSpace` backed by a heap-allocated buffer,
//! allowing VM logic to be tested without an actual seL4 kernel.

#![allow(unsafe_code)] // Required for low-level memory operations
#![allow(clippy::panic)] // Test infrastructure - panicking on invalid input is correct

use crate::platform::traits::MemorySpace;
use crate::types::Vaddr;

#[cfg(any(test, feature = "std"))]
use std::{boxed::Box, vec};

#[cfg(not(any(test, feature = "std")))]
use alloc::{boxed::Box, vec};

/// A mock virtual address space backed by a heap-allocated buffer.
///
/// This allows testing VM components that operate on memory without
/// requiring a real seL4 `VSpace`. The mock `VSpace` simulates a contiguous
/// region of memory starting at a configurable base address.
pub struct MockVSpace {
    memory: Box<[u8]>,
    base: Vaddr,
}

impl MockVSpace {
    /// Create a new mock `VSpace` with the given size and base address.
    #[must_use]
    pub fn new(size: usize, base: Vaddr) -> Self {
        Self {
            memory: vec![0u8; size].into_boxed_slice(),
            base,
        }
    }

    /// Get the base address of this `VSpace`.
    #[inline]
    #[must_use]
    pub const fn base(&self) -> Vaddr {
        self.base
    }

    /// Get the size of this `VSpace` in bytes.
    #[inline]
    #[must_use]
    pub fn size(&self) -> usize {
        self.memory.len()
    }

    /// Get the end address (exclusive) of this `VSpace`.
    #[inline]
    #[must_use]
    pub fn end(&self) -> Vaddr {
        self.base.add(self.memory.len() as u64)
    }

    /// Check if a virtual address is within this `VSpace`.
    #[inline]
    #[must_use]
    pub fn contains(&self, vaddr: Vaddr) -> bool {
        vaddr >= self.base && vaddr < self.end()
    }

    /// Convert a virtual address to an offset into the backing buffer.
    fn offset(&self, vaddr: Vaddr) -> usize {
        assert!(
            vaddr >= self.base,
            "virtual address {vaddr} is below base {}",
            self.base
        );
        let offset = vaddr.as_u64().wrapping_sub(self.base.as_u64());
        let offset_usize = usize::try_from(offset).unwrap_or_else(|_| {
            panic!("virtual address {vaddr} exceeds usize::MAX on this platform")
        });
        assert!(
            offset_usize < self.memory.len(),
            "virtual address {vaddr} is beyond end {}",
            self.end()
        );
        offset_usize
    }

    /// Get raw access to the backing memory (for debugging/testing).
    #[must_use]
    pub fn raw_memory(&self) -> &[u8] {
        &self.memory
    }

    /// Get mutable raw access to the backing memory (for debugging/testing).
    #[must_use]
    pub fn raw_memory_mut(&mut self) -> &mut [u8] {
        &mut self.memory
    }
}

impl MemorySpace for MockVSpace {
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T {
        let offset = self.offset(vaddr);
        let size = core::mem::size_of::<T>();
        assert!(
            offset
                .checked_add(size)
                .is_some_and(|end| end <= self.memory.len()),
            "read of {size} bytes at {vaddr} would exceed VSpace bounds"
        );

        let ptr = self.memory[offset..].as_ptr().cast::<T>();
        // SAFETY: We've verified bounds above, and we're reading from our own buffer.
        // Using read_unaligned because we don't enforce alignment in the mock.
        unsafe { ptr.read_unaligned() }
    }

    fn write<T>(&mut self, vaddr: Vaddr, value: T) {
        let offset = self.offset(vaddr);
        let size = core::mem::size_of::<T>();
        assert!(
            offset
                .checked_add(size)
                .is_some_and(|end| end <= self.memory.len()),
            "write of {size} bytes at {vaddr} would exceed VSpace bounds"
        );

        let ptr = self.memory[offset..].as_mut_ptr().cast::<T>();
        // SAFETY: We've verified bounds above, and we're writing to our own buffer.
        // Using write_unaligned because we don't enforce alignment in the mock.
        unsafe { ptr.write_unaligned(value) }
    }

    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8] {
        let offset = self.offset(vaddr);
        assert!(
            offset
                .checked_add(len)
                .is_some_and(|end| end <= self.memory.len()),
            "slice of {len} bytes at {vaddr} would exceed VSpace bounds"
        );
        &self.memory[offset..offset + len]
    }

    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8] {
        let offset = self.offset(vaddr);
        assert!(
            offset
                .checked_add(len)
                .is_some_and(|end| end <= self.memory.len()),
            "slice of {len} bytes at {vaddr} would exceed VSpace bounds"
        );
        &mut self.memory[offset..offset + len]
    }

    fn copy_within(&mut self, src: Vaddr, dst: Vaddr, len: usize) {
        let src_offset = self.offset(src);
        let dst_offset = self.offset(dst);
        assert!(
            src_offset
                .checked_add(len)
                .is_some_and(|end| end <= self.memory.len()),
            "copy source exceeds VSpace bounds"
        );
        assert!(
            dst_offset
                .checked_add(len)
                .is_some_and(|end| end <= self.memory.len()),
            "copy destination exceeds VSpace bounds"
        );
        self.memory
            .copy_within(src_offset..src_offset + len, dst_offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_vspace_creation() {
        let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
        assert_eq!(vspace.base(), Vaddr::new(0x1000));
        assert_eq!(vspace.size(), 4096);
        assert_eq!(vspace.end(), Vaddr::new(0x2000));
    }

    #[test]
    fn test_mock_vspace_contains() {
        let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
        assert!(vspace.contains(Vaddr::new(0x1000)));
        assert!(vspace.contains(Vaddr::new(0x1FFF)));
        assert!(!vspace.contains(Vaddr::new(0x0FFF)));
        assert!(!vspace.contains(Vaddr::new(0x2000)));
    }

    #[test]
    fn test_mock_vspace_read_write_u32() {
        let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

        vspace.write(Vaddr::new(0x1000), 0xDEAD_BEEFu32);
        let value: u32 = vspace.read(Vaddr::new(0x1000));
        assert_eq!(value, 0xDEAD_BEEF);
    }

    #[test]
    fn test_mock_vspace_read_write_u64() {
        let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

        vspace.write(Vaddr::new(0x1008), 0x1234_5678_9ABC_DEF0u64);
        let value: u64 = vspace.read(Vaddr::new(0x1008));
        assert_eq!(value, 0x1234_5678_9ABC_DEF0);
    }

    #[test]
    fn test_mock_vspace_read_write_struct() {
        #[repr(C)]
        #[derive(Clone, Copy, Debug, PartialEq)]
        struct TestStruct {
            a: u32,
            b: u64,
            c: u16,
        }

        let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

        let original = TestStruct {
            a: 42,
            b: 0xDEAD_BEEF_CAFE_BABE,
            c: 1234,
        };

        vspace.write(Vaddr::new(0x1100), original);
        let read_back: TestStruct = vspace.read(Vaddr::new(0x1100));
        assert_eq!(read_back, original);
    }

    #[test]
    fn test_mock_vspace_slice() {
        let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

        let data = b"Hello, World!";
        vspace
            .slice_mut(Vaddr::new(0x1000), data.len())
            .copy_from_slice(data);

        let slice = vspace.slice(Vaddr::new(0x1000), data.len());
        assert_eq!(slice, data);
    }

    #[test]
    fn test_mock_vspace_zero() {
        let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

        vspace.write(Vaddr::new(0x1000), 0xFFFF_FFFFu32);
        vspace.zero(Vaddr::new(0x1000), 4);

        let value: u32 = vspace.read(Vaddr::new(0x1000));
        assert_eq!(value, 0);
    }

    #[test]
    fn test_mock_vspace_copy_within() {
        let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

        vspace.write(Vaddr::new(0x1000), 0xDEAD_BEEFu32);
        vspace.copy_within(Vaddr::new(0x1000), Vaddr::new(0x1100), 4);

        let src: u32 = vspace.read(Vaddr::new(0x1000));
        let dst: u32 = vspace.read(Vaddr::new(0x1100));
        assert_eq!(src, 0xDEAD_BEEF);
        assert_eq!(dst, 0xDEAD_BEEF);
    }

    #[test]
    #[should_panic(expected = "below base")]
    fn test_mock_vspace_read_below_base() {
        let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
        let _: u32 = vspace.read(Vaddr::new(0x0FFF));
    }

    #[test]
    #[should_panic(expected = "beyond end")]
    fn test_mock_vspace_read_beyond_end() {
        let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
        let _: u32 = vspace.read(Vaddr::new(0x2000));
    }

    #[test]
    #[should_panic(expected = "would exceed")]
    fn test_mock_vspace_read_partial_beyond_end() {
        let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
        let _: u32 = vspace.read(Vaddr::new(0x1FFE));
    }

    #[test]
    fn test_mock_vspace_unaligned_access() {
        let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

        vspace.write(Vaddr::new(0x1001), 0xDEAD_BEEFu32);
        let value: u32 = vspace.read(Vaddr::new(0x1001));
        assert_eq!(value, 0xDEAD_BEEF);
    }

    #[test]
    fn test_mock_vspace_raw_memory_access() {
        let mut vspace = MockVSpace::new(16, Vaddr::new(0x1000));

        vspace.write(Vaddr::new(0x1000), 0x1234_5678u32);

        let raw = vspace.raw_memory();
        assert_eq!(raw[0], 0x78);
        assert_eq!(raw[1], 0x56);
        assert_eq!(raw[2], 0x34);
        assert_eq!(raw[3], 0x12);
    }
}
