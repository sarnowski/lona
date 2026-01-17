// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Realm creation and management.

mod boot_module;
pub mod constants;
mod device;
mod init;
mod kernel_objects;
mod page_tables;
mod tcb;
mod types;

pub mod frame_mapping;

pub use boot_module::{VmBootModule, find_vm_boot_module};
pub use init::create_init_realm;
#[cfg(feature = "sel4")]
pub use kernel_objects::create_reply;
#[cfg(feature = "sel4")]
pub use tcb::start_worker;
pub use types::{Realm, RealmError};

// Non-seL4 stubs
#[cfg(not(feature = "sel4"))]
pub use init::start_worker;
