// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Mock platform implementation for testing.
//!
//! This module provides a mock `VSpace` backed by a heap-allocated buffer,
//! allowing VM logic to be tested without an actual seL4 kernel.

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
    #[expect(
        clippy::panic,
        reason = "test mock panics intentionally on invalid address"
    )]
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
        // Empty slice at end boundary is valid
        if len == 0 {
            return &[];
        }
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
        // Empty slice at end boundary is valid
        if len == 0 {
            return &mut [];
        }
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
