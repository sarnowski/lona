// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the ELF parser.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// Minimal valid ELF64 executable header (64 bytes).
/// This represents a valid ELF with 1 program header.
fn minimal_elf_header() -> [u8; 64] {
    let mut header = [0u8; 64];

    // Magic
    header[0] = 0x7F;
    header[1] = b'E';
    header[2] = b'L';
    header[3] = b'F';

    // Class: 64-bit
    header[4] = 2;

    // Data: little-endian
    header[5] = 1;

    // Version
    header[6] = 1;

    // OS/ABI (System V)
    header[7] = 0;

    // Type: executable (little-endian u16 = 2)
    header[16] = 2;
    header[17] = 0;

    // Machine: x86_64 (0x3E)
    header[18] = 0x3E;
    header[19] = 0;

    // Version (u32 = 1)
    header[20] = 1;

    // Entry point (u64 = 0x1000)
    header[24] = 0x00;
    header[25] = 0x10;

    // Program header offset (u64 = 64, right after ELF header)
    header[32] = 64;

    // ELF header size (u16 = 64)
    header[52] = 64;

    // Program header entry size (u16 = 56)
    header[54] = 56;

    // Number of program headers (u16 = 1)
    header[56] = 1;

    header
}

/// Create a `PT_LOAD` program header.
fn pt_load_phdr(vaddr: u64, offset: u64, filesz: u64, memsz: u64, flags: u32) -> [u8; 56] {
    let mut phdr = [0u8; 56];

    // Type: PT_LOAD (u32 = 1)
    phdr[0] = 1;

    // Flags (u32)
    phdr[4] = (flags & 0xFF) as u8;
    phdr[5] = ((flags >> 8) & 0xFF) as u8;
    phdr[6] = ((flags >> 16) & 0xFF) as u8;
    phdr[7] = ((flags >> 24) & 0xFF) as u8;

    // Offset (u64)
    write_u64(&mut phdr[8..16], offset);

    // Virtual address (u64)
    write_u64(&mut phdr[16..24], vaddr);

    // Physical address (unused)
    write_u64(&mut phdr[24..32], vaddr);

    // File size (u64)
    write_u64(&mut phdr[32..40], filesz);

    // Memory size (u64)
    write_u64(&mut phdr[40..48], memsz);

    // Alignment
    write_u64(&mut phdr[48..56], 0x1000);

    phdr
}

fn write_u64(buf: &mut [u8], value: u64) {
    buf[0] = (value & 0xFF) as u8;
    buf[1] = ((value >> 8) & 0xFF) as u8;
    buf[2] = ((value >> 16) & 0xFF) as u8;
    buf[3] = ((value >> 24) & 0xFF) as u8;
    buf[4] = ((value >> 32) & 0xFF) as u8;
    buf[5] = ((value >> 40) & 0xFF) as u8;
    buf[6] = ((value >> 48) & 0xFF) as u8;
    buf[7] = ((value >> 56) & 0xFF) as u8;
}

#[test]
fn parse_minimal_elf() {
    let header = minimal_elf_header();
    // Program header starts at offset 64, is 56 bytes
    // Segment data starts at offset 120, is 16 bytes
    let phdr = pt_load_phdr(
        0x0000_0001_0000_0000, // vaddr (SHARED_CODE_BASE)
        120,                   // offset (after headers)
        16,                    // filesz
        16,                    // memsz
        PF_R | PF_X,           // flags (RX)
    );

    let segment_data = [
        0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE,
        0xF0,
    ];

    // Combine: header + phdr + segment data
    let mut elf_data = Vec::new();
    elf_data.extend_from_slice(&header);
    elf_data.extend_from_slice(&phdr);
    elf_data.extend_from_slice(&segment_data);

    let elf = Elf::parse(&elf_data).unwrap();

    assert_eq!(elf.entry_point(), 0x1000);
    assert_eq!(elf.loadable_segment_count(), 1);

    let segment = elf.loadable_segments().next().unwrap();
    assert_eq!(segment.vaddr, 0x0000_0001_0000_0000);
    assert_eq!(segment.mem_size, 16);
    assert_eq!(segment.data, &segment_data);
    assert!(segment.permissions.read);
    assert!(!segment.permissions.write);
    assert!(segment.permissions.execute);
    assert_eq!(segment.permissions.as_str(), "RX");
}

