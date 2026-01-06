// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! VM boot module discovery and parsing.
//!
//! This module handles finding and parsing the VM binary, either from
//! embedded data or seL4 boot modules.

use crate::elf::{Elf, ElfError};
use crate::embedded;

use super::RealmError;

/// Information about the VM boot module.
pub struct VmBootModule<'a> {
    /// Parsed ELF file.
    elf: Elf<'a>,
    /// Entry point address.
    pub entry_point: u64,
    /// Number of loadable segments.
    pub segment_count: usize,
    /// Total size of all segments in memory.
    pub total_mem_size: u64,
}

impl<'a> VmBootModule<'a> {
    /// Returns an iterator over loadable segments.
    pub fn segments(&self) -> impl Iterator<Item = crate::elf::LoadableSegment<'a>> + '_ {
        self.elf.loadable_segments()
    }
}

/// Find the VM binary in boot modules or embedded data.
#[cfg(feature = "sel4")]
pub fn find_vm_boot_module(
    _bootinfo: &sel4::BootInfoPtr,
) -> Result<VmBootModule<'static>, RealmError> {
    // First, check for embedded VM binary
    if let Some(elf_bytes) = embedded::embedded_vm() {
        sel4::debug_println!("Using embedded VM binary ({} bytes)", elf_bytes.len());
        return parse_vm_elf(elf_bytes);
    }

    // No embedded VM available
    sel4::debug_println!("No embedded VM found");
    Err(RealmError::NoVmBootModule)
}

/// Find the VM binary in boot modules (non-seL4 stub).
///
/// # Errors
///
/// Returns `RealmError::NoVmBootModule` when no VM is available.
#[cfg(not(feature = "sel4"))]
pub fn find_vm_boot_module() -> Result<VmBootModule<'static>, RealmError> {
    // Check for embedded VM binary
    if let Some(elf_bytes) = embedded::embedded_vm() {
        return parse_vm_elf(elf_bytes);
    }
    Err(RealmError::NoVmBootModule)
}

/// Parse VM ELF binary.
fn parse_vm_elf(elf_bytes: &[u8]) -> Result<VmBootModule<'_>, RealmError> {
    let elf = Elf::parse(elf_bytes).map_err(|e| match e {
        ElfError::TooSmall
        | ElfError::InvalidMagic
        | ElfError::Not64Bit
        | ElfError::NotLittleEndian
        | ElfError::NotExecutable
        | ElfError::InvalidPhdrOffset => RealmError::NoVmBootModule,
    })?;

    let entry_point = elf.entry_point();
    let segment_count = elf.loadable_segment_count();
    let total_mem_size: u64 = elf.loadable_segments().map(|s| s.mem_size).sum();

    Ok(VmBootModule {
        elf,
        entry_point,
        segment_count,
        total_mem_size,
    })
}
