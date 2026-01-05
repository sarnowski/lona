// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Write-only UART driver for logging.
//!
//! The Lona Memory Manager uses this for debug output. Unlike the VM's UART,
//! this is write-only (no REPL support needed).

#[cfg(all(target_arch = "aarch64", feature = "sel4"))]
mod aarch64;
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
mod x86_64;

#[cfg(all(target_arch = "aarch64", feature = "sel4"))]
pub use aarch64::{PL011_PADDR, Pl011Writer, init as aarch64_init};
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
pub use x86_64::{Com1Writer, init as x86_64_init};

/// Write-only UART interface.
pub trait UartWriter {
    /// Write a single byte.
    fn write_byte(&mut self, byte: u8);

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
}
