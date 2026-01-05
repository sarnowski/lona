// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Realm creation and management.

mod init;

pub use init::{
    Realm, RealmError, VmBootModule, create_init_realm, find_vm_boot_module, start_worker,
};
