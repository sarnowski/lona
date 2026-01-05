// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Platform abstraction for the Lona VM.
//!
//! This module provides abstractions over seL4-specific operations,
//! allowing the VM to be tested on the host system.

#[cfg(test)]
mod mock_test;

#[cfg(test)]
mod traits_test;

// Mock requires alloc, only available with std or test
#[cfg(any(test, feature = "std"))]
mod mock;
mod traits;

#[cfg(any(test, feature = "std"))]
pub use mock::MockVSpace;
pub use traits::{CacheAttr, MapError, MemorySpace, PagePerms, Platform};

#[cfg(not(any(test, feature = "std")))]
pub use traits::Sel4VSpace;
