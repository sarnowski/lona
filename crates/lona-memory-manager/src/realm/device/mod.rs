// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Device setup for realm initialization.
//!
//! This module provides architecture-specific device setup for realms:
//! - aarch64: MMIO-based UART mapping
//! - `x86_64`: `IOPort` capability for COM1

#[cfg(all(feature = "sel4", target_arch = "aarch64"))]
mod aarch64;
#[cfg(all(feature = "sel4", target_arch = "x86_64"))]
mod x86_64;

#[cfg(all(feature = "sel4", target_arch = "aarch64"))]
pub use aarch64::map_uart;
#[cfg(all(feature = "sel4", target_arch = "x86_64"))]
pub use x86_64::setup_ioport_uart;
