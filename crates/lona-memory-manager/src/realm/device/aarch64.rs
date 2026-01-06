// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! aarch64 UART device mapping.
//!
//! Maps the PL011 UART device at its well-known virtual address using MMIO.

use super::super::types::RealmError;
use crate::platform::mmio;
use crate::slots::SlotAllocator;
use crate::untyped::UntypedAllocator;
use lona_abi::Paddr;
use lona_abi::layout::UART_VADDR;
use sel4::Cap;
use sel4::cap_type::VSpace;

/// Map UART device memory.
///
/// Uses the platform MMIO module to map the PL011 UART at its well-known
/// virtual address.
pub fn map_uart(
    bootinfo: &sel4::BootInfoPtr,
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
    vspace: Cap<VSpace>,
) -> Result<(), RealmError> {
    // UART physical address depends on platform (QEMU virt: 0x0900_0000)
    const UART_PADDR: Paddr = Paddr::new(0x0900_0000);

    mmio::map_device_frame(bootinfo, slots, untypeds, vspace, UART_PADDR, UART_VADDR).map_err(
        |e| {
            sel4::debug_println!("UART mapping failed: {:?}", e);
            RealmError::MappingFailed
        },
    )?;

    Ok(())
}
