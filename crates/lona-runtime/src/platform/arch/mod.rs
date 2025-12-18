// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Architecture-specific platform support.
//!
//! Provides UART and other platform-specific drivers for each supported
//! architecture. On ARM64, this uses PL011 UART via MMIO. On `x86_64`,
//! this uses 16550 UART via I/O ports.

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;
