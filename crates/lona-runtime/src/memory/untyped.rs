// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Untyped memory management for seL4.
//!
//! seL4 provides untyped memory capabilities at boot. These must be "retyped"
//! into specific kernel objects (frames, TCBs, endpoints, etc.) before use.
//! This module tracks available untyped memory and allocates from it.

use sel4::BootInfo;

/// Minimum size for frame allocation (4KB = 2^12 bytes).
pub const FRAME_SIZE_BITS: usize = 12;

/// Frame size in bytes.
pub const FRAME_SIZE: usize = 1 << FRAME_SIZE_BITS;

/// Tracks untyped memory regions from boot info.
///
/// Manages the allocation of frames from seL4's untyped memory pool.
/// Uses a simple sequential allocation strategy: tracks which untyped
/// regions have remaining capacity and allocates from them in order.
pub struct UntypedTracker {
    /// Index of the next untyped region to try allocating from.
    next_untyped_index: usize,
    /// Offset within the current untyped region (for large regions).
    current_offset: usize,
}

impl UntypedTracker {
    /// Creates a new untyped tracker.
    pub const fn new() -> Self {
        Self {
            next_untyped_index: 0,
            current_offset: 0,
        }
    }

    /// Finds the next untyped capability suitable for frame allocation.
    ///
    /// Returns the untyped capability index and the slot to retype into,
    /// or `None` if no suitable untyped memory remains.
    pub fn find_next_frame_untyped(&mut self, bootinfo: &BootInfo) -> Option<UntypedAllocation> {
        let untyped_list = bootinfo.untyped_list();
        let num_untypeds = untyped_list.len();

        while self.next_untyped_index < num_untypeds {
            let desc = &untyped_list[self.next_untyped_index];
            let size_bits = desc.size_bits();

            // Skip device memory (we only want regular RAM)
            if desc.is_device() {
                self.next_untyped_index += 1;
                self.current_offset = 0;
                continue;
            }

            // Check if this untyped is large enough for a frame
            if size_bits < FRAME_SIZE_BITS {
                self.next_untyped_index += 1;
                self.current_offset = 0;
                continue;
            }

            // Calculate how many frames fit in this untyped
            let untyped_size = 1usize << size_bits;
            let frames_in_untyped = untyped_size / FRAME_SIZE;
            let frames_used = self.current_offset;

            if frames_used < frames_in_untyped {
                // We can allocate from this untyped
                let allocation = UntypedAllocation {
                    untyped_index: self.next_untyped_index,
                    frame_offset: self.current_offset,
                };

                self.current_offset += 1;

                // If we've exhausted this untyped, move to the next
                if self.current_offset >= frames_in_untyped {
                    self.next_untyped_index += 1;
                    self.current_offset = 0;
                }

                return Some(allocation);
            }

            // This untyped is exhausted, try the next one
            self.next_untyped_index += 1;
            self.current_offset = 0;
        }

        // No more untyped memory available
        None
    }
}

/// Represents an allocation from untyped memory.
#[derive(Debug, Clone, Copy)]
pub struct UntypedAllocation {
    /// Index into the bootinfo untyped list.
    pub untyped_index: usize,
    /// Frame offset within this untyped region.
    #[expect(dead_code, reason = "will be used for sub-allocation tracking")]
    pub frame_offset: usize,
}
