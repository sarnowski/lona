// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Embedded VM binary for single-image boot.
//!
//! When building for platforms without multiboot module support (e.g., aarch64),
//! the Lona VM ELF is embedded directly into the memory manager binary.
//!
//! Set `LONA_VM_ELF` environment variable during build to enable embedding.

// Include the generated code from build.rs
include!(concat!(env!("OUT_DIR"), "/embedded_vm.rs"));

/// Returns the embedded VM ELF bytes, if available.
#[must_use]
pub fn embedded_vm() -> Option<&'static [u8]> {
    EMBEDDED_VM_ELF
}

/// Returns true if a VM binary is embedded.
#[must_use]
pub const fn has_embedded_vm() -> bool {
    EMBEDDED_VM_ELF.is_some()
}
