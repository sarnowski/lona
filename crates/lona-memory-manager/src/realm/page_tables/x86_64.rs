// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! x86_64 page table creation.
//!
//! On x86_64, page tables have 3 levels below PML4:
//! - PDPT (Page Directory Pointer Table) - covers 512GB
//! - `PageDirectory` - covers 1GB
//! - `PageTable` - covers 2MB

use super::super::types::RealmError;
use crate::slots::SlotAllocator;
use crate::untyped::UntypedAllocator;
use sel4::cap_type::VSpace;
use sel4::{Cap, ObjectBlueprint, VmAttributes};

/// x86_64 page table level indicator.
#[derive(Clone, Copy, Debug)]
enum PageTableLevel {
    /// Page Directory Pointer Table (level 1, covers 512GB)
    Pdpt,
    /// Page Directory (level 2, covers 1GB)
    PageDirectory,
    /// Page Table (level 3, covers 2MB)
    PageTable,
}

/// Result of mapping a translation table.
enum MapResult {
    /// Successfully mapped a new table.
    Mapped,
    /// Table already exists at this level.
    AlreadyExists,
}

/// Create and map translation tables as needed for x86_64.
///
/// When `frame_map` fails with `FailedLookup`, any of these levels could be
/// missing. This function tries each level from top to bottom until one
/// can be mapped successfully.
///
/// # Resource Usage
///
/// If a level already exists (returns `DeleteFirst`), the allocated slot is
/// not used but the memory remains consumed. This is bounded to at most 2
/// wasted allocations per call (if only the `PageTable` level was missing).
/// During realm initialization, most addresses share upper levels, so waste
/// is minimal in practice.
///
/// # Returns
///
/// - `Ok(())` if at least one level was successfully mapped
/// - `Err(MappingFailed)` if all levels already exist
pub fn create_and_map_page_table(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    vspace: Cap<VSpace>,
    vaddr: u64,
) -> Result<(), RealmError> {
    const LEVELS: [PageTableLevel; 3] = [
        PageTableLevel::Pdpt,
        PageTableLevel::PageDirectory,
        PageTableLevel::PageTable,
    ];

    for level in LEVELS {
        let slot = create_translation_table(slots, untypeds, level)?;
        match map_translation_table(slot, vspace, vaddr, level) {
            Ok(MapResult::Mapped) => {
                // Successfully mapped this level
                return Ok(());
            }
            Ok(MapResult::AlreadyExists) => {
                // This level already exists, try next level
                continue;
            }
            Err(RealmError::MappingFailed) => {
                // Mapping failed at this level, try next
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    // All levels already exist
    Err(RealmError::MappingFailed)
}

/// Create an x86_64 translation structure at the specified level.
fn create_translation_table(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    level: PageTableLevel,
) -> Result<usize, RealmError> {
    let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
    let blueprint = match level {
        PageTableLevel::Pdpt => ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SeL4Arch(
            sel4::ObjectBlueprintX64::PDPT,
        )),
        PageTableLevel::PageDirectory => {
            ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PageDirectory)
        }
        PageTableLevel::PageTable => ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PageTable),
    };

    let size_bits = blueprint.physical_size_bits() as u8;
    let (ut_slot, _, _) = untypeds
        .allocate(size_bits, slots, false)
        .ok_or(RealmError::OutOfMemory)?;

    let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
    let cnode = sel4::init_thread::slot::CNODE.cap();

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
        .map_err(|e| {
            sel4::debug_println!("{:?} retype failed: {:?}", level, e);
            RealmError::ObjectCreation
        })?;

    Ok(dest_slot)
}

/// Map an x86_64 translation structure into `VSpace` at the specified level.
fn map_translation_table(
    slot: usize,
    vspace: Cap<VSpace>,
    vaddr: u64,
    level: PageTableLevel,
) -> Result<MapResult, RealmError> {
    let attrs = VmAttributes::default();
    let result = match level {
        PageTableLevel::Pdpt => {
            let cap: Cap<sel4::cap_type::PDPT> = Cap::from_bits(slot as u64);
            cap.pdpt_map(vspace, vaddr as usize, attrs)
        }
        PageTableLevel::PageDirectory => {
            let cap: Cap<sel4::cap_type::PageDirectory> = Cap::from_bits(slot as u64);
            cap.page_directory_map(vspace, vaddr as usize, attrs)
        }
        PageTableLevel::PageTable => {
            let cap: Cap<sel4::cap_type::PageTable> = Cap::from_bits(slot as u64);
            cap.page_table_map(vspace, vaddr as usize, attrs)
        }
    };

    match result {
        Ok(()) => Ok(MapResult::Mapped),
        Err(sel4::Error::DeleteFirst) => Ok(MapResult::AlreadyExists),
        Err(e) => {
            sel4::debug_println!("{:?} map at 0x{:x} failed: {:?}", level, vaddr, e);
            Err(RealmError::MappingFailed)
        }
    }
}
