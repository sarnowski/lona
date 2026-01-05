// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! x86_64 COM1 UART driver.
//!
//! Uses seL4 IOPort capabilities to access the 8250/16550 UART at 0x3F8.
//! Direct port I/O is forbidden in seL4 userspace, so we use syscalls.

use core::cell::UnsafeCell;

use sel4::sys::seL4_CPtr;

use super::Uart;

/// COM1 base I/O port address.
pub const COM1_PORT: u16 = 0x3F8;

/// Line Status Register offset.
const LSR_OFFSET: u16 = 5;

/// LSR bit: Data Ready (RX buffer has data).
const LSR_DATA_READY: u8 = 0x01;

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

    /// Read the Line Status Register.
    fn read_lsr(&self) -> u8 {
        self.port_in8(COM1_PORT + LSR_OFFSET)
    }

    fn can_write(&self) -> bool {
        self.read_lsr() & LSR_TX_EMPTY != 0
    }

    fn can_read(&self) -> bool {
        self.read_lsr() & LSR_DATA_READY != 0
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
/// Must be called before using `Com1Uart`.
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

/// COM1 UART handle.
///
/// This is a zero-sized type that provides access to the global UART driver.
/// The driver must be initialized with `init()` first.
pub struct Com1Uart;

impl Com1Uart {
    /// Create a new COM1 UART handle.
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

impl Default for Com1Uart {
    fn default() -> Self {
        Self::new()
    }
}

impl Uart for Com1Uart {
    fn write_byte(&mut self, byte: u8) {
        if let Some(inner) = self.driver() {
            // Wait for transmit buffer to be empty
            while !inner.can_write() {
                core::hint::spin_loop();
            }
            inner.port_out8(COM1_PORT, byte);
        }
    }

    fn read_byte(&mut self) -> u8 {
        if let Some(inner) = self.driver() {
            // Wait for data to be available
            while !inner.can_read() {
                core::hint::spin_loop();
            }
            inner.port_in8(COM1_PORT)
        } else {
            0
        }
    }

    fn can_read(&self) -> bool {
        self.driver().is_some_and(Com1Inner::can_read)
    }

    fn can_write(&self) -> bool {
        self.driver().is_some_and(Com1Inner::can_write)
    }
}
