// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Page table creation and management.
//!
//! This module handles creating intermediate page table structures needed
//! to map frames into a `VSpace`. It provides architecture-specific
//! implementations for aarch64 and `x86_64`.

#[cfg(all(feature = "sel4", target_arch = "aarch64"))]
mod aarch64;
#[cfg(all(feature = "sel4", target_arch = "x86_64"))]
mod x86_64;

#[cfg(feature = "sel4")]
use super::types::RealmError;
#[cfg(feature = "sel4")]
use crate::elf::SegmentPermissions;
#[cfg(feature = "sel4")]
use crate::slots::SlotAllocator;
#[cfg(feature = "sel4")]
use crate::untyped::UntypedAllocator;
#[cfg(feature = "sel4")]
use sel4::cap_type::{Granule, VSpace};
#[cfg(feature = "sel4")]
use sel4::{Cap, CapRights, VmAttributes};

#[cfg(all(feature = "sel4", target_arch = "aarch64"))]
use aarch64::create_and_map_page_table;
#[cfg(all(feature = "sel4", target_arch = "x86_64"))]
use x86_64::create_and_map_page_table;

/// Map a frame into `VSpace`, creating page tables as needed.
///
/// Enforces W^X policy:
/// - RX (code): read-only rights, execute allowed
/// - RW (data/BSS): read-write rights, execute-never
/// - RO (rodata): read-only rights, execute-never
#[cfg(feature = "sel4")]
pub fn map_frame_with_page_tables(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    vspace: Cap<VSpace>,
    frame_slot: usize,
    vaddr: u64,
    permissions: SegmentPermissions,
) -> Result<(), RealmError> {
    let frame_cap: Cap<Granule> = Cap::from_bits(frame_slot as u64);

    // Determine capability rights based on permissions
    let rights = if permissions.write {
        CapRights::read_write()
    } else {
        CapRights::read_only()
    };

    // Set execute-never for non-executable segments (W^X enforcement)
    // aarch64: Use EXECUTE_NEVER attribute to prevent execution
    // x86_64: seL4 on x86_64 doesn't expose NX bit via VmAttributes; pages are
    // executable by default. Full W^X on x86_64 requires kernel configuration.
    #[cfg(target_arch = "aarch64")]
    let attrs = if permissions.execute {
        VmAttributes::default()
    } else {
        VmAttributes::EXECUTE_NEVER
    };
    #[cfg(target_arch = "x86_64")]
    let attrs = VmAttributes::default();

    // Try mapping, creating page tables as needed (up to 4 levels on aarch64)
    for _ in 0..4 {
        match frame_cap.frame_map(vspace, vaddr as usize, rights.clone(), attrs) {
            Ok(()) => return Ok(()),
            Err(sel4::Error::FailedLookup) => {
                // Missing page table - create and map one
                create_and_map_page_table(slots, untypeds, vspace, vaddr)?;
            }
            Err(e) => {
                sel4::debug_println!("Frame map at 0x{:x} failed: {:?}", vaddr, e);
                return Err(RealmError::MappingFailed);
            }
        }
    }

    sel4::debug_println!("Failed to map frame after creating 4 page tables");
    Err(RealmError::MappingFailed)
}

/// Ensure all page table levels exist for a virtual address.
///
/// This function creates page tables iteratively until all required levels
/// exist for the given virtual address. It calls `create_and_map_page_table`
/// repeatedly until that function returns `MappingFailed`, which signals
/// that no more levels need to be created.
///
/// # Architecture Details
///
/// - **aarch64**: Up to 3 levels (L1, L2, L3 tables)
/// - **x86_64**: Up to 3 levels (PDPT, `PageDirectory`, `PageTable`)
///
/// The loop runs up to 4 times to handle all possible levels plus one
/// final check.
#[cfg(feature = "sel4")]
pub fn ensure_page_tables_exist(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    vspace: Cap<VSpace>,
    vaddr: u64,
) -> Result<(), RealmError> {
    for _ in 0..4 {
        match create_and_map_page_table(slots, untypeds, vspace, vaddr) {
            Ok(()) => continue,                      // Created one level, might need more
            Err(RealmError::MappingFailed) => break, // All levels exist
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
