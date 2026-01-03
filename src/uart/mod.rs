// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Platform-independent UART interface.
//!
//! This module provides UART I/O abstraction for both input and output.
//! Platform-specific implementations are in separate modules:
//! - `mock` - Testing mock backed by `VecDeque`/`Vec`
//! - `x86_64` - COM1 port I/O at 0x3F8
//! - `aarch64` - PL011 MMIO at QEMU virt platform address

#![allow(unsafe_code)] // UART I/O requires unsafe port/MMIO access

#[cfg(test)]
mod mod_test;

#[cfg(all(target_arch = "aarch64", not(any(test, feature = "std"))))]
mod aarch64;
#[cfg(any(test, feature = "std"))]
mod mock;
#[cfg(all(target_arch = "x86_64", not(any(test, feature = "std"))))]
mod x86_64;

#[cfg(all(target_arch = "aarch64", not(any(test, feature = "std"))))]
pub use aarch64::{PL011_PADDR, Pl011Uart, init as aarch64_init};
#[cfg(any(test, feature = "std"))]
pub use mock::MockUart;
#[cfg(all(target_arch = "x86_64", not(any(test, feature = "std"))))]
pub use x86_64::{COM1_PORT, Com1Uart, init as x86_64_init};

/// Platform-independent UART interface.
pub trait Uart {
    /// Write a single byte. Blocks until the transmit buffer is ready.
    fn write_byte(&mut self, byte: u8);

    /// Read a single byte. Blocks until data is available.
    fn read_byte(&mut self) -> u8;

    /// Check if data is available to read (non-blocking).
    fn can_read(&self) -> bool;

    /// Check if ready to transmit (non-blocking).
    fn can_write(&self) -> bool;
}

/// Extension trait providing convenience methods for UART.
pub trait UartExt: Uart {
    /// Write a string slice.
    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// Write a string slice followed by a newline.
    fn write_line(&mut self, s: &str) {
        self.write_str(s);
        self.write_byte(b'\n');
    }

    /// Read a line into the provided buffer, handling backspace.
    ///
    /// Returns the number of bytes read (not including the newline).
    /// Echoes characters back to the UART as they're typed.
    /// Supports backspace (0x7F or 0x08) for editing.
    fn read_line(&mut self, buf: &mut [u8]) -> usize {
        let mut pos = 0;
        loop {
            let byte = self.read_byte();
            match byte {
                // Enter (CR or LF)
                b'\r' | b'\n' => {
                    self.write_byte(b'\r');
                    self.write_byte(b'\n');
                    return pos;
                }
                // Backspace or DEL
                0x08 | 0x7F => {
                    if pos > 0 {
                        pos -= 1;
                        // Erase character: backspace, space, backspace
                        self.write_byte(0x08);
                        self.write_byte(b' ');
                        self.write_byte(0x08);
                    }
                }
                // Printable ASCII
                byte if (0x20..0x7F).contains(&byte) => {
                    if pos < buf.len() {
                        buf[pos] = byte;
                        pos += 1;
                        self.write_byte(byte); // Echo
                    }
                }
                // Ignore other control characters
                _ => {}
            }
        }
    }
}

// Blanket implementation for all types implementing Uart
impl<T: Uart> UartExt for T {}
