// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Capability slot allocator.
//!
//! Manages allocation of capability slots in the root task's `CNode`.
//! seL4 requires that capabilities be stored in specific slots; this
//! module tracks which slots are available for new capabilities.

/// Allocator for capability slots in the root `CNode`.
///
/// The root task starts with a range of empty slots (from bootinfo).
/// This allocator hands them out sequentially.
pub struct SlotAllocator {
    /// Next slot to allocate.
    next: usize,
    /// One past the last valid slot.
    end: usize,
}

impl SlotAllocator {
    /// Creates a new slot allocator for the given range.
    ///
    /// # Arguments
    ///
    /// * `start` - First available slot
    /// * `end` - One past the last available slot
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { next: start, end }
    }

    /// Creates a slot allocator from seL4 bootinfo.
    #[cfg(feature = "sel4")]
    #[must_use]
    pub fn from_bootinfo(bootinfo: &sel4::BootInfoPtr) -> Self {
        let empty = bootinfo.empty();
        Self::new(empty.start(), empty.end())
    }

    /// Allocates a single capability slot.
    ///
    /// # Returns
    ///
    /// The slot index, or `None` if no slots remain.
    pub const fn alloc(&mut self) -> Option<usize> {
        if self.next < self.end {
            let slot = self.next;
            self.next += 1;
            Some(slot)
        } else {
            None
        }
    }

    /// Allocates a contiguous range of capability slots.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of slots to allocate
    ///
    /// # Returns
    ///
    /// The first slot index, or `None` if not enough slots remain.
    pub fn alloc_range(&mut self, count: usize) -> Option<usize> {
        if count == 0 {
            return Some(self.next);
        }
        let start = self.next;
        let new_next = start.checked_add(count)?;
        if new_next <= self.end {
            self.next = new_next;
            Some(start)
        } else {
            None
        }
    }

    /// Returns the number of remaining slots.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.end.saturating_sub(self.next)
    }

    /// Returns true if no slots remain.
    #[must_use]
    pub const fn is_exhausted(&self) -> bool {
        self.next >= self.end
    }
}

#[cfg(test)]
mod slots_test;
