// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! ARM64-specific platform support.
//!
//! Provides PL011 UART driver and FDT-based device discovery for ARM64
//! platforms (QEMU virt, Raspberry Pi 4B).

mod mmio;
mod uart;

use core::fmt;

use sel4::BootInfoPtr;

use crate::memory::Sel4PageProvider;
use crate::platform::fdt;

/// Initializes the UART for serial output.
///
/// Discovers UART address from FDT, maps device memory, and initializes driver.
/// Returns `true` if UART was initialized successfully, `false` otherwise.
pub fn init_uart(bootinfo: &BootInfoPtr, page_provider: &Sel4PageProvider) -> bool {
    // Discover UART from FDT in bootinfo
    let uart_info = match fdt::discover_uart(bootinfo) {
        Ok(info) => {
            sel4::debug_println!(
                "Found UART at paddr 0x{:x}, size 0x{:x}",
                info.paddr,
                info.size
            );
            info
        }
        Err(err) => {
            sel4::debug_println!("Warning: UART discovery failed: {:?}", err);
            return false;
        }
    };

    // Map the UART device memory via MMIO module
    // SAFETY: page_provider is initialized, single-threaded context
    let Some(base) = (unsafe { mmio::map_device_frame(bootinfo, page_provider, uart_info.paddr) })
    else {
        sel4::debug_println!("Warning: Failed to map UART device memory");
        return false;
    };

    // Initialize the UART driver with the mapped base address
    // SAFETY: base is valid mapped MMIO pointer, single-threaded context
    let success = unsafe { uart::init(base) };

    if success {
        crate::println!("UART initialized successfully");
    } else {
        sel4::debug_println!("Warning: UART initialization failed");
    }

    success
}

/// Reads a single byte from the UART.
///
/// Blocks until a byte is available. Returns `None` if the UART is not initialized.
#[cfg(not(feature = "integration-test"))]
pub fn read_byte() -> Option<u8> {
    uart::read_byte()
}

/// Writes formatted arguments to the UART.
#[doc(hidden)]
pub fn print_fmt(args: fmt::Arguments) {
    uart::print_fmt(args);
}

/// Writes a single byte to the UART.
///
/// Used for echoing raw bytes, including UTF-8 lead and continuation bytes.
#[cfg(not(feature = "integration-test"))]
#[inline]
pub fn write_byte(byte: u8) {
    uart::write_byte(byte);
}