#[test]
fn parse_elf_with_bss() {
    let header = minimal_elf_header();
    // Segment with memsz > filesz (has .bss)
    let phdr = pt_load_phdr(
        0x0000_0001_0008_0000, // vaddr
        120,                   // offset
        8,                     // filesz
        4096,                  // memsz (much larger - .bss)
        PF_R | PF_W,           // flags (RW)
    );

    let segment_data = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];

    let mut elf_data = Vec::new();
    elf_data.extend_from_slice(&header);
    elf_data.extend_from_slice(&phdr);
    elf_data.extend_from_slice(&segment_data);

    let elf = Elf::parse(&elf_data).unwrap();
    let segment = elf.loadable_segments().next().unwrap();

    assert_eq!(segment.data.len(), 8);
    assert_eq!(segment.mem_size, 4096);
    assert!(segment.permissions.read);
    assert!(segment.permissions.write);
    assert!(!segment.permissions.execute);
    assert_eq!(segment.permissions.as_str(), "RW");
}

#[test]
fn error_too_small() {
    let data = [0u8; 32]; // Too small for ELF header
    let result = Elf::parse(&data);
    assert_eq!(result.unwrap_err(), ElfError::TooSmall);
}

#[test]
fn error_invalid_magic() {
    let mut header = minimal_elf_header();
    header[0] = 0x00; // Corrupt magic

    let result = Elf::parse(&header);
    assert_eq!(result.unwrap_err(), ElfError::InvalidMagic);
}

#[test]
fn error_not_64_bit() {
    let mut header = minimal_elf_header();
    header[4] = 1; // 32-bit

    let result = Elf::parse(&header);
    assert_eq!(result.unwrap_err(), ElfError::Not64Bit);
}

#[test]
fn error_not_little_endian() {
    let mut header = minimal_elf_header();
    header[5] = 2; // Big-endian

    let result = Elf::parse(&header);
    assert_eq!(result.unwrap_err(), ElfError::NotLittleEndian);
}

#[test]
fn error_not_executable() {
    let mut header = minimal_elf_header();
    header[16] = 1; // ET_REL (relocatable)

    let result = Elf::parse(&header);
    assert_eq!(result.unwrap_err(), ElfError::NotExecutable);
}

#[test]
fn segment_permissions_all_combinations() {
    assert_eq!(SegmentPermissions::from_flags(0).as_str(), "--");
    assert_eq!(SegmentPermissions::from_flags(PF_R).as_str(), "RO");
    assert_eq!(SegmentPermissions::from_flags(PF_W).as_str(), "W");
    assert_eq!(SegmentPermissions::from_flags(PF_X).as_str(), "X");
    assert_eq!(SegmentPermissions::from_flags(PF_R | PF_W).as_str(), "RW");
    assert_eq!(SegmentPermissions::from_flags(PF_R | PF_X).as_str(), "RX");
    assert_eq!(SegmentPermissions::from_flags(PF_W | PF_X).as_str(), "WX");
    assert_eq!(
        SegmentPermissions::from_flags(PF_R | PF_W | PF_X).as_str(),
        "RWX"
    );
}

#[test]
fn multiple_segments() {
    let mut header = minimal_elf_header();
    // Update to have 2 program headers
    header[56] = 2;

    // First segment: code (RX)
    let phdr1 = pt_load_phdr(
        0x0000_0001_0000_0000, // vaddr
        176,                   // offset (after 2 phdrs: 64 + 56*2 = 176)
        32,                    // filesz
        32,                    // memsz
        PF_R | PF_X,
    );

    // Second segment: data (RW)
    let phdr2 = pt_load_phdr(
        0x0000_0001_0008_0000, // vaddr
        208,                   // offset (after first segment: 176 + 32)
        16,                    // filesz
        16,                    // memsz
        PF_R | PF_W,
    );

    let segment1_data = [0xAAu8; 32];
    let segment2_data = [0xBBu8; 16];

    let mut elf_data = Vec::new();
    elf_data.extend_from_slice(&header);
    elf_data.extend_from_slice(&phdr1);
    elf_data.extend_from_slice(&phdr2);
    elf_data.extend_from_slice(&segment1_data);
    elf_data.extend_from_slice(&segment2_data);

    let elf = Elf::parse(&elf_data).unwrap();
    assert_eq!(elf.loadable_segment_count(), 2);

    let segments: Vec<_> = elf.loadable_segments().collect();

    assert_eq!(segments[0].vaddr, 0x0000_0001_0000_0000);
    assert_eq!(segments[0].permissions.as_str(), "RX");
    assert_eq!(segments[0].data, &segment1_data);

    assert_eq!(segments[1].vaddr, 0x0000_0001_0008_0000);
    assert_eq!(segments[1].permissions.as_str(), "RW");
    assert_eq!(segments[1].data, &segment2_data);
}
