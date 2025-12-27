// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! x86_64-specific platform support.
//!
//! Provides 16550 UART driver via I/O ports for `x86_64` platforms.
//! Uses the standard COM1 port at 0x3F8.
//!
//! On seL4 `x86_64`, I/O port access requires capabilities. The root task
//! receives `IOPortControl` which can issue `IOPort` capabilities for
//! specific port ranges.

mod uart;

use core::fmt;

use sel4::BootInfoPtr;

use crate::memory::Sel4PageProvider;

/// COM1 I/O port range (base 0x3F8, 8 registers).
const COM1_FIRST_PORT: u64 = 0x3F8;
const COM1_LAST_PORT: u64 = 0x3FF;

/// Initializes the UART for serial output.
///
/// On `x86_64`, uses fixed COM1 port at 0x3F8. No FDT discovery or memory
/// mapping needed - uses I/O port capabilities.
///
/// The process is:
/// 1. Get `IOPortControl` from the root task's initial capabilities
/// 2. Issue an `IOPort` capability for COM1 ports (0x3F8-0x3FF)
/// 3. Use that capability to perform I/O via seL4 syscalls
///
/// Returns `true` if UART was initialized successfully, `false` otherwise.
pub fn init_uart(_bootinfo: &BootInfoPtr, page_provider: &Sel4PageProvider) -> bool {
    sel4::debug_println!("x86_64: Initializing UART via IOPort capabilities");

    // Get the IOPortControl capability from the root task's initial slots
    let io_port_control = sel4::init_thread::slot::IO_PORT_CONTROL.cap();

    // Allocate a slot for the IOPort capability using the central slot allocator
    let Some(ioport_slot_index) = page_provider.allocate_slot() else {
        sel4::debug_println!("x86_64: No empty slots available for IOPort capability");
        return false;
    };

    sel4::debug_println!(
        "x86_64: Issuing IOPort capability for ports 0x{:x}-0x{:x} to slot {}",
        COM1_FIRST_PORT,
        COM1_LAST_PORT,
        ioport_slot_index
    );

    // Create an AbsoluteCPtr for the destination slot
    let root_cnode = sel4::init_thread::slot::CNODE.cap();
    let ioport_slot_cptr = sel4::CPtr::from_bits(u64::try_from(ioport_slot_index).unwrap_or(0));
    let dst = root_cnode.absolute_cptr(ioport_slot_cptr);

    // Issue the IOPort capability for COM1 ports
    match io_port_control.ioport_control_issue(COM1_FIRST_PORT, COM1_LAST_PORT, &dst) {
        Ok(()) => {
            sel4::debug_println!("x86_64: IOPort capability issued successfully");
        }
        Err(err) => {
            sel4::debug_println!("x86_64: Failed to issue IOPort capability: {:?}", err);
            return false;
        }
    }

    // Initialize the UART driver with the IOPort capability
    let io_port_cap = ioport_slot_cptr.bits();
    let success = uart::init(io_port_cap);

    if success {
        crate::println!("UART initialized successfully");
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
