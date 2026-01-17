// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Process memory pool allocator.
//!
//! The `ProcessPool` is a simple bump allocator that allocates memory regions
//! for process heaps. It tracks a contiguous region of memory and hands out
//! chunks for process young and old heaps.
//!
//! When the pool runs out of space, it can request additional pages from the
//! Lona Memory Manager via IPC.

use crate::Vaddr;
use crate::platform::lmm::lmm_request_pages;
use lona_abi::ipc::IpcRegionType;
use lona_abi::layout::PAGE_SIZE;

/// Minimum number of pages to request when growing the pool.
const MIN_GROWTH_PAGES: u64 = 16;

/// Bump allocator for process memory regions.
///
/// The pool tracks a contiguous region of memory and allocates
/// chunks for process heaps using a simple bump pointer.
///
/// When allocations fail due to insufficient space, the pool can
/// request additional pages from the Lona Memory Manager.
pub struct ProcessPool {
    /// Next available address for allocation.
    next: Vaddr,
    /// Upper limit of the pool (exclusive).
    limit: Vaddr,
}

impl ProcessPool {
    /// Create a new pool starting at `base` with `size` bytes.
    ///
    /// Uses saturating arithmetic to prevent overflow if base + size exceeds `u64::MAX`.
    #[must_use]
    pub const fn new(base: Vaddr, size: usize) -> Self {
        Self {
            next: base,
            limit: Vaddr::new(base.as_u64().saturating_add(size as u64)),
        }
    }

    /// Returns the current allocation pointer.
    #[must_use]
    pub const fn next(&self) -> Vaddr {
        self.next
    }

    /// Returns the upper limit of the pool.
    #[must_use]
    pub const fn limit(&self) -> Vaddr {
        self.limit
    }

    /// Returns the remaining space in the pool.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.limit.as_u64().saturating_sub(self.next.as_u64()) as usize
    }

    /// Allocate a contiguous region from the pool.
    ///
    /// Returns the base address or `None` if insufficient space.
    /// The allocation is aligned to `align` bytes.
    pub fn allocate(&mut self, size: usize, align: usize) -> Option<Vaddr> {
        // Align next up
        let mask = (align as u64).wrapping_sub(1);
        let aligned = (self.next.as_u64() + mask) & !mask;

        let new_next = aligned.checked_add(size as u64)?;
        if new_next > self.limit.as_u64() {
            return None;
        }

        let result = Vaddr::new(aligned);
        self.next = Vaddr::new(new_next);
        Some(result)
    }

    /// Allocate a contiguous region for a process's heaps.
    ///
    /// Returns `(young_base, old_base)` or `None` if insufficient space.
    ///
    /// The young heap is placed first, followed by the old heap:
    /// ```text
    /// [young_heap: young_size bytes][old_heap: old_size bytes]
    /// ```
    pub fn allocate_process_memory(
        &mut self,
        young_size: usize,
        old_size: usize,
    ) -> Option<(Vaddr, Vaddr)> {
        // Use checked arithmetic to prevent overflow
        let total = young_size.checked_add(old_size)?;
        let new_next = self.next.as_u64().checked_add(total as u64)?;

        if new_next > self.limit.as_u64() {
            return None;
        }

        let young_base = self.next;
        let old_base = self.next.add(young_size as u64);
        self.next = Vaddr::new(new_next);

        Some((young_base, old_base))
    }

    /// Allocate process memory, requesting more pages from LMM if needed.
    ///
    /// This is the recommended method for process allocation in production.
    /// It automatically grows the pool via IPC when necessary.
    ///
    /// Returns `(young_base, old_base)` or `None` if allocation fails
    /// (either LMM is out of memory or the request is invalid).
    pub fn allocate_process_memory_with_growth(
        &mut self,
        young_size: usize,
        old_size: usize,
    ) -> Option<(Vaddr, Vaddr)> {
        // First try without growing
        if let result @ Some(_) = self.allocate_process_memory(young_size, old_size) {
            return result;
        }

        // Need more memory - try to grow the pool
        let total = young_size.checked_add(old_size)?;
        if !self.try_grow(total) {
            return None;
        }

        // Try allocation again
        self.allocate_process_memory(young_size, old_size)
    }

    /// Try to grow the pool by requesting pages from the LMM.
    ///
    /// Returns `true` if growth succeeded, `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `min_bytes` - Minimum number of bytes needed
    pub fn try_grow(&mut self, min_bytes: usize) -> bool {
        // Calculate pages needed
        let min_pages = (min_bytes as u64).div_ceil(PAGE_SIZE);
        let pages_to_request = min_pages.max(MIN_GROWTH_PAGES);

        // Request pages at the current limit (contiguous growth)
        match lmm_request_pages(
            IpcRegionType::ProcessPool,
            pages_to_request as usize,
            Some(self.limit),
        ) {
            Ok(vaddr) => {
                // Verify we got pages at the expected address
                if vaddr != self.limit {
                    // LMM gave us pages at a different address - this is unexpected
                    // but we can still use them if they're contiguous
                    // For now, just fail
                    return false;
                }

                // Extend the pool
                let new_limit = self
                    .limit
                    .as_u64()
                    .saturating_add(pages_to_request * PAGE_SIZE);
                self.limit = Vaddr::new(new_limit);
                true
            }
            Err(_) => false,
        }
    }

    /// Extend the pool's limit.
    ///
    /// This is used when pages have been pre-mapped (e.g., at boot time).
    /// It does NOT allocate or map new pages.
    pub const fn extend(&mut self, additional_bytes: usize) {
        let new_limit = self.limit.as_u64().saturating_add(additional_bytes as u64);
        self.limit = Vaddr::new(new_limit);
    }
}
