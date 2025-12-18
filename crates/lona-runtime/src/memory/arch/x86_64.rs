// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! `x86_64` paging support for seL4.
//!
//! `x86_64` uses a 4-level page table structure with different object types
//! at each level:
//! - PML4 (level 4) - 512GB per entry (`VSpace` root, provided by kernel)
//! - PDPT (level 3) - 1GB per entry
//! - `PageDirectory` (level 2) - 2MB per entry
//! - `PageTable` (level 1) - 4KB per entry

use sel4::cap_type::{_4k as SmallPage, PDPT, PageDirectory, PageTable};
use sel4::{Cap, CapRights, ObjectBlueprint, VmAttributes};

use super::{PagingContext, create_paging_structure};

/// Returns the object blueprint for a small page (4KB frame) on `x86_64`.
pub const fn frame_blueprint() -> ObjectBlueprint {
    ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::_4k)
}

/// Returns the object blueprint for a page table (level 1).
const fn page_table_blueprint() -> ObjectBlueprint {
    ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PageTable)
}

/// Returns the object blueprint for a page directory (level 2).
const fn page_directory_blueprint() -> ObjectBlueprint {
    ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PageDirectory)
}

/// Returns the object blueprint for a PDPT (level 3).
const fn pdpt_blueprint() -> ObjectBlueprint {
    ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SeL4Arch(
        sel4::ObjectBlueprintX64::PDPT,
    ))
}

/// Creates a page table from untyped memory.
fn create_page_table(ctx: &mut PagingContext<'_>) -> Option<usize> {
    create_paging_structure(ctx, page_table_blueprint(), "page table")
}

/// Creates a page directory from untyped memory.
fn create_page_directory(ctx: &mut PagingContext<'_>) -> Option<usize> {
    create_paging_structure(ctx, page_directory_blueprint(), "page directory")
}

/// Creates a PDPT from untyped memory.
fn create_pdpt(ctx: &mut PagingContext<'_>) -> Option<usize> {
    create_paging_structure(ctx, pdpt_blueprint(), "PDPT")
}

/// Maps a page table at the given virtual address.
///
/// Returns `Ok(true)` if mapped, `Ok(false)` if already exists,
/// `Err(true)` if needs higher-level table, `Err(false)` on other errors.
fn map_page_table(slot: usize, vaddr: usize) -> Result<bool, bool> {
    let cap: Cap<PageTable> = Cap::from_bits(u64::try_from(slot).unwrap_or(0));
    let vspace = sel4::init_thread::slot::VSPACE.cap();

    match cap.page_table_map(vspace, vaddr, VmAttributes::default()) {
        Ok(()) => {
            sel4::debug_println!("Mapped page table at 0x{:x}", vaddr);
            Ok(true)
        }
        Err(sel4::Error::DeleteFirst) => Ok(false),
        Err(sel4::Error::FailedLookup) => Err(true),
        Err(err) => {
            sel4::debug_println!("Page table map failed at 0x{:x}: {:?}", vaddr, err);
            Err(false)
        }
    }
}

/// Maps a page directory at the given virtual address.
///
/// Returns `Ok(true)` if mapped, `Ok(false)` if already exists,
/// `Err(true)` if needs higher-level table, `Err(false)` on other errors.
fn map_page_directory(slot: usize, vaddr: usize) -> Result<bool, bool> {
    let cap: Cap<PageDirectory> = Cap::from_bits(u64::try_from(slot).unwrap_or(0));
    let vspace = sel4::init_thread::slot::VSPACE.cap();

    match cap.page_directory_map(vspace, vaddr, VmAttributes::default()) {
        Ok(()) => {
            sel4::debug_println!("Mapped page directory at 0x{:x}", vaddr);
            Ok(true)
        }
        Err(sel4::Error::DeleteFirst) => Ok(false),
        Err(sel4::Error::FailedLookup) => Err(true),
        Err(err) => {
            sel4::debug_println!("Page directory map failed at 0x{:x}: {:?}", vaddr, err);
            Err(false)
        }
    }
}

