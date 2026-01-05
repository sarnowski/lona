// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! UART abstraction for the Lona VM.
//!
//! Provides a trait-based interface for UART communication, allowing
//! both hardware drivers and mock implementations for testing.

#[cfg(test)]
mod mod_test;

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(any(test, feature = "std"))]
mod mock;
// x86_64 UART requires seL4 IOPort capabilities
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
mod x86_64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::{PL011_PADDR, Pl011Uart, init as aarch64_init};
#[cfg(any(test, feature = "std"))]
pub use mock::MockUart;
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
pub use x86_64::{Com1Uart, init as x86_64_init};

/// UART interface for byte-level I/O.
pub trait Uart {
    /// Write a single byte.
    fn write_byte(&mut self, byte: u8);

    /// Read a single byte (blocking).
    fn read_byte(&mut self) -> u8;

    /// Check if data is available to read.
    fn can_read(&self) -> bool;

    /// Check if the transmit buffer has space.
    fn can_write(&self) -> bool;
}

/// Extension trait providing higher-level string operations.
pub trait UartExt: Uart {
    /// Write a string.
    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// Write a string followed by a newline.
    fn write_line(&mut self, s: &str) {
        self.write_str(s);
        self.write_byte(b'\n');
    }

    /// Read a line into a buffer, returning the number of bytes read.
    ///
    /// Echoes characters back as they're typed.
    /// Handles backspace (0x7F and 0x08).
    /// Returns when CR (0x0D) or LF (0x0A) is received.
    fn read_line(&mut self, buf: &mut [u8]) -> usize {
        let mut pos = 0;

        loop {
            let byte = self.read_byte();

            match byte {
                // CR - end of line
                b'\r' | b'\n' => {
                    self.write_str("\r\n");
                    return pos;
                }
                // Backspace (DEL or BS)
                0x7F | 0x08 if pos > 0 => {
                    pos -= 1;
                    // Echo backspace: move back, write space, move back
                    self.write_str("\x08 \x08");
                }
                // Regular character
                byte if pos < buf.len() && (0x20..0x7F).contains(&byte) => {
                    buf[pos] = byte;
                    pos += 1;
                    self.write_byte(byte);
                }
                // Buffer full or non-printable - ignore
                _ => {}
            }
        }
    }
}

// Blanket implementation for all Uart types
impl<T: Uart> UartExt for T {}
