// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! PL011 UART driver for ARM platforms.
//!
//! Provides serial I/O for debugging and console output. Uses MMIO to
//! communicate with the PL011 UART hardware found on QEMU virt and
//! Raspberry Pi platforms.

use core::cell::UnsafeCell;
use core::fmt::{self, Write};
use core::ptr::{read_volatile, write_volatile};

use super::fdt::UartInfo;
use crate::memory::Sel4PageProvider;

// PL011 UART register offsets
/// Data register - write to transmit, read to receive.
const UARTDR: usize = 0x000;
/// Flag register - contains status bits.
const UARTFR: usize = 0x018;

// Flag register bits
/// Transmit FIFO full flag.
const UARTFR_TXFF: u32 = 1 << 5;
/// Receive FIFO empty flag.
#[cfg(not(feature = "integration-test"))]
const UARTFR_RXFE: u32 = 1 << 4;

/// PL011 UART driver for serial I/O.
///
/// Provides blocking read/write operations. The driver must be initialized
/// before use by calling `init` with device information from FDT.
pub struct Pl011 {
    /// Virtual address of the UART MMIO region.
    base: *mut u32,
}

impl Pl011 {
    /// Creates a new UART driver with the given MMIO base address.
    ///
    /// # Safety
    ///
    /// - `base` must point to a valid PL011 UART MMIO region
    /// - `base` must be aligned to 4 bytes (u32)
    /// - Only one `Pl011` instance should exist per physical UART
    const unsafe fn new(base: *mut u32) -> Self {
        Self { base }
    }

    /// Writes a single byte to the UART.
    ///
    /// Blocks until the transmit FIFO has space, then writes the byte.
    pub fn write_byte(&self, byte: u8) {
        // SAFETY: UART base is valid and properly aligned for u32 access
        unsafe {
            self.wait_tx_ready();
        }
        // SAFETY: UART base is valid, volatile write required for MMIO
        unsafe {
            self.write_data(byte);
        }
    }

    /// Waits until the TX FIFO has space.
    ///
    /// # Safety
    ///
    /// UART base must be valid.
    unsafe fn wait_tx_ready(&self) {
        // SAFETY: Caller ensures base is valid
        let fr_ptr = unsafe { self.base.add(UARTFR / 4) };
        // SAFETY: fr_ptr points to valid MMIO register
        while (unsafe { read_volatile(fr_ptr) } & UARTFR_TXFF) != 0 {
            core::hint::spin_loop();
        }
    }

    /// Writes a byte to the data register.
    ///
    /// # Safety
    ///
    /// TX FIFO must have space (call `wait_tx_ready` first).
    unsafe fn write_data(&self, byte: u8) {
        // SAFETY: Caller ensures base is valid and TX ready
        let dr_ptr = unsafe { self.base.add(UARTDR / 4) };
        // SAFETY: dr_ptr points to valid MMIO register
        unsafe {
            write_volatile(dr_ptr, u32::from(byte));
        }
    }

    /// Reads a single byte from the UART.
    ///
    /// Blocks until a byte is available in the receive FIFO.
    #[cfg(not(feature = "integration-test"))]
    pub fn read_byte(&self) -> u8 {
        // SAFETY: UART base is valid
        unsafe {
            self.wait_rx_ready();
        }
        // SAFETY: UART base is valid, data available
        unsafe { self.read_data() }
    }

    /// Waits until the RX FIFO has data.
    ///
    /// # Safety
    ///
    /// UART base must be valid.
    #[cfg(not(feature = "integration-test"))]
    unsafe fn wait_rx_ready(&self) {
        // SAFETY: Caller ensures base is valid
        let fr_ptr = unsafe { self.base.add(UARTFR / 4) };
        // SAFETY: fr_ptr points to valid MMIO register
        while (unsafe { read_volatile(fr_ptr) } & UARTFR_RXFE) != 0 {
            core::hint::spin_loop();
        }
    }

    /// Reads a byte from the data register.
    ///
    /// # Safety
    ///
    /// RX FIFO must have data (call `wait_rx_ready` first).
    #[cfg(not(feature = "integration-test"))]
    unsafe fn read_data(&self) -> u8 {
        // SAFETY: Caller ensures base is valid and RX ready
        let dr_ptr = unsafe { self.base.add(UARTDR / 4) };
        // SAFETY: dr_ptr points to valid MMIO register
        let value = unsafe { read_volatile(dr_ptr) };
        // Only lower 8 bits contain the received byte
        #[expect(
            clippy::cast_possible_truncation,
            clippy::as_conversions,
            reason = "[approved] intentional u8 extraction from UART data register"
        )]
        let byte = value as u8;
        byte
    }
}

/// Global UART driver instance.
///
/// Initialized by `init` during system startup.
struct Driver {
    inner: UnsafeCell<Option<Pl011>>,
}

// SAFETY: Single-threaded access in seL4 root task
unsafe impl Sync for Driver {}

static UART_DRIVER: Driver = Driver {
    inner: UnsafeCell::new(None),
};

/// Initializes the global UART driver.
///
/// Maps the UART device memory and initializes the driver for use.
/// Must be called once during startup before using print!/println! macros.
///
/// # Safety
///
/// - Must be called after `PAGE_PROVIDER` is initialized
/// - Must be called in single-threaded context
/// - Should only be called once
pub unsafe fn init(uart_info: UartInfo, page_provider: &Sel4PageProvider) -> bool {
    // Map the UART device memory
    // SAFETY: page_provider is initialized, single-threaded context
    let Some(base) = (unsafe { page_provider.map_device_frame(uart_info.paddr) }) else {
        return false;
    };

    // PL011 UART registers are 32-bit aligned, but map_device_frame returns *mut u8.
    // The QEMU virt platform UART base (0x09000000) is 4-byte aligned.
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "UART base address is 4-byte aligned"
    )]
    let base_u32 = base.cast::<u32>();

    // SAFETY: base is valid MMIO pointer from map_device_frame
    let uart = unsafe { Pl011::new(base_u32) };

    // SAFETY: Single-threaded initialization
    unsafe {
        *UART_DRIVER.inner.get() = Some(uart);
    }

    true
}

/// Writer that outputs to the UART.
///
/// Implements `core::fmt::Write` for use with formatting macros.
pub struct Writer;

#[expect(
    clippy::missing_trait_methods,
    reason = "default implementations are sufficient for our use case"
)]
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
/// Blocks until a byte is available. Returns `None` if the UART is not initialized.
#[cfg(not(feature = "integration-test"))]
pub fn read_byte() -> Option<u8> {
    // SAFETY: Accessing global state in single-threaded context
    let driver = unsafe { &*UART_DRIVER.inner.get() };
    driver.as_ref().map(Pl011::read_byte)
}

/// Writes formatted arguments to the UART.
///
/// This is the internal implementation used by the print macros.
#[doc(hidden)]
pub fn print_fmt(args: fmt::Arguments) {
    // Ignore write errors - UART output is best-effort
    if Writer.write_fmt(args).is_err() {
        // Nothing to do - UART errors are silent
    }
}

/// Prints to the UART without a newline.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::platform::uart::print_fmt(format_args!($($arg)*))
    };
}

/// Prints to the UART with a newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*))
    };
}
