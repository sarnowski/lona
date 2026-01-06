// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Process memory pool allocator.
//!
//! The `ProcessPool` is a simple bump allocator that allocates memory regions
//! for process heaps. It tracks a contiguous region of memory and hands out
//! chunks for process young and old heaps.

use crate::Vaddr;

/// Bump allocator for process memory regions.
///
/// The pool tracks a contiguous region of memory and allocates
/// chunks for process heaps using a simple bump pointer.
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
}
