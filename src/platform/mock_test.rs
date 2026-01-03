// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the mock `VSpace`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::Vaddr;

#[test]
fn test_mock_vspace_creation() {
    let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
    assert_eq!(vspace.base(), Vaddr::new(0x1000));
    assert_eq!(vspace.size(), 4096);
    assert_eq!(vspace.end(), Vaddr::new(0x2000));
}

#[test]
fn test_mock_vspace_contains() {
    let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
    assert!(vspace.contains(Vaddr::new(0x1000)));
    assert!(vspace.contains(Vaddr::new(0x1FFF)));
    assert!(!vspace.contains(Vaddr::new(0x0FFF)));
    assert!(!vspace.contains(Vaddr::new(0x2000)));
}

#[test]
fn test_mock_vspace_read_write_u32() {
    let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

    vspace.write(Vaddr::new(0x1000), 0xDEAD_BEEFu32);
    let value: u32 = vspace.read(Vaddr::new(0x1000));
    assert_eq!(value, 0xDEAD_BEEF);
}

#[test]
fn test_mock_vspace_read_write_u64() {
    let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

    vspace.write(Vaddr::new(0x1008), 0x1234_5678_9ABC_DEF0u64);
    let value: u64 = vspace.read(Vaddr::new(0x1008));
    assert_eq!(value, 0x1234_5678_9ABC_DEF0);
}

#[test]
fn test_mock_vspace_read_write_struct() {
    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestStruct {
        a: u32,
        b: u64,
        c: u16,
    }

    let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

    let original = TestStruct {
        a: 42,
        b: 0xDEAD_BEEF_CAFE_BABE,
        c: 1234,
    };

    vspace.write(Vaddr::new(0x1100), original);
    let read_back: TestStruct = vspace.read(Vaddr::new(0x1100));
    assert_eq!(read_back, original);
}

#[test]
fn test_mock_vspace_slice() {
    let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

    let data = b"Hello, World!";
    vspace
        .slice_mut(Vaddr::new(0x1000), data.len())
        .copy_from_slice(data);

    let slice = vspace.slice(Vaddr::new(0x1000), data.len());
    assert_eq!(slice, data);
}

#[test]
fn test_mock_vspace_zero() {
    let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

    vspace.write(Vaddr::new(0x1000), 0xFFFF_FFFFu32);
    vspace.zero(Vaddr::new(0x1000), 4);

    let value: u32 = vspace.read(Vaddr::new(0x1000));
    assert_eq!(value, 0);
}

#[test]
fn test_mock_vspace_copy_within() {
    let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

    vspace.write(Vaddr::new(0x1000), 0xDEAD_BEEFu32);
    vspace.copy_within(Vaddr::new(0x1000), Vaddr::new(0x1100), 4);

    let src: u32 = vspace.read(Vaddr::new(0x1000));
    let dst: u32 = vspace.read(Vaddr::new(0x1100));
    assert_eq!(src, 0xDEAD_BEEF);
    assert_eq!(dst, 0xDEAD_BEEF);
}

#[test]
#[should_panic(expected = "below base")]
fn test_mock_vspace_read_below_base() {
    let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
    let _: u32 = vspace.read(Vaddr::new(0x0FFF));
}

#[test]
#[should_panic(expected = "beyond end")]
fn test_mock_vspace_read_beyond_end() {
    let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
    let _: u32 = vspace.read(Vaddr::new(0x2000));
}

#[test]
#[should_panic(expected = "would exceed")]
fn test_mock_vspace_read_partial_beyond_end() {
    let vspace = MockVSpace::new(4096, Vaddr::new(0x1000));
    let _: u32 = vspace.read(Vaddr::new(0x1FFE));
}

#[test]
fn test_mock_vspace_unaligned_access() {
    let mut vspace = MockVSpace::new(4096, Vaddr::new(0x1000));

    vspace.write(Vaddr::new(0x1001), 0xDEAD_BEEFu32);
    let value: u32 = vspace.read(Vaddr::new(0x1001));
    assert_eq!(value, 0xDEAD_BEEF);
}

#[test]
fn test_mock_vspace_raw_memory_access() {
    let mut vspace = MockVSpace::new(16, Vaddr::new(0x1000));

    vspace.write(Vaddr::new(0x1000), 0x1234_5678u32);

    let raw = vspace.raw_memory();
    assert_eq!(raw[0], 0x78);
    assert_eq!(raw[1], 0x56);
    assert_eq!(raw[2], 0x34);
    assert_eq!(raw[3], 0x12);
}
