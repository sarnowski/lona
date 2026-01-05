// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! x86_64 COM1 UART driver (write-only).
//!
//! Uses seL4 IOPort capabilities to access the 8250/16550 UART at 0x3F8.
//! The root task receives IOPort capabilities from bootinfo.

use core::cell::UnsafeCell;

use sel4::sys::seL4_CPtr;

use super::UartWriter;

/// COM1 base I/O port address.
pub const COM1_PORT: u16 = 0x3F8;

/// Line Status Register offset.
const LSR_OFFSET: u16 = 5;

/// LSR bit: Transmitter Holding Register Empty (TX ready).
const LSR_TX_EMPTY: u8 = 0x20;

/// COM1 UART driver using seL4 IOPort capabilities.
struct Com1Inner {
    /// Capability pointer to access I/O ports via seL4 syscalls.
    io_port_cap: seL4_CPtr,
}

impl Com1Inner {
    /// Read a byte from an I/O port using seL4 syscall.
    fn port_in8(&self, port: u16) -> u8 {
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

    /// Write a byte to an I/O port using seL4 syscall.
    fn port_out8(&self, port: u16, value: u8) {
        sel4::with_ipc_buffer_mut(|ipc_buffer| {
            let _ = ipc_buffer.inner_mut().seL4_X86_IOPort_Out8(
                self.io_port_cap,
                u64::from(port),
                u64::from(value),
            );
        });
    }

    /// Wait until TX is ready and write a byte.
    fn write_byte(&self, byte: u8) {
        // Wait for transmit buffer to be empty
        while (self.port_in8(COM1_PORT + LSR_OFFSET) & LSR_TX_EMPTY) == 0 {
            core::hint::spin_loop();
        }
        self.port_out8(COM1_PORT, byte);
    }
}

/// Global UART driver state.
struct UartDriver {
    inner: UnsafeCell<Option<Com1Inner>>,
}

// SAFETY: Single-threaded access in seL4 root task.
unsafe impl Sync for UartDriver {}

static UART_DRIVER: UartDriver = UartDriver {
    inner: UnsafeCell::new(None),
};

/// Initialize the global UART driver with an IOPort capability.
///
/// Must be called before using `Com1Writer`.
///
/// # Safety
///
/// Must be called in single-threaded context.
pub unsafe fn init(io_port_cap: seL4_CPtr) {
    let inner = Com1Inner { io_port_cap };
    // SAFETY: Single-threaded initialization
    unsafe {
        *UART_DRIVER.inner.get() = Some(inner);
    }
}

/// COM1 UART writer handle.
///
/// This is a zero-sized type that provides write-only access to the global UART driver.
/// The driver must be initialized with `init()` first.
pub struct Com1Writer;

impl Com1Writer {
    /// Create a new COM1 UART writer handle.
    ///
    /// The global driver must be initialized with `init()` first.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Get a reference to the initialized driver.
    fn driver(&self) -> Option<&Com1Inner> {
        // SAFETY: Single-threaded access
        unsafe { (*UART_DRIVER.inner.get()).as_ref() }
    }
}

impl Default for Com1Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl UartWriter for Com1Writer {
    fn write_byte(&mut self, byte: u8) {
        if let Some(inner) = self.driver() {
            inner.write_byte(byte);
        }
    }
}
