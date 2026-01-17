// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! # Lona Memory Manager
//!
//! seL4 root task for resource management and realm creation.
//!
//! This crate is the trusted computing base (TCB) for Lona. It:
//! - Manages physical memory and capabilities
//! - Creates and terminates realms
//! - Handles IPC requests from the Lona VM
//! - Maps device memory for drivers

#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod elf;
pub mod embedded;
pub mod event_loop;
pub mod platform;
pub mod realm;
pub mod slots;
#[cfg(feature = "sel4")]
pub mod uart;
pub mod untyped;

/// Crate version.
pub const VERSION: &str = match option_env!("LONA_VERSION") {
    Some(v) => v,
    None => "unknown",
};
