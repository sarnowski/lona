// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Shared ABI definitions between Lona Memory Manager and Lona VM.
//!
//! This crate defines the contract between the two Lona binaries:
//! - Type definitions for IDs, addresses, and capabilities
//! - `VSpace` layout constants (fixed virtual addresses for all regions)
//! - IPC message formats for fault handling and requests
//! - Boot protocol (entry point arguments)
//!
//! # Design Principles
//!
//! - **No dependencies**: Pure data types, 100% host-testable
//! - **Stable layout**: All types use `#[repr(C)]` for FFI safety
//! - **64-bit only**: Lona targets 64-bit platforms exclusively
//!
//! # Modules
//!
//! - [`types`]: Core ID types (`RealmId`, `ProcessId`, `WorkerId`, `CapSlot`)
//! - [`layout`]: `VSpace` region addresses and constants
//! - [`boot`]: Realm entry point argument format
//! - [`fault`]: Fault information structures

#![no_std]

pub mod boot;
pub mod fault;
pub mod layout;
pub mod tcb;
pub mod types;

// Re-export commonly used types at crate root
pub use boot::BootFlags;
pub use fault::{FaultInfo, FaultType};
pub use layout::{Permissions, RegionType};
pub use tcb::{InitialRegisters, ipc_buffer_vaddr};
pub use types::{CapSlot, ProcessId, RealmId, WorkerId};
