// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! # Lona VM
//!
//! Runtime for Lonala bytecode in seL4 realms.
//!
//! This crate provides:
//! - Heap management for Lonala values
//! - UART drivers for aarch64 (PL011) and `x86_64` (COM1)
//! - Reader (lexer/parser) for Lonala source code
//! - Value representation and printing
//! - Library loading from embedded tar archives
//! - REPL for interactive development
//!
//! The VM runs in isolation within a realm's `VSpace`, communicating
//! with the Lona Memory Manager only via IPC.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(any(test, feature = "std"))]
extern crate std;

#[cfg(not(any(test, feature = "std")))]
extern crate alloc;

pub mod bytecode;
pub mod compiler;
pub mod heap;
pub mod intrinsics;
pub mod loader;
pub mod platform;
pub mod reader;
pub mod repl;
pub mod types;
pub mod uart;
pub mod value;
pub mod vm;

#[cfg(feature = "e2e-test")]
pub mod e2e;

// Re-export commonly used types at crate root
pub use types::{Paddr, Vaddr};

/// Crate version.
pub const VERSION: &str = match option_env!("LONA_VERSION") {
    Some(v) => v,
    None => "unknown",
};

#[cfg(test)]
mod lib_test;
