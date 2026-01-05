// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Platform abstractions for the Lona Memory Manager.
//!
//! This module provides MMIO mapping for device memory (used in root task).

#[cfg(all(target_arch = "aarch64", feature = "sel4"))]
pub mod mmio;
