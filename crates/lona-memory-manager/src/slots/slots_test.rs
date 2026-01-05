// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the slot allocator.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn new_allocator() {
    let alloc = SlotAllocator::new(100, 200);
    assert_eq!(alloc.remaining(), 100);
    assert!(!alloc.is_exhausted());
}

#[test]
fn alloc_single() {
    let mut alloc = SlotAllocator::new(10, 13);

    assert_eq!(alloc.alloc(), Some(10));
    assert_eq!(alloc.alloc(), Some(11));
    assert_eq!(alloc.alloc(), Some(12));
    assert_eq!(alloc.alloc(), None);
    assert!(alloc.is_exhausted());
}

#[test]
fn alloc_range() {
    let mut alloc = SlotAllocator::new(0, 10);

    assert_eq!(alloc.alloc_range(3), Some(0));
    assert_eq!(alloc.remaining(), 7);

    assert_eq!(alloc.alloc_range(5), Some(3));
    assert_eq!(alloc.remaining(), 2);

    assert_eq!(alloc.alloc_range(3), None); // Not enough
    assert_eq!(alloc.remaining(), 2);

    assert_eq!(alloc.alloc_range(2), Some(8));
    assert!(alloc.is_exhausted());
}

#[test]
fn alloc_range_zero() {
    let mut alloc = SlotAllocator::new(5, 10);
    assert_eq!(alloc.alloc_range(0), Some(5));
    assert_eq!(alloc.remaining(), 5); // No change
}

#[test]
fn empty_allocator() {
    let mut alloc = SlotAllocator::new(0, 0);
    assert!(alloc.is_exhausted());
    assert_eq!(alloc.remaining(), 0);
    assert_eq!(alloc.alloc(), None);
}

#[test]
fn remaining_decreases() {
    let mut alloc = SlotAllocator::new(0, 5);

    assert_eq!(alloc.remaining(), 5);
    let _ = alloc.alloc();
    assert_eq!(alloc.remaining(), 4);
    let _ = alloc.alloc_range(2);
    assert_eq!(alloc.remaining(), 2);
}
