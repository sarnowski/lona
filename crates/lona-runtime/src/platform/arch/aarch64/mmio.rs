// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! MMIO device memory mapping for ARM64.
//!
//! Handles mapping device memory (like UART) into virtual address space.
//! This is ARM64-specific because `x86_64` uses I/O ports for devices.

use core::cell::UnsafeCell;

use sel4::BootInfo;

use crate::memory::{FRAME_SIZE, Sel4PageProvider};

/// Virtual address where MMIO device memory starts.
///
/// Positioned at 8GB to avoid overlap with heap region (4GB-8GB).
const MMIO_VADDR_START: usize = 0x2_0000_0000;

/// State for MMIO virtual address allocation.
struct MmioState {
    /// Next virtual address for device mapping.
    next_vaddr: usize,
}

/// Global MMIO state.
struct MmioAllocator {
    inner: UnsafeCell<MmioState>,
}

// SAFETY: Single-threaded access in seL4 root task - no concurrent access.
// Only Sync is needed for static variables (shared access), not Send (transfer).
unsafe impl Sync for MmioAllocator {}

static MMIO_STATE: MmioAllocator = MmioAllocator {
    inner: UnsafeCell::new(MmioState {
        next_vaddr: MMIO_VADDR_START,
    }),
};

/// Finds a device untyped capability containing the given physical address.
///
/// Device untypeds represent MMIO regions (UART, network controllers, etc).
/// Searches bootinfo for a device untyped that contains the specified
/// physical address.
///
/// Returns the index into bootinfo's untyped list, or `None` if not found.
fn find_device_untyped_containing(bootinfo: &BootInfo, paddr: usize) -> Option<usize> {
    let untyped_list = bootinfo.untyped_list();

    for (index, desc) in untyped_list.iter().enumerate() {
        // Only consider device memory
        if !desc.is_device() {
            continue;
        }

        let base = desc.paddr();
        let size = 1_usize << desc.size_bits();
        let end = base.saturating_add(size);

        // Check if paddr falls within this region
        if paddr >= base && paddr < end {
            return Some(index);
        }
    }

    None
}

/// Maps a device frame at the given physical address into virtual memory.
///
/// Device frames are memory-mapped I/O regions for hardware like UARTs.
/// Unlike regular memory, device untypeds are pre-existing and just need
/// to be retyped and mapped.
///
/// Returns a pointer to the mapped virtual address, or `None` on failure.
///
/// # Safety
///
/// - The page provider must be initialized
/// - Must be called in single-threaded context
pub unsafe fn map_device_frame(
    bootinfo: &BootInfo,
    page_provider: &Sel4PageProvider,
    paddr: usize,
) -> Option<*mut u8> {
    use crate::memory::arch::frame_blueprint;

    // Verify the physical address is page-aligned
    if !paddr.is_multiple_of(FRAME_SIZE) {
        sel4::debug_println!("map_device_frame: paddr 0x{:x} is not page-aligned", paddr);
        return None;
    }

    // SAFETY: Single-threaded access
    let state = unsafe { &mut *MMIO_STATE.inner.get() };

    // Find the device untyped containing this physical address
    let untyped_index = find_device_untyped_containing(bootinfo, paddr)?;

    // Allocate a CNode slot for the device frame capability
    let slot = page_provider.allocate_slot()?;

    // Get the device untyped capability
    let untyped = bootinfo.untyped().index(untyped_index).cap();
    let cnode = sel4::init_thread::slot::CNODE.cap();

    // Retype into a page frame
    let blueprint = frame_blueprint();
    if let Err(err) = untyped.untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), slot, 1) {
        sel4::debug_println!("Failed to retype device untyped: {:?}", err);
        return None;
    }

    // Get virtual address for this device mapping
    let vaddr = state.next_vaddr;

    // Map the frame with page tables as needed
    // SAFETY: page_provider is initialized, slot contains valid frame capability
    if !unsafe { page_provider.map_frame(bootinfo, slot, vaddr) } {
        sel4::debug_println!("Failed to map device frame at 0x{:x}", vaddr);
        return None;
    }

    // Update state for next device mapping
    state.next_vaddr = vaddr.saturating_add(FRAME_SIZE);

    sel4::debug_println!(
        "Mapped device frame at paddr 0x{:x} to vaddr 0x{:x}",
        paddr,
        vaddr
    );

    #[expect(
        clippy::as_conversions,
        reason = "[approved] usize to pointer is required for MMIO access"
    )]
    Some(vaddr as *mut u8)
}
