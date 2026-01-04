// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! # Lona VM
//!
//! Bytecode virtual machine for the Lonala language, designed to run on seL4.
//!
//! ## Architecture
//!
//! The VM implements BEAM-style lightweight processes with:
//! - Per-process heaps (growing down) and stacks (growing up)
//! - Per-process generational garbage collection
//! - Lock-free message passing via MPSC mailboxes
//! - Work-stealing scheduling with Chase-Lev deques
//!
//! ## `no_std` Support
//!
//! This crate is `no_std` by default for running on seL4. The `std` feature
//! is automatically enabled during testing to allow use of standard library
//! testing infrastructure.

#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(any(test, feature = "std"))]
extern crate std;

#[cfg(test)]
mod lib_test;

pub mod heap;
pub mod loader;
pub mod platform;
pub mod reader;
pub mod repl;
pub mod types;
pub mod uart;
pub mod value;

/// End-to-end test framework for seL4 environment.
///
/// Only available when the `e2e-test` feature is enabled.
#[cfg(feature = "e2e-test")]
pub mod e2e;

// Import core prelude items needed in this file
use core::result::Result::{self, Ok};

pub use heap::Heap;
pub use loader::{ChainedSource, NamespaceSource, TarSource};
pub use platform::{MemorySpace, Platform};
pub use types::{Paddr, Vaddr};
pub use uart::{Uart, UartExt};
pub use value::{HeapString, Pair, Value, print_value};

/// Crate version for runtime queries.
///
/// Uses the git-derived version from `LONA_VERSION` environment variable when available,
/// falling back to "unknown" otherwise.
pub const VERSION: &str = match option_env!("LONA_VERSION") {
    Some(v) => v,
    None => "unknown",
};

/// Initialize the VM runtime.
///
/// This is a placeholder that will eventually set up:
/// - Global control structures
/// - Scheduler state
/// - Initial process
///
/// # Errors
///
/// Returns an error describing what went wrong during initialization.
pub const fn init() -> Result<(), InitError> {
    Ok(())
}

/// Errors that can occur during VM initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitError {
    /// Failed to set up global control region.
    GlobalControlSetup,
    /// Failed to initialize scheduler.
    SchedulerInit,
    /// Insufficient memory for initial structures.
    InsufficientMemory,
}

impl core::fmt::Display for InitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GlobalControlSetup => write!(f, "failed to set up global control region"),
            Self::SchedulerInit => write!(f, "failed to initialize scheduler"),
            Self::InsufficientMemory => write!(f, "insufficient memory for initial structures"),
        }
    }
}
