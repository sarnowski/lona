// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! ARM64 paging support for seL4.
//!
//! ARM64 uses a 4-level page table structure where all intermediate levels
//! use the same PT (Page Table) object type. This simplifies the paging
//! logic compared to `x86_64`.

use sel4::cap_type::{PT, SmallPage};
use sel4::{Cap, CapRights, ObjectBlueprint, VmAttributes};

use super::{PagingContext, create_paging_structure};

/// Returns the object blueprint for a small page (4KB frame) on ARM64.
pub const fn frame_blueprint() -> ObjectBlueprint {
    ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SmallPage)
}

/// Returns the object blueprint for a page table on ARM64.
const fn page_table_blueprint() -> ObjectBlueprint {
    ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PT)
}

/// Creates a page table from untyped memory.
fn create_page_table(ctx: &mut PagingContext<'_>) -> Option<usize> {
    create_paging_structure(ctx, page_table_blueprint(), "page table")
}

/// Maps a page table at the given virtual address.
///
/// Returns `Ok(true)` if mapped successfully, `Ok(false)` if already exists,
/// `Err(())` on other errors.
fn map_page_table(pt_slot: usize, vaddr: usize) -> Result<bool, ()> {
    let pt_cap: Cap<PT> = Cap::from_bits(u64::try_from(pt_slot).unwrap_or(0));
    let vspace = sel4::init_thread::slot::VSPACE.cap();

    match pt_cap.pt_map(vspace, vaddr, VmAttributes::default()) {
        Ok(()) => {
            sel4::debug_println!("Mapped page table at 0x{:x}", vaddr);
            Ok(true)
        }
        Err(sel4::Error::DeleteFirst) => Ok(false),
        Err(err) => {
            sel4::debug_println!("Page table map failed at 0x{:x}: {:?}", vaddr, err);
            Err(())
        }
    }
}

/// Maps a frame at the given virtual address, creating page tables as needed.
///
/// ARM64 has up to 4 levels of page tables. This function tries mapping the
/// frame, and if it fails due to missing page tables, creates them one at a
/// time until the mapping succeeds.
///
/// Returns `true` if the frame was mapped successfully, `false` on failure.
pub fn map_frame_with_page_tables(
    ctx: &mut PagingContext<'_>,
    frame_slot: usize,
    vaddr: usize,
) -> bool {
    let vspace = sel4::init_thread::slot::VSPACE.cap();
    let frame_cap: Cap<SmallPage> = Cap::from_bits(u64::try_from(frame_slot).unwrap_or(0));

    // ARM64 has up to 4 levels of page tables. We try mapping the frame,
    // and if it fails due to missing page tables, we create them one at a time.
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
                // Missing page table - create and map one
                let Some(pt_slot) = create_page_table(ctx) else {
                    sel4::debug_println!("Failed to create page table");
                    return false;
                };

                if map_page_table(pt_slot, vaddr).is_err() {
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
        "Failed to map frame after creating 4 page tables at 0x{:x}",
        vaddr
    );
    false
}