/// Maps a PDPT at the given virtual address.
///
/// Returns `Ok(true)` if mapped, `Ok(false)` if already exists, `Err(())` on error.
fn map_pdpt(slot: usize, vaddr: usize) -> Result<bool, ()> {
    let cap: Cap<PDPT> = Cap::from_bits(u64::try_from(slot).unwrap_or(0));
    let vspace = sel4::init_thread::slot::VSPACE.cap();

    match cap.pdpt_map(vspace, vaddr, VmAttributes::default()) {
        Ok(()) => {
            sel4::debug_println!("Mapped PDPT at 0x{:x}", vaddr);
            Ok(true)
        }
        Err(sel4::Error::DeleteFirst) => Ok(false),
        Err(err) => {
            sel4::debug_println!("PDPT map failed at 0x{:x}: {:?}", vaddr, err);
            Err(())
        }
    }
}

/// Ensures all required page table structures exist for an address.
///
/// Creates PDPT, `PageDirectory`, and `PageTable` as needed by working from
/// the lowest level up and creating higher levels when required.
fn ensure_page_tables(ctx: &mut PagingContext<'_>, vaddr: usize) -> bool {
    // Try to create and map a PageTable first
    let Some(page_table_slot) = create_page_table(ctx) else {
        sel4::debug_println!("Failed to create page table");
        return false;
    };

    match map_page_table(page_table_slot, vaddr) {
        Ok(_mapped) => return true,
        Err(true) => {
            // Needs PageDirectory - continue to create that
        }
        Err(false) => return false,
    }

    // PageTable mapping failed because we need a PageDirectory
    let Some(page_dir_slot) = create_page_directory(ctx) else {
        sel4::debug_println!("Failed to create page directory");
        return false;
    };

    match map_page_directory(page_dir_slot, vaddr) {
        Ok(_mapped) => {
            // PageDirectory mapped, now try PageTable again
            return map_page_table(page_table_slot, vaddr).is_ok();
        }
        Err(true) => {
            // Needs PDPT - continue to create that
        }
        Err(false) => return false,
    }

    // PageDirectory mapping failed because we need a PDPT
    let Some(pdpt_slot) = create_pdpt(ctx) else {
        sel4::debug_println!("Failed to create PDPT");
        return false;
    };

    if map_pdpt(pdpt_slot, vaddr).is_err() {
        return false;
    }

    // PDPT mapped, now map PageDirectory
    if map_page_directory(page_dir_slot, vaddr).is_err() {
        return false;
    }

    // PageDirectory mapped, now map PageTable
    map_page_table(page_table_slot, vaddr).is_ok()
}

/// Maps a frame at the given virtual address, creating page tables as needed.
///
/// On `x86_64`, we may need to create PDPT, `PageDirectory`, and `PageTable`
/// structures before we can map a frame. This function handles the entire
/// hierarchy.
///
/// Returns `true` if the frame was mapped successfully, `false` on failure.
pub fn map_frame_with_page_tables(
    ctx: &mut PagingContext<'_>,
    frame_slot: usize,
    vaddr: usize,
) -> bool {
    let vspace = sel4::init_thread::slot::VSPACE.cap();
    let frame_cap: Cap<SmallPage> = Cap::from_bits(u64::try_from(frame_slot).unwrap_or(0));

    // x86_64 has 4 levels: PML4 (root), PDPT, PageDirectory, PageTable
    // We try mapping the frame and create intermediate structures as needed.
    for _ in 0_u8..4_u8 {
        let result = frame_cap.frame_map(
            vspace,
            vaddr,
            CapRights::read_write(),
            VmAttributes::default(),
        );

        match result {
            Ok(()) => return true,
            Err(sel4::Error::FailedLookup) => {
                // Missing page table structure - create from lowest level up
                if !ensure_page_tables(ctx, vaddr) {
                    return false;
                }
            }
            Err(err) => {
                sel4::debug_println!("Frame map failed at 0x{:x}: {:?}", vaddr, err);
                return false;
            }
        }
    }

    sel4::debug_println!(
        "Failed to map frame after creating page table structures at 0x{:x}",
        vaddr
    );
    false
}
