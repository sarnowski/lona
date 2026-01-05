// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Boot protocol definitions for realm entry.
//!
//! When the Lona Memory Manager starts a realm, it passes arguments via CPU
//! registers. This module defines the argument layout and boot flags.
//!
//! # Register Layout
//!
//! ## `AArch64`
//!
//! | Register | Content |
//! |----------|---------|
//! | x0 | `realm_id` |
//! | x1 | `worker_id` |
//! | x2 | `heap_start` |
//! | x3 | `heap_size` |
//! | x4 | `flags` |
//!
//! ## `x86_64`
//!
//! | Register | Content |
//! |----------|---------|
//! | rdi | `realm_id` |
//! | rsi | `worker_id` |
//! | rdx | `heap_start` |
//! | rcx | `heap_size` |
//! | r8 | `flags` |

use crate::types::{RealmId, WorkerId};

/// Boot arguments passed to realm entry point.
///
/// This structure mirrors the register layout. The Lona VM entry point
/// receives these values and uses them to initialize the runtime.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BootArgs {
    /// This realm's unique identifier.
    pub realm_id: RealmId,

    /// This worker's index within the realm.
    pub worker_id: WorkerId,

    /// Start address of the process pool (for heap allocation).
    pub heap_start: u64,

    /// Initial size of mapped heap memory.
    pub heap_size: u64,

    /// Boot flags indicating realm capabilities.
    pub flags: BootFlags,
}

/// Boot flags indicating realm capabilities and mode.
///
/// These flags are set by the Lona Memory Manager based on the realm's
/// configuration and available hardware.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct BootFlags(u64);

impl BootFlags {
    /// No flags set.
    pub const NONE: Self = Self(0);

    /// This is the init realm (first user realm).
    pub const IS_INIT_REALM: u64 = 1 << 0;

    /// UART is mapped at `MMIO_BASE` for console output.
    pub const HAS_UART: u64 = 1 << 1;

    /// Framebuffer is available for graphics output.
    pub const HAS_FRAMEBUFFER: u64 = 1 << 2;

    /// Creates new boot flags from raw value.
    #[inline]
    #[must_use]
    pub const fn new(flags: u64) -> Self {
        Self(flags)
    }

    /// Returns the raw flags value.
    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Checks if a specific flag is set.
    #[inline]
    #[must_use]
    pub const fn has(self, flag: u64) -> bool {
        (self.0 & flag) != 0
    }

    /// Checks if this is the init realm.
    #[inline]
    #[must_use]
    pub const fn is_init_realm(self) -> bool {
        self.has(Self::IS_INIT_REALM)
    }

    /// Checks if UART is available.
    #[inline]
    #[must_use]
    pub const fn has_uart(self) -> bool {
        self.has(Self::HAS_UART)
    }

    /// Checks if framebuffer is available.
    #[inline]
    #[must_use]
    pub const fn has_framebuffer(self) -> bool {
        self.has(Self::HAS_FRAMEBUFFER)
    }

    /// Sets a flag.
    #[inline]
    #[must_use]
    pub const fn with(self, flag: u64) -> Self {
        Self(self.0 | flag)
    }

    /// Clears a flag.
    #[inline]
    #[must_use]
    pub const fn without(self, flag: u64) -> Self {
        Self(self.0 & !flag)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn boot_flags_default_is_none() {
        assert_eq!(BootFlags::default(), BootFlags::NONE);
        assert!(!BootFlags::NONE.is_init_realm());
        assert!(!BootFlags::NONE.has_uart());
    }

    #[test]
    fn boot_flags_can_set_and_clear() {
        let flags = BootFlags::NONE
            .with(BootFlags::IS_INIT_REALM)
            .with(BootFlags::HAS_UART);

        assert!(flags.is_init_realm());
        assert!(flags.has_uart());
        assert!(!flags.has_framebuffer());

        let flags = flags.without(BootFlags::HAS_UART);
        assert!(flags.is_init_realm());
        assert!(!flags.has_uart());
    }

    #[test]
    fn boot_args_layout() {
        // Verify BootArgs has C layout and expected size
        assert_eq!(
            core::mem::size_of::<BootArgs>(),
            8 + 2 + 6 + 8 + 8 + 8, // realm_id + worker_id + padding + heap_start + heap_size + flags
        );
    }
}
