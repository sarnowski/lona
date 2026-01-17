// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Event loop for handling realm IPC requests.
//!
//! The Lona Memory Manager event loop:
//! 1. Waits for messages from realms
//! 2. Identifies the sender (realm ID)
//! 3. Dispatches to the appropriate handler
//! 4. Replies with results
//!
//! Currently supports:
//! - `AllocPages`: Allocate and map frames into a realm's `VSpace`

#[cfg(feature = "sel4")]
mod sel4_impl;

#[cfg(feature = "sel4")]
pub use sel4_impl::{EventLoop, EventLoopError, RealmEntry};

// =============================================================================
// Non-seL4 stubs
// =============================================================================

#[cfg(not(feature = "sel4"))]
use lona_abi::types::RealmId;

/// Stub for realm entry (non-seL4 builds).
#[cfg(not(feature = "sel4"))]
pub struct RealmEntry {
    /// Realm identifier.
    pub id: RealmId,
}

/// Stub for event loop error (non-seL4 builds).
#[cfg(not(feature = "sel4"))]
#[derive(Debug, Clone, Copy)]
pub enum EventLoopError {
    /// Out of memory.
    OutOfMemory,
}

/// Stub for event loop (non-seL4 builds).
#[cfg(not(feature = "sel4"))]
pub struct EventLoop;

#[cfg(not(feature = "sel4"))]
impl EventLoop {
    /// Create a new event loop.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Register a realm.
    pub const fn register_realm(&mut self, _realm: RealmEntry) {}

    /// Run the event loop (stub - just suspends).
    pub fn run(&mut self) -> ! {
        loop {
            core::hint::spin_loop();
        }
    }
}

#[cfg(not(feature = "sel4"))]
impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}
