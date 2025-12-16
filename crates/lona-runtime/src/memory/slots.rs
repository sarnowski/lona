// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! `CNode` slot allocation for seL4 capabilities.
//!
//! seL4 requires `CNode` slots for storing new capabilities. This module
//! provides a simple sequential allocator that tracks available slots
//! from the boot info's empty slot range.

use sel4::BootInfo;

/// Allocates `CNode` slots from the empty slot range provided by boot info.
///
/// New capabilities (frames, page tables, etc.) need to be placed in
/// `CNode` slots. The boot info provides a range of pre-allocated empty
/// slots that we can use.
pub struct SlotAllocator {
    /// Start of the empty slot range (set once during init).
    start: usize,
    /// Next slot index to allocate.
    next: usize,
    /// End of the empty slot range (exclusive).
    end: usize,
}

impl SlotAllocator {
    /// Creates a new uninitialized slot allocator.
    pub const fn new() -> Self {
        Self {
            start: 0,
            next: 0,
            end: 0,
        }
    }

    /// Initializes the slot allocator from boot info.
    ///
    /// Must be called before any slot allocations.
    pub fn init(&mut self, bootinfo: &BootInfo) {
        let empty_range = bootinfo.empty().range();
        self.start = empty_range.start;
        self.next = empty_range.start;
        self.end = empty_range.end;
    }

    /// Allocates a single `CNode` slot.
    ///
    /// Returns the slot index, or `None` if no slots remain.
    pub const fn allocate(&mut self) -> Option<usize> {
        if self.next >= self.end {
            return None;
        }

        let slot = self.next;
        self.next = self.next.saturating_add(1);
        Some(slot)
    }

    /// Returns the number of slots remaining.
    #[expect(dead_code, reason = "useful for debugging and future diagnostics")]
    pub const fn remaining(&self) -> usize {
        self.end.saturating_sub(self.next)
    }

    /// Returns the number of slots allocated so far.
    #[expect(dead_code, reason = "useful for debugging and future diagnostics")]
    pub const fn allocated(&self) -> usize {
        self.next.saturating_sub(self.start)
    }
}
