// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Platform-specific hardware abstraction for Lona.
//!
//! This module contains drivers and abstractions for hardware devices.
//! Currently supports QEMU virt ARM64 with PL011 UART.

pub mod fdt;
pub mod uart;
