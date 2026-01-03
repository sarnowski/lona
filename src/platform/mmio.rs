// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! MMIO device memory mapping for aarch64.
//!
//! On aarch64, device memory (like UART) must be explicitly mapped into the
//! virtual address space before it can be accessed. This module provides
//! functions to map device untypeds from seL4's bootinfo.
//!
//! When mapping a frame, if the page tables don't exist, we create them
//! from untyped memory. ARM64 uses up to 4 levels of page tables.

use core::cell::UnsafeCell;

use sel4::cap_type::{PT, SmallPage};
use sel4::{BootInfoPtr, Cap, CapRights, ObjectBlueprint, VmAttributes};

use crate::Vaddr;

/// Standard page size (4 KiB).
const PAGE_SIZE: usize = 4096;

/// Virtual address where MMIO device memory starts.
///
/// Positioned at 8GB to avoid overlap with other regions.
const MMIO_VADDR_START: u64 = 0x2_0000_0000;

/// State for MMIO allocation.
struct MmioState {
    /// Next virtual address for device mapping.
    next_vaddr: u64,
    /// Next slot index for capabilities.
    next_slot: usize,
}

/// Global MMIO allocator.
struct MmioAllocator {
    inner: UnsafeCell<MmioState>,
}

// SAFETY: Single-threaded access in seL4 root task.
unsafe impl Sync for MmioAllocator {}

static MMIO_ALLOCATOR: MmioAllocator = MmioAllocator {
    inner: UnsafeCell::new(MmioState {
        next_vaddr: MMIO_VADDR_START,
        next_slot: 0, // Will be initialized from bootinfo
    }),
};

/// Finds a device untyped capability containing the given physical address.
fn find_device_untyped_containing(bootinfo: &BootInfoPtr, paddr: usize) -> Option<usize> {
    let untyped_list = bootinfo.untyped_list();

    for (index, desc) in untyped_list.iter().enumerate() {
        if !desc.is_device() {
            continue;
        }

        let base = desc.paddr();
        let size = 1_usize << desc.size_bits();
        let end = base.saturating_add(size);

        if paddr >= base && paddr < end {
            return Some(index);
        }
    }

    None
}

/// Find an untyped region suitable for creating a page table or frame.
fn find_untyped_for_object(bootinfo: &BootInfoPtr, size_bits: usize) -> Option<usize> {
    let untyped_list = bootinfo.untyped_list();

    for (index, desc) in untyped_list.iter().enumerate() {
        // Skip device memory - we need regular memory for page tables
        if desc.is_device() {
            continue;
        }

        // Check if this untyped has enough space
        if desc.size_bits() >= size_bits {
            return Some(index);
        }
    }

    None
}

/// Allocate a CNode slot from bootinfo's empty region.
fn allocate_slot(bootinfo: &BootInfoPtr, state: &mut MmioState) -> Option<usize> {
    let empty = bootinfo.empty();
    let range = empty.range();

    // Initialize next_slot on first call
    if state.next_slot == 0 {
        state.next_slot = range.start;
    }

    if state.next_slot >= range.end {
        sel4::debug_println!("mmio: no more slots available");
        return None;
    }

    let slot = state.next_slot;
    state.next_slot += 1;
    Some(slot)
}

/// Create a page table from untyped memory.
fn create_page_table(bootinfo: &BootInfoPtr, state: &mut MmioState) -> Option<usize> {
    // Page tables are 4KB (12 bits)
    let untyped_index = find_untyped_for_object(bootinfo, 12)?;
    let slot = allocate_slot(bootinfo, state)?;

    let untyped = bootinfo.untyped().index(untyped_index).cap();
    let cnode = sel4::init_thread::slot::CNODE.cap();
    let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PT);

    match untyped.untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), slot, 1) {
        Ok(()) => Some(slot),
        Err(e) => {
            sel4::debug_println!("mmio: failed to create page table: {:?}", e);
            None
        }
    }
}

/// Map a page table at the given virtual address.
fn map_page_table(pt_slot: usize, vaddr: u64) -> Result<(), ()> {
    let pt_cap: Cap<PT> = Cap::from_bits(pt_slot as u64);
    let vspace = sel4::init_thread::slot::VSPACE.cap();

    match pt_cap.pt_map(vspace, vaddr as usize, VmAttributes::default()) {
        Ok(()) => Ok(()),
        Err(sel4::Error::DeleteFirst) => Ok(()), // Already exists
        Err(e) => {
            sel4::debug_println!("mmio: page table map failed: {:?}", e);
            Err(())
        }
    }
}

/// Maps a device frame at the given physical address into virtual memory.
///
/// Creates intermediate page tables as needed (ARM64 has up to 4 levels).
///
/// Returns the mapped virtual address, or `None` on failure.
///
/// # Safety
///
/// Must be called in single-threaded context with valid bootinfo.
pub unsafe fn map_device_frame(bootinfo: &BootInfoPtr, paddr: usize) -> Option<Vaddr> {
    // Verify page alignment
    if paddr % PAGE_SIZE != 0 {
        sel4::debug_println!("mmio: paddr 0x{:x} not page-aligned", paddr);
        return None;
    }

    // SAFETY: Single-threaded access
    let state = unsafe { &mut *MMIO_ALLOCATOR.inner.get() };

    // Find the device untyped containing this address
    let untyped_index = find_device_untyped_containing(bootinfo, paddr)?;

    // Allocate a slot for the device frame capability
    let frame_slot = allocate_slot(bootinfo, state)?;

    // Get virtual address for this mapping
    let vaddr = state.next_vaddr;

    // Retype the device untyped into a frame
    let untyped = bootinfo.untyped().index(untyped_index).cap();
    let cnode = sel4::init_thread::slot::CNODE.cap();
    let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SmallPage);

    if let Err(e) =
        untyped.untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), frame_slot, 1)
    {
        sel4::debug_println!("mmio: retype failed: {:?}", e);
        return None;
    }

    // Get the frame capability
    let frame_cap: Cap<SmallPage> = Cap::from_bits(frame_slot as u64);
    let vspace = sel4::init_thread::slot::VSPACE.cap();

    // Try to map the frame, creating page tables as needed (up to 4 levels)
    for _ in 0..4 {
        match frame_cap.frame_map(
            vspace,
            vaddr as usize,
            CapRights::read_write(),
            VmAttributes::default(),
        ) {
            Ok(()) => {
                // Success - update state and return
                state.next_vaddr = vaddr.saturating_add(PAGE_SIZE as u64);
                return Some(Vaddr::new(vaddr));
            }
            Err(sel4::Error::FailedLookup) => {
                // Missing page table - create and map one
                let Some(pt_slot) = create_page_table(bootinfo, state) else {
                    return None;
                };
                if map_page_table(pt_slot, vaddr).is_err() {
                    return None;
                }
                // Retry mapping the frame
            }
            Err(e) => {
                sel4::debug_println!("mmio: frame map failed: {:?}", e);
                return None;
            }
        }
    }

    sel4::debug_println!("mmio: failed after creating 4 page tables");
    None
}
