// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Minimal ELF64 parser for loading VM binary.
//!
//! This module provides just enough ELF parsing to extract `PT_LOAD` segments
//! from a 64-bit little-endian ELF binary. It does not support:
//! - Relocations (assumes position-dependent code at fixed addresses)
//! - Dynamic linking
//! - Section headers (only program headers)
//! - 32-bit ELF
//! - Big-endian targets

#[cfg(test)]
mod elf_test;

use core::mem::size_of;

// =============================================================================
// Constants
// =============================================================================

/// ELF magic bytes.
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

/// ELF class: 64-bit.
const ELFCLASS64: u8 = 2;

/// ELF data encoding: little-endian.
const ELFDATA2LSB: u8 = 1;

/// ELF type: executable.
const ET_EXEC: u16 = 2;

/// Program header type: loadable segment.
const PT_LOAD: u32 = 1;

/// Segment flag: executable.
const PF_X: u32 = 1;

/// Segment flag: writable.
const PF_W: u32 = 2;

/// Segment flag: readable.
const PF_R: u32 = 4;

/// ELF header size for 64-bit.
const ELF64_HEADER_SIZE: usize = 64;

// =============================================================================
// ELF Structures
// =============================================================================

/// ELF64 file header (layout matches ELF specification).
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Elf64Header {
    /// Magic number and identification bytes.
    ident: [u8; 16],
    /// Object file type (`ET_EXEC` = 2).
    file_type: u16,
    /// Target architecture.
    machine: u16,
    /// ELF version (1).
    version: u32,
    /// Entry point virtual address.
    entry: u64,
    /// Program header table offset.
    phoff: u64,
    /// Section header table offset (unused).
    shoff: u64,
    /// Processor-specific flags.
    flags: u32,
    /// ELF header size.
    ehsize: u16,
    /// Program header entry size.
    phentsize: u16,
    /// Number of program headers.
    phnum: u16,
    /// Section header entry size (unused).
    shentsize: u16,
    /// Number of section headers (unused).
    shnum: u16,
    /// Section name string table index (unused).
    shstrndx: u16,
}

/// ELF64 program header (layout matches ELF specification).
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Elf64Phdr {
    /// Segment type (`PT_LOAD` = 1).
    seg_type: u32,
    /// Segment flags (`PF_R`, `PF_W`, `PF_X`).
    flags: u32,
    /// Offset in file.
    offset: u64,
    /// Virtual address in memory.
    vaddr: u64,
    /// Physical address (unused).
    paddr: u64,
    /// Size in file.
    filesz: u64,
    /// Size in memory (>= `filesz`).
    memsz: u64,
    /// Alignment.
    align: u64,
}

// =============================================================================
// Public Types
// =============================================================================

/// Error during ELF parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// File too small for ELF header.
    TooSmall,
    /// Invalid ELF magic bytes.
    InvalidMagic,
    /// Not a 64-bit ELF.
    Not64Bit,
    /// Not little-endian.
    NotLittleEndian,
    /// Not an executable.
    NotExecutable,
    /// Program header table extends beyond file.
    InvalidPhdrOffset,
}

/// Memory permissions for a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentPermissions {
    /// Segment is readable.
    pub read: bool,
    /// Segment is writable.
    pub write: bool,
    /// Segment is executable.
    pub execute: bool,
}

impl SegmentPermissions {
    /// Creates permissions from ELF flags.
    const fn from_flags(flags: u32) -> Self {
        Self {
            read: (flags & PF_R) != 0,
            write: (flags & PF_W) != 0,
            execute: (flags & PF_X) != 0,
        }
    }

    /// Returns a short string representation (e.g., "RX", "RW").
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match (self.read, self.write, self.execute) {
            (true, false, true) => "RX",
            (true, true, false) => "RW",
            (true, false, false) => "RO",
            (true, true, true) => "RWX",
            (false, false, true) => "X",
            (false, true, false) => "W",
            (false, false, false) => "--",
            (false, true, true) => "WX",
        }
    }
}

