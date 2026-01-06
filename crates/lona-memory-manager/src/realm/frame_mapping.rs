// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Frame allocation and segment mapping.
//!
//! This module handles allocating physical frames (4KB pages) and mapping
//! them into a realm's `VSpace`. It provides functions for:
//! - Mapping ELF segments with proper data copying
//! - Allocating zeroed frames for stack and IPC buffer
//! - Temporary frame mapping for data copy operations

#[cfg(feature = "sel4")]
use super::constants::TEMP_MAP_VADDR;
#[cfg(feature = "sel4")]
use super::page_tables::{ensure_page_tables_exist, map_frame_with_page_tables};
#[cfg(feature = "sel4")]
use super::types::RealmError;
#[cfg(feature = "sel4")]
use crate::elf::SegmentPermissions;
#[cfg(feature = "sel4")]
use crate::slots::SlotAllocator;
#[cfg(feature = "sel4")]
use crate::untyped::UntypedAllocator;
#[cfg(feature = "sel4")]
use lona_abi::layout::PAGE_SIZE;
#[cfg(feature = "sel4")]
use sel4::cap_type::{Granule, VSpace};
#[cfg(feature = "sel4")]
use sel4::{Cap, CapRights, ObjectBlueprint, VmAttributes};

/// Map a segment from the ELF file into the realm's `VSpace`.
#[cfg(feature = "sel4")]
pub fn map_segment(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    vspace: Cap<VSpace>,
    vaddr: u64,
    mem_size: u64,
    data: &[u8],
    permissions: SegmentPermissions,
) -> Result<(), RealmError> {
    let page_size = PAGE_SIZE;
    let num_pages = ((mem_size + page_size - 1) / page_size) as usize;

    for i in 0..num_pages {
        let page_vaddr = (vaddr & !(page_size - 1)) + (i as u64) * page_size;
        let page_offset = i * (page_size as usize);

        // Determine how much data to copy to this page
        let data_start = if vaddr > page_vaddr {
            0
        } else {
            page_offset.saturating_sub((vaddr - (vaddr & !(page_size - 1))) as usize)
        };
        let data_end = (data_start + page_size as usize).min(data.len());
        let page_data = if data_start < data.len() {
            &data[data_start..data_end]
        } else {
            &[]
        };

        // Allocate frame
        let frame_slot = allocate_frame(slots, untypeds)?;

        // Copy data to frame via temporary mapping in root task
        if !page_data.is_empty() {
            copy_data_to_frame(slots, untypeds, frame_slot, page_data)?;
        }

        // Ensure page tables exist and map frame
        map_frame_with_page_tables(slots, untypeds, vspace, frame_slot, page_vaddr, permissions)?;
    }

    Ok(())
}

/// Map a RW frame at the given address (for stack/IPC buffer).
#[cfg(feature = "sel4")]
pub fn map_rw_frame(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    vspace: Cap<VSpace>,
    vaddr: u64,
) -> Result<usize, RealmError> {
    let frame_slot = allocate_frame(slots, untypeds)?;

    // Zero the frame
    copy_data_to_frame(slots, untypeds, frame_slot, &[])?;

    let permissions = SegmentPermissions {
        read: true,
        write: true,
        execute: false,
    };

    map_frame_with_page_tables(slots, untypeds, vspace, frame_slot, vaddr, permissions)?;

    Ok(frame_slot)
}

/// Allocate a frame (4KB page).
#[cfg(feature = "sel4")]
fn allocate_frame(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
) -> Result<usize, RealmError> {
    let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
    #[cfg(target_arch = "aarch64")]
    let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SmallPage);
    #[cfg(target_arch = "x86_64")]
    let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::_4k);

    let (ut_slot, _, _) = untypeds
        .allocate(12, slots, false) // 4KB = 2^12
        .ok_or(RealmError::OutOfMemory)?;

    // ut_slot is an absolute slot number, use `Cap::from_bits` directly
    let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
    let cnode = sel4::init_thread::slot::CNODE.cap();

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
        .map_err(|e| {
            sel4::debug_println!("Frame retype failed: {:?}", e);
            RealmError::ObjectCreation
        })?;

    Ok(dest_slot)
}

/// Copy data to a frame via temporary mapping.
#[cfg(feature = "sel4")]
fn copy_data_to_frame(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    frame_slot: usize,
    data: &[u8],
) -> Result<(), RealmError> {
    let frame_cap: Cap<Granule> = Cap::from_bits(frame_slot as u64);
    let root_vspace = sel4::init_thread::slot::VSPACE.cap();

    // Map frame temporarily in root task's VSpace
    ensure_page_tables_exist(slots, untypeds, root_vspace, TEMP_MAP_VADDR)?;

    frame_cap
        .frame_map(
            root_vspace,
            TEMP_MAP_VADDR as usize,
            CapRights::read_write(),
            VmAttributes::default(),
        )
        .map_err(|e| {
            sel4::debug_println!("Temp frame map failed: {:?}", e);
            RealmError::MappingFailed
        })?;

    // Copy data
    // SAFETY: We just mapped this address
    unsafe {
        let dst = TEMP_MAP_VADDR as *mut u8;
        core::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        // Zero-fill the rest of the page
        let remaining = (PAGE_SIZE as usize) - data.len();
        if remaining > 0 {
            core::ptr::write_bytes(dst.add(data.len()), 0, remaining);
        }
    }

    // Unmap from root task
    frame_cap.frame_unmap().map_err(|e| {
        sel4::debug_println!("Temp frame unmap failed: {:?}", e);
        RealmError::MappingFailed
    })?;

    Ok(())
}
