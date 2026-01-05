// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the untyped allocator.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn make_desc(slot: usize, paddr: u64, size_bits: u8) -> UntypedDesc {
    UntypedDesc {
        slot,
        paddr,
        size_bits,
        is_device: false,
        watermark: 0,
    }
}

#[test]
fn untyped_desc_size() {
    let desc = make_desc(0, 0, 12); // 4 KB
    assert_eq!(desc.size(), 4096);
    assert_eq!(desc.remaining(), 4096);
}

#[test]
fn untyped_desc_can_allocate() {
    let desc = make_desc(0, 0, 12); // 4 KB
    assert!(desc.can_allocate(12)); // Exactly fits
    assert!(!desc.can_allocate(13)); // Too large
}

#[test]
fn untyped_desc_allocate() {
    let mut desc = make_desc(0, 0x1000, 14); // 16 KB at 0x1000

    // Allocate a 4 KB object
    let paddr = desc.allocate(12).unwrap();
    assert_eq!(paddr, 0x1000);
    assert_eq!(desc.watermark, 4096);
    assert_eq!(desc.remaining(), 12288);

    // Allocate another 4 KB object
    let paddr = desc.allocate(12).unwrap();
    assert_eq!(paddr, 0x2000);

    // Allocate 8 KB - should fail (only 8 KB left but alignment)
    // Actually 8 KB fits starting at 0x3000
    let paddr = desc.allocate(13).unwrap();
    assert_eq!(paddr, 0x3000); // Aligned to 8 KB

    // No more space
    assert!(desc.allocate(12).is_none());
}

#[test]
fn allocator_add() {
    let mut alloc = UntypedAllocator::new();

    assert!(alloc.add(make_desc(0, 0, 12)));
    assert!(alloc.add(make_desc(1, 0x1000, 14)));

    assert_eq!(alloc.count, 2);
}

#[test]
fn allocator_allocate() {
    let mut alloc = UntypedAllocator::new();
    alloc.add(make_desc(10, 0x10000, 14)); // 16 KB

    let mut slots = SlotAllocator::new(100, 200);

    // Allocate a 4 KB frame
    let (ut_slot, dest_slot, paddr) = alloc.allocate(12, &mut slots, false).unwrap();
    assert_eq!(ut_slot, 10);
    assert_eq!(dest_slot, 100);
    assert_eq!(paddr, 0x10000);

    // Allocate another
    let (_, dest_slot, paddr) = alloc.allocate(12, &mut slots, false).unwrap();
    assert_eq!(dest_slot, 101);
    assert_eq!(paddr, 0x11000);
}

#[test]
fn allocator_total_free() {
    let mut alloc = UntypedAllocator::new();
    alloc.add(make_desc(0, 0, 14)); // 16 KB
    alloc.add(make_desc(1, 0x10000, 12)); // 4 KB

    assert_eq!(alloc.total_free(), 16384 + 4096);

    let mut slots = SlotAllocator::new(0, 100);
    let _ = alloc.allocate(12, &mut slots, false);

    assert_eq!(alloc.total_free(), 12288 + 4096);
}

#[test]
fn allocator_prefers_larger() {
    let mut alloc = UntypedAllocator::new();
    // Add small first, large second
    alloc.add(make_desc(0, 0, 12)); // 4 KB
    alloc.add(make_desc(1, 0x10000, 20)); // 1 MB

    // Sort should put larger first
    alloc.sort_by_size();

    // Allocation should come from larger untyped first
    let mut slots = SlotAllocator::new(0, 100);
    let (ut_slot, _, paddr) = alloc.allocate(12, &mut slots, false).unwrap();
    assert_eq!(ut_slot, 1); // From the 1 MB untyped
    assert_eq!(paddr, 0x10000);
}

#[test]
fn allocator_device_memory() {
    let mut alloc = UntypedAllocator::new();

    // Add regular memory
    alloc.add(make_desc(0, 0, 14));

    // Add device memory
    alloc.add(UntypedDesc {
        slot: 1,
        paddr: 0x1_0000_0000,
        size_bits: 12,
        is_device: true,
        watermark: 0,
    });

    let mut slots = SlotAllocator::new(0, 100);

    // Regular allocation should use non-device
    let (ut_slot, _, _) = alloc.allocate(12, &mut slots, false).unwrap();
    assert_eq!(ut_slot, 0);

    // Device allocation should use device
    let (ut_slot, _, paddr) = alloc.allocate(12, &mut slots, true).unwrap();
    assert_eq!(ut_slot, 1);
    assert_eq!(paddr, 0x1_0000_0000);
}