/// A loadable segment from the ELF file.
#[derive(Debug, Clone, Copy)]
pub struct LoadableSegment<'a> {
    /// Virtual address where this segment should be loaded.
    pub vaddr: u64,
    /// Size in memory (may be larger than data for .bss).
    pub mem_size: u64,
    /// Segment data from the file.
    pub data: &'a [u8],
    /// Memory permissions.
    pub permissions: SegmentPermissions,
}

/// Parsed ELF file.
#[derive(Debug)]
pub struct Elf<'a> {
    data: &'a [u8],
    header: Elf64Header,
}

impl<'a> Elf<'a> {
    /// Parse an ELF file from raw bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not a valid 64-bit little-endian ELF.
    pub fn parse(data: &'a [u8]) -> Result<Self, ElfError> {
        // Check minimum size
        if data.len() < ELF64_HEADER_SIZE {
            return Err(ElfError::TooSmall);
        }

        // Parse header
        // SAFETY: We verified the slice is large enough, and the struct is repr(C).
        let header: Elf64Header = unsafe { read_struct(data) };

        // Validate magic
        if header.ident[0..4] != ELF_MAGIC {
            return Err(ElfError::InvalidMagic);
        }

        // Validate class (64-bit)
        if header.ident[4] != ELFCLASS64 {
            return Err(ElfError::Not64Bit);
        }

        // Validate endianness (little-endian)
        if header.ident[5] != ELFDATA2LSB {
            return Err(ElfError::NotLittleEndian);
        }

        // Validate type (executable)
        if header.file_type != ET_EXEC {
            return Err(ElfError::NotExecutable);
        }

        // Validate program header table
        let phdr_end = header
            .phoff
            .checked_add(u64::from(header.phnum) * u64::from(header.phentsize))
            .ok_or(ElfError::InvalidPhdrOffset)?;

        if phdr_end > data.len() as u64 {
            return Err(ElfError::InvalidPhdrOffset);
        }

        Ok(Self { data, header })
    }

    /// Returns the entry point virtual address.
    #[must_use]
    pub const fn entry_point(&self) -> u64 {
        self.header.entry
    }

    /// Returns an iterator over loadable segments.
    pub fn loadable_segments(&self) -> impl Iterator<Item = LoadableSegment<'a>> + '_ {
        (0..self.header.phnum).filter_map(move |i| {
            let phdr = self.program_header(i);
            if phdr.seg_type != PT_LOAD {
                return None;
            }

            // Extract segment data
            let offset = phdr.offset as usize;
            let filesz = phdr.filesz as usize;

            // Bounds check
            if offset.saturating_add(filesz) > self.data.len() {
                return None;
            }

            let data = &self.data[offset..offset + filesz];

            Some(LoadableSegment {
                vaddr: phdr.vaddr,
                mem_size: phdr.memsz,
                data,
                permissions: SegmentPermissions::from_flags(phdr.flags),
            })
        })
    }

    /// Returns the number of loadable segments.
    #[must_use]
    pub fn loadable_segment_count(&self) -> usize {
        self.loadable_segments().count()
    }

    /// Read a program header by index.
    fn program_header(&self, index: u16) -> Elf64Phdr {
        let offset = self.header.phoff as usize + (index as usize) * self.header.phentsize as usize;

        // SAFETY: We validated the program header table bounds in parse().
        unsafe { read_struct(&self.data[offset..]) }
    }
}

/// Read a struct from a byte slice.
///
/// # Safety
///
/// The slice must be at least `size_of::<T>()` bytes.
/// The struct must be `repr(C)` with no padding requirements beyond the slice.
unsafe fn read_struct<T: Copy>(data: &[u8]) -> T {
    debug_assert!(data.len() >= size_of::<T>());
    // SAFETY: Caller ensures data is large enough. Use read_unaligned since ELF data may not be aligned.
    unsafe { data.as_ptr().cast::<T>().read_unaligned() }
}
