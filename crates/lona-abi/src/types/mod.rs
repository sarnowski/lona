// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Core type definitions for realm, process, capability identifiers, and addresses.
//!
//! These newtypes prevent accidentally mixing different ID types at compile time.

mod addr;
mod cap;
mod id;

#[cfg(test)]
mod addr_test;
#[cfg(test)]
mod cap_test;
#[cfg(test)]
mod id_test;

pub use addr::{Paddr, Vaddr};
pub use cap::CapSlot;
pub use id::{ProcessId, RealmId, WorkerId};
