// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Platform-specific hardware abstraction for Lona.
//!
//! This module contains drivers and abstractions for hardware devices.
//! Supports multiple platforms:
//! - ARM64: QEMU virt, Raspberry Pi 4B (PL011 UART via MMIO)
//! - `x86_64`: Standard PCs (16550 UART via I/O ports)

pub mod arch;

#[cfg(target_arch = "aarch64")]
pub mod fdt;

/// Prints to the UART without a newline.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::platform::arch::print_fmt(format_args!($($arg)*))
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
