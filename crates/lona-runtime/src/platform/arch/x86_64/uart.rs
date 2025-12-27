// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! 16550 UART driver for `x86_64` platforms.
//!
//! Provides serial I/O via I/O ports for `x86_64` platforms.
//! Uses the standard COM1 port at 0x3F8.
//!
//! On seL4, direct port I/O is forbidden in userspace. Instead, we use
//! seL4 `IOPort` capabilities to perform I/O through system calls.

use core::cell::UnsafeCell;
use core::fmt::{self, Write};

use sel4::sys::seL4_CPtr;

/// COM1 I/O port base address.
const COM1_PORT: u16 = 0x3F8;

// 16550 UART register offsets from base port
/// Data register (read/write).
const DATA_REG: u16 = 0;
/// Line Status Register - contains TX ready bit.
const LSR_REG: u16 = 5;

/// Line Status Register: Transmitter Holding Register Empty.
const LSR_THRE: u8 = 0x20;
/// Line Status Register: Data Ready bit.
#[cfg(not(feature = "integration-test"))]
const LSR_DR: u8 = 0x01;

/// 16550 UART driver for `x86_64` serial I/O.
///
/// Uses seL4 I/O port capability to communicate with the UART hardware.
struct Uart16550 {
    /// Base I/O port address.
    port: u16,
    /// Capability pointer to access I/O ports via seL4 syscalls.
    io_port_cap: seL4_CPtr,
}

impl Uart16550 {
    /// Creates a new UART driver for the given I/O port.
    const fn new(port: u16, io_port_cap: seL4_CPtr) -> Self {
        Self { port, io_port_cap }
    }

    /// Reads a byte from the specified register offset.
    fn inb(&self, offset: u16) -> u8 {
        // Calculate the actual port number
        let port = self.port.checked_add(offset).unwrap_or(self.port);

        // Use seL4 IOPort capability to read from the port
        sel4::with_ipc_buffer_mut(|ipc_buffer| {
            let result = ipc_buffer
                .inner_mut()
                .seL4_X86_IOPort_In8(self.io_port_cap, port);
            if result.error == 0 {
                result.result
            } else {
                0xFF
            }
        })
    }

    /// Writes a byte to the specified register offset.
    fn outb(&self, offset: u16, value: u8) {
        // Calculate the actual port number
        let port = self.port.checked_add(offset).unwrap_or(self.port);

        // Use seL4 IOPort capability to write to the port
        sel4::with_ipc_buffer_mut(|ipc_buffer| {
            let _: sel4::sys::seL4_Error::Type = ipc_buffer.inner_mut().seL4_X86_IOPort_Out8(
                self.io_port_cap,
                u64::from(port),
                u64::from(value),
            );
        });
    }

    /// Waits until the transmit holding register is empty.
    fn wait_tx_ready(&self) {
        while (self.inb(LSR_REG) & LSR_THRE) == 0 {
            core::hint::spin_loop();
        }
    }

    /// Writes a single byte to the UART.
    fn write_byte(&self, byte: u8) {
        self.wait_tx_ready();
        self.outb(DATA_REG, byte);
    }

    /// Checks if data is available to read.
    #[cfg(not(feature = "integration-test"))]
    fn data_ready(&self) -> bool {
        (self.inb(LSR_REG) & LSR_DR) != 0
    }

    /// Reads a single byte from the UART, blocking until available.
    #[cfg(not(feature = "integration-test"))]
    fn read_byte(&self) -> u8 {
        while !self.data_ready() {
            core::hint::spin_loop();
        }
        self.inb(DATA_REG)
    }
}

/// Global UART driver instance.
struct Driver {
    inner: UnsafeCell<Option<Uart16550>>,
}

// SAFETY: Single-threaded access in seL4 root task - no concurrent access.
// Only Sync is needed for static variables (shared access), not Send (transfer).
unsafe impl Sync for Driver {}

static UART_DRIVER: Driver = Driver {
    inner: UnsafeCell::new(None),
};

/// Initializes the UART for serial output.
///
/// Uses fixed COM1 port at 0x3F8.
///
/// Returns `true` if UART was initialized successfully, `false` if the
/// UART does not appear to be present (reads 0xFF from status register).
pub fn init(io_port_cap: seL4_CPtr) -> bool {
    sel4::debug_println!("Initializing x86_64 UART (COM1 at 0x{:x})", COM1_PORT);

    let uart = Uart16550::new(COM1_PORT, io_port_cap);

    // Verify UART is present by checking Line Status Register
    // A non-existent port typically returns 0xFF
    let lsr = uart.inb(LSR_REG);
    if lsr == 0xFF {
        sel4::debug_println!("UART not detected at COM1 (LSR=0xFF)");
        return false;
    }

    sel4::debug_println!("UART detected at COM1 (LSR=0x{:02x})", lsr);

    // SAFETY: Single-threaded initialization
    unsafe {
        *UART_DRIVER.inner.get() = Some(uart);
    }

    true
}

/// Writer that outputs to the UART.
pub struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // SAFETY: Accessing global state in single-threaded context
        let driver = unsafe { &*UART_DRIVER.inner.get() };
        if let Some(ref uart) = *driver {
            for byte in s.bytes() {
                // Convert \n to \r\n for terminal compatibility
                if byte == b'\n' {
                    uart.write_byte(b'\r');
                }
                uart.write_byte(byte);
            }
        }
        Ok(())
    }
}

/// Reads a single byte from the UART.
///
/// Blocks until a byte is available. Returns `None` if UART not initialized.
#[cfg(not(feature = "integration-test"))]
pub fn read_byte() -> Option<u8> {
    // SAFETY: Accessing global state in single-threaded context
    let driver = unsafe { &*UART_DRIVER.inner.get() };
    driver.as_ref().map(Uart16550::read_byte)
}

/// Writes formatted arguments to the UART.
#[doc(hidden)]
pub fn print_fmt(args: fmt::Arguments) {
    if Writer.write_fmt(args).is_err() {
        // Nothing to do - UART errors are silent
    }
}

/// Writes a single byte to the UART.
///
/// Used for echoing raw bytes, including UTF-8 lead and continuation bytes.
/// The terminal accumulates bytes and renders complete UTF-8 characters.
#[cfg(not(feature = "integration-test"))]
#[inline]
pub fn write_byte(byte: u8) {
    // SAFETY: Accessing global state in single-threaded context
    let driver = unsafe { &*UART_DRIVER.inner.get() };
    if let Some(uart) = driver.as_ref() {
        uart.write_byte(byte);
    }
}
