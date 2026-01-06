// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! ARM aarch64 page table creation.
//!
//! On ARM, a single PT (Page Table) object type works at all intermediate
//! levels between the `VSpace` (root page table) and the frame mapping.

use super::super::types::RealmError;
use crate::slots::SlotAllocator;
use crate::untyped::UntypedAllocator;
use sel4::cap_type::{PT, VSpace};
use sel4::{Cap, ObjectBlueprint, VmAttributes};

/// Create and map a page table (ARM version).
pub fn create_and_map_page_table(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    vspace: Cap<VSpace>,
    vaddr: u64,
) -> Result<(), RealmError> {
    let pt_slot = create_page_table(slots, untypeds)?;
    map_page_table(pt_slot, vspace, vaddr)?;
    Ok(())
}

/// Create a page table object.
fn create_page_table(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
) -> Result<usize, RealmError> {
    let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
    let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PT);

    let (ut_slot, _, _) = untypeds
        .allocate(12, slots, false) // Page tables are 4KB
        .ok_or(RealmError::OutOfMemory)?;

    let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
    let cnode = sel4::init_thread::slot::CNODE.cap();

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
        .map_err(|e| {
            sel4::debug_println!("Page table retype failed: {:?}", e);
            RealmError::ObjectCreation
        })?;

    Ok(dest_slot)
}

/// Map a page table into `VSpace`.
fn map_page_table(pt_slot: usize, vspace: Cap<VSpace>, vaddr: u64) -> Result<(), RealmError> {
    let pt_cap: Cap<PT> = Cap::from_bits(pt_slot as u64);

    match pt_cap.pt_map(vspace, vaddr as usize, VmAttributes::default()) {
        Ok(()) => Ok(()),
        Err(sel4::Error::DeleteFirst) => Ok(()), // Already exists
        Err(e) => {
            sel4::debug_println!("Page table map failed: {:?}", e);
            Err(RealmError::MappingFailed)
        }
    }
}
