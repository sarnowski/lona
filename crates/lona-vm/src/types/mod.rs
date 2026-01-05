// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Core type definitions for the Lona VM.
//!
//! This module provides type-safe wrappers for addresses and other fundamental
//! types. Using newtypes prevents mixing incompatible values (e.g., passing a
//! physical address where a virtual address is expected).

#[cfg(test)]
mod address_test;

mod address;

pub use address::{Paddr, Vaddr};
