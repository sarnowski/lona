// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! aarch64 PL011 UART driver.
//!
//! Uses MMIO to access the PL011 UART. The device memory must be mapped
//! before the UART can be used - call `init()` with the mapped virtual address.

use core::cell::UnsafeCell;
use core::ptr::{read_volatile, write_volatile};

use lona_abi::Paddr;

use super::Uart;

/// PL011 UART physical base address on QEMU virt platform.
pub const PL011_PADDR: Paddr = Paddr::new(0x0900_0000);

/// Data Register offset.
const UARTDR: usize = 0x00;

/// Flag Register offset.
const UARTFR: usize = 0x18;

/// Flag Register bit: RX FIFO Empty.
const UARTFR_RXFE: u32 = 1 << 4;

/// Flag Register bit: TX FIFO Full.
const UARTFR_TXFF: u32 = 1 << 5;

/// PL011 UART driver using MMIO.
struct Pl011Inner {
    /// Virtual address of the UART MMIO region.
    base: *mut u32,
}

impl Pl011Inner {
    /// Wait until TX FIFO has space.
    ///
    /// # Safety
    ///
    /// Base must be valid.
    unsafe fn wait_tx_ready(&self) {
        let fr_ptr = unsafe { self.base.add(UARTFR / 4) };
        while (unsafe { read_volatile(fr_ptr) } & UARTFR_TXFF) != 0 {
            core::hint::spin_loop();
        }
    }

    /// Write a byte to the data register.
    ///
    /// # Safety
    ///
    /// TX must be ready.
    unsafe fn write_data(&self, byte: u8) {
        let dr_ptr = unsafe { self.base.add(UARTDR / 4) };
        unsafe {
            write_volatile(dr_ptr, u32::from(byte));
        }
    }

    /// Wait until RX FIFO has data.
    ///
    /// # Safety
    ///
    /// Base must be valid.
    unsafe fn wait_rx_ready(&self) {
        let fr_ptr = unsafe { self.base.add(UARTFR / 4) };
        while (unsafe { read_volatile(fr_ptr) } & UARTFR_RXFE) != 0 {
            core::hint::spin_loop();
        }
    }

    /// Read a byte from the data register.
    ///
    /// # Safety
    ///
    /// RX must be ready.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "intentional u8 extraction from 32-bit UART data register"
    )]
    unsafe fn read_data(&self) -> u8 {
        let dr_ptr = unsafe { self.base.add(UARTDR / 4) };
        let value = unsafe { read_volatile(dr_ptr) };
        value as u8
    }

    /// Check if TX FIFO has space.
    fn can_write(&self) -> bool {
        // SAFETY: base is valid after initialization
        let fr_ptr = unsafe { self.base.add(UARTFR / 4) };
        (unsafe { read_volatile(fr_ptr) } & UARTFR_TXFF) == 0
    }

    /// Check if RX FIFO has data.
    fn can_read(&self) -> bool {
        // SAFETY: base is valid after initialization
        let fr_ptr = unsafe { self.base.add(UARTFR / 4) };
        (unsafe { read_volatile(fr_ptr) } & UARTFR_RXFE) == 0
    }
}

/// Global UART driver state.
struct UartDriver {
    inner: UnsafeCell<Option<Pl011Inner>>,
}

// SAFETY: Single-threaded access in seL4 root task.
unsafe impl Sync for UartDriver {}

static UART_DRIVER: UartDriver = UartDriver {
    inner: UnsafeCell::new(None),
};

/// Initialize the global UART driver with a mapped MMIO base address.
///
/// Must be called before using `Pl011Uart`.
///
/// # Safety
///
/// - `base` must point to valid, mapped PL011 UART MMIO memory
/// - Must be called in single-threaded context
/// - Should only be called once
#[expect(
    clippy::cast_ptr_alignment,
    reason = "UART base address is guaranteed 4-byte aligned (page-aligned mapping)"
)]
pub unsafe fn init(base: *mut u8) {
    let inner = Pl011Inner {
        base: base.cast::<u32>(),
    };
    // SAFETY: Single-threaded initialization
    unsafe {
        *UART_DRIVER.inner.get() = Some(inner);
    }
}

/// PL011 UART handle.
///
/// This is a zero-sized type that provides access to the global UART driver.
/// The driver must be initialized with `init()` before use.
pub struct Pl011Uart;

impl Pl011Uart {
    /// Create a new PL011 UART handle.
    ///
    /// The global driver must be initialized with `init()` first.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Get a reference to the initialized driver.
    fn driver(&self) -> Option<&Pl011Inner> {
        // SAFETY: Single-threaded access
        unsafe { (*UART_DRIVER.inner.get()).as_ref() }
    }
}

impl Default for Pl011Uart {
    fn default() -> Self {
        Self::new()
    }
}

impl Uart for Pl011Uart {
    fn write_byte(&mut self, byte: u8) {
        if let Some(inner) = self.driver() {
            // SAFETY: driver is initialized
            unsafe {
                inner.wait_tx_ready();
                inner.write_data(byte);
            }
        }
    }

    fn read_byte(&mut self) -> u8 {
        if let Some(inner) = self.driver() {
            // SAFETY: driver is initialized
            unsafe {
                inner.wait_rx_ready();
                inner.read_data()
            }
        } else {
            0
        }
    }

    fn can_read(&self) -> bool {
        self.driver().is_some_and(Pl011Inner::can_read)
    }

    fn can_write(&self) -> bool {
        self.driver().is_some_and(Pl011Inner::can_write)
    }
}
