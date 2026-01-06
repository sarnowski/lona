// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Realm creation and management.

mod boot_module;
mod constants;
mod device;
mod frame_mapping;
mod init;
mod kernel_objects;
mod page_tables;
mod tcb;
mod types;

pub use boot_module::{VmBootModule, find_vm_boot_module};
pub use init::create_init_realm;
#[cfg(feature = "sel4")]
pub use tcb::start_worker;
pub use types::{Realm, RealmError};

// Non-seL4 stubs
#[cfg(not(feature = "sel4"))]
pub use init::start_worker;
