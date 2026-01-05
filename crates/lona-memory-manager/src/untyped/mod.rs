// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Untyped memory allocator.
//!
//! Manages allocation of kernel objects from seL4 untyped capabilities.
//! The root task receives untyped capabilities representing all free
//! physical memory at boot time.

use crate::slots::SlotAllocator;

/// Maximum number of untyped capabilities we track.
const MAX_UNTYPEDS: usize = 256;

/// Descriptor for a single untyped capability.
#[derive(Clone, Copy, Debug)]
pub struct UntypedDesc {
    /// Capability slot containing the untyped.
    pub slot: usize,
    /// Physical address of the untyped region.
    pub paddr: u64,
    /// Size of the region in bits (size = 1 << `size_bits`).
    pub size_bits: u8,
    /// Whether this is device memory (uncached).
    pub is_device: bool,
    /// Current allocation watermark (offset from start).
    pub watermark: u64,
}

impl UntypedDesc {
    /// Returns the total size of this untyped region.
    #[must_use]
    pub const fn size(&self) -> u64 {
        1 << self.size_bits
    }

    /// Returns the remaining unallocated space.
    #[must_use]
    pub const fn remaining(&self) -> u64 {
        self.size().saturating_sub(self.watermark)
    }

    /// Checks if this untyped can satisfy an allocation of the given size.
    #[must_use]
    pub const fn can_allocate(&self, size_bits: u8) -> bool {
        let obj_size = 1u64 << size_bits;
        // Alignment: object must be aligned to its size
        let aligned_watermark = (self.watermark + obj_size - 1) & !(obj_size - 1);
        aligned_watermark + obj_size <= self.size()
    }

    /// Allocates space for an object, updating the watermark.
    ///
    /// Returns the physical address of the allocated space.
    pub const fn allocate(&mut self, size_bits: u8) -> Option<u64> {
        let obj_size = 1u64 << size_bits;
        // Align watermark to object size
        let aligned = (self.watermark + obj_size - 1) & !(obj_size - 1);
        if aligned + obj_size > self.size() {
            return None;
        }
        let paddr = self.paddr + aligned;
        self.watermark = aligned + obj_size;
        Some(paddr)
    }
}

/// Allocator for untyped memory.
///
/// Tracks available untyped capabilities and allocates kernel objects
/// from them using seL4's retype operation.
pub struct UntypedAllocator {
    /// Untyped descriptors, sorted by size (largest first).
    untypeds: [Option<UntypedDesc>; MAX_UNTYPEDS],
    /// Number of valid entries.
    count: usize,
}

impl UntypedAllocator {
    /// Creates a new empty untyped allocator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            untypeds: [None; MAX_UNTYPEDS],
            count: 0,
        }
    }

    /// Adds an untyped capability to the allocator.
    ///
    /// # Returns
    ///
    /// `true` if added successfully, `false` if full.
    pub const fn add(&mut self, desc: UntypedDesc) -> bool {
        if self.count >= MAX_UNTYPEDS {
            return false;
        }
        self.untypeds[self.count] = Some(desc);
        self.count += 1;
        true
    }

    /// Creates an untyped allocator from seL4 bootinfo.
    #[cfg(feature = "sel4")]
    #[must_use]
    pub fn from_bootinfo(bootinfo: &sel4::BootInfoPtr) -> Self {
        let mut alloc = Self::new();

        for (i, desc) in bootinfo.untyped_list().iter().enumerate() {
            let slot = bootinfo.untyped().start() + i;
            let untyped_desc = UntypedDesc {
                slot,
                paddr: desc.paddr() as u64,
                size_bits: desc.size_bits() as u8,
                is_device: desc.is_device(),
                watermark: 0,
            };
            if !alloc.add(untyped_desc) {
                break; // Full
            }
        }

        // Sort by size (largest first) for best-fit allocation
        alloc.sort_by_size();
        alloc
    }

    /// Sorts untypeds by size (largest first).
    #[cfg(any(test, feature = "sel4"))]
    fn sort_by_size(&mut self) {
        // Simple insertion sort - MAX_UNTYPEDS is small
        for i in 1..self.count {
            let mut j = i;
            while j > 0 {
                let curr_bits = self.untypeds[j].map_or(0, |u| u.size_bits);
                let prev_bits = self.untypeds[j - 1].map_or(0, |u| u.size_bits);
                if curr_bits > prev_bits {
                    self.untypeds.swap(j, j - 1);
                    j -= 1;
                } else {
                    break;
                }
            }
        }
    }

    /// Finds an untyped that can satisfy an allocation of the given size.
    ///
    /// Returns the index into the untypeds array.
    fn find_fit(&self, size_bits: u8, device: bool) -> Option<usize> {
        for i in 0..self.count {
            if let Some(desc) = &self.untypeds[i] {
                if desc.is_device == device && desc.can_allocate(size_bits) {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Allocates an object of the given type.
    ///
    /// # Arguments
    ///
    /// * `size_bits` - Size of the object in bits (size = 1 << `size_bits`)
    /// * `slots` - Slot allocator for the destination slot
    /// * `device` - Whether to allocate from device memory
    ///
    /// # Returns
    ///
    /// The (untyped slot, destination slot, physical address) or `None`.
    pub fn allocate(
        &mut self,
        size_bits: u8,
        slots: &mut SlotAllocator,
        device: bool,
    ) -> Option<(usize, usize, u64)> {
        let idx = self.find_fit(size_bits, device)?;
        let dest_slot = slots.alloc()?;

        let desc = self.untypeds[idx].as_mut()?;
        let paddr = desc.allocate(size_bits)?;

        Some((desc.slot, dest_slot, paddr))
    }

    /// Returns the total remaining memory in non-device untypeds.
    #[must_use]
    pub fn total_free(&self) -> u64 {
        let mut total = 0u64;
        for i in 0..self.count {
            if let Some(desc) = &self.untypeds[i] {
                if !desc.is_device {
                    total = total.saturating_add(desc.remaining());
                }
            }
        }
        total
    }
}

impl Default for UntypedAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod untyped_test;
