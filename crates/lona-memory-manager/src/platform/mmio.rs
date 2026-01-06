// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! MMIO device memory mapping for aarch64.
//!
//! On aarch64, device memory (like UART) must be explicitly mapped into the
//! virtual address space before it can be accessed. This module provides
//! functions to find and map device untypeds from seL4's bootinfo.

use crate::slots::SlotAllocator;
use crate::untyped::UntypedAllocator;
use lona_abi::Paddr;
use sel4::cap_type::{Granule, VSpace};
use sel4::{BootInfoPtr, Cap, CapRights, ObjectBlueprint, VmAttributes};

/// Standard page size (4 KiB).
const PAGE_SIZE: usize = 4096;

/// Finds a device untyped capability containing the given physical address.
///
/// Searches through bootinfo's untyped list for a device-memory untyped
/// that contains the specified physical address.
///
/// Returns the index into bootinfo's untyped list, or `None` if not found.
pub fn find_device_untyped_containing(bootinfo: &BootInfoPtr, paddr: Paddr) -> Option<usize> {
    let paddr_val = paddr.as_u64() as usize;
    let untyped_list = bootinfo.untyped_list();

    for (index, desc) in untyped_list.iter().enumerate() {
        if !desc.is_device() {
            continue;
        }

        let base = desc.paddr();
        let size = 1_usize << desc.size_bits();
        let end = base.saturating_add(size);

        if paddr_val >= base && paddr_val < end {
            return Some(index);
        }
    }

    None
}

/// Error during device mapping.
#[derive(Debug, Clone, Copy)]
pub enum MmioError {
    /// Physical address not page-aligned.
    NotAligned,
    /// Device untyped not found for physical address.
    DeviceNotFound,
    /// No more capability slots available.
    OutOfSlots,
    /// Failed to retype untyped into frame.
    RetypeFailed,
    /// Failed to create page table.
    PageTableCreation,
    /// Failed to map frame.
    MappingFailed,
}

/// Maps a device frame at the given physical address into a VSpace.
///
/// This function:
/// 1. Finds the device untyped containing the physical address
/// 2. Retypes it into a frame capability
/// 3. Creates page tables as needed
/// 4. Maps the frame at the specified virtual address
///
/// # Arguments
///
/// * `bootinfo` - seL4 boot information
/// * `slots` - Slot allocator for capability slots
/// * `untypeds` - Untyped allocator (for creating page tables)
/// * `vspace` - Target VSpace to map into
/// * `paddr` - Physical address of the device (must be page-aligned)
/// * `vaddr` - Virtual address to map at
///
/// # Errors
///
/// Returns `MmioError` on failure.
pub fn map_device_frame(
    bootinfo: &BootInfoPtr,
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    vspace: Cap<VSpace>,
    paddr: Paddr,
    vaddr: u64,
) -> Result<usize, MmioError> {
    // Verify page alignment
    if paddr.as_u64() % PAGE_SIZE as u64 != 0 {
        return Err(MmioError::NotAligned);
    }

    // Find the device untyped containing this address
    let untyped_index =
        find_device_untyped_containing(bootinfo, paddr).ok_or(MmioError::DeviceNotFound)?;

    // Allocate a slot for the device frame capability
    let frame_slot = slots.alloc().ok_or(MmioError::OutOfSlots)?;

    // Retype the device untyped into a frame
    let untyped = bootinfo.untyped().index(untyped_index).cap();
    let cnode = sel4::init_thread::slot::CNODE.cap();
    let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SmallPage);

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), frame_slot, 1)
        .map_err(|_| MmioError::RetypeFailed)?;

    // Get the frame capability
    let frame_cap: Cap<Granule> = Cap::from_bits(frame_slot as u64);

    // Try to map the frame, creating page tables as needed (up to 4 levels)
    for _ in 0..4 {
        match frame_cap.frame_map(
            vspace,
            vaddr as usize,
            CapRights::read_write(),
            VmAttributes::default(),
        ) {
            Ok(()) => {
                return Ok(frame_slot);
            }
            Err(sel4::Error::FailedLookup) => {
                // Missing page table - create and map one
                create_and_map_page_table(slots, untypeds, vspace, vaddr)?;
            }
            Err(_) => {
                return Err(MmioError::MappingFailed);
            }
        }
    }

    Err(MmioError::MappingFailed)
}

/// Create a page table from untyped memory.
fn create_page_table(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
) -> Result<usize, MmioError> {
    let dest_slot = slots.alloc().ok_or(MmioError::OutOfSlots)?;
    let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PT);

    let (ut_slot, _, _) = untypeds
        .allocate(12, slots, false) // Page tables are 4KB
        .ok_or(MmioError::PageTableCreation)?;

    let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
    let cnode = sel4::init_thread::slot::CNODE.cap();

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
        .map_err(|_| MmioError::PageTableCreation)?;

    Ok(dest_slot)
}

/// Map a page table into VSpace.
fn map_page_table(pt_slot: usize, vspace: Cap<VSpace>, vaddr: u64) -> Result<(), MmioError> {
    use sel4::cap_type::PT;
    let pt_cap: Cap<PT> = Cap::from_bits(pt_slot as u64);

    match pt_cap.pt_map(vspace, vaddr as usize, VmAttributes::default()) {
        Ok(()) => Ok(()),
        Err(sel4::Error::DeleteFirst) => Ok(()), // Already exists
        Err(_) => Err(MmioError::MappingFailed),
    }
}

/// Create and map a page table for the given virtual address.
fn create_and_map_page_table(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    vspace: Cap<VSpace>,
    vaddr: u64,
) -> Result<(), MmioError> {
    let pt_slot = create_page_table(slots, untypeds)?;
    map_page_table(pt_slot, vspace, vaddr)
}
