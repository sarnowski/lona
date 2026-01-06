// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! aarch64 PL011 UART driver (write-only).
//!
//! Uses direct MMIO to the PL011 UART. The root task has all capabilities,
//! so it can access device memory directly after mapping.

use core::cell::UnsafeCell;
use core::ptr::{read_volatile, write_volatile};

use lona_abi::Paddr;

use super::UartWriter;

/// PL011 UART physical base address on QEMU virt platform.
pub const PL011_PADDR: Paddr = Paddr::new(0x0900_0000);

/// Data Register offset.
const UARTDR: usize = 0x00;

/// Flag Register offset.
const UARTFR: usize = 0x18;

/// Flag Register bit: TX FIFO Full.
const UARTFR_TXFF: u32 = 1 << 5;

/// PL011 UART driver using MMIO.
struct Pl011Inner {
    /// Virtual address of the UART MMIO region.
    base: *mut u32,
}

impl Pl011Inner {
    /// Wait until TX FIFO has space and write a byte.
    ///
    /// # Safety
    ///
    /// Base must be valid.
    unsafe fn write_byte(&self, byte: u8) {
        // SAFETY: Caller guarantees base is valid
        let fr_ptr = unsafe { self.base.add(UARTFR / 4) };
        // Wait for TX FIFO to have space
        // SAFETY: fr_ptr points to valid UART register
        while (unsafe { read_volatile(fr_ptr) } & UARTFR_TXFF) != 0 {
            core::hint::spin_loop();
        }
        // Write to data register
        // SAFETY: Caller guarantees base is valid
        let dr_ptr = unsafe { self.base.add(UARTDR / 4) };
        // SAFETY: dr_ptr points to valid UART register
        unsafe { write_volatile(dr_ptr, u32::from(byte)) };
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
/// Must be called before using `Pl011Writer`.
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

/// PL011 UART writer handle.
///
/// This is a zero-sized type that provides write-only access to the global UART driver.
/// The driver must be initialized with `init()` before use.
pub struct Pl011Writer;

impl Pl011Writer {
    /// Create a new PL011 UART writer handle.
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

impl Default for Pl011Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl UartWriter for Pl011Writer {
    fn write_byte(&mut self, byte: u8) {
        if let Some(inner) = self.driver() {
            // SAFETY: driver is initialized
            unsafe {
                inner.write_byte(byte);
            }
        }
    }
}
