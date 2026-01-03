// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the address types.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{Paddr, Vaddr};

#[test]
fn test_paddr_basic() {
    let addr = Paddr::new(0x1000);
    assert_eq!(addr.as_u64(), 0x1000);
    assert!(!addr.is_null());
    assert!(Paddr::null().is_null());
}

#[test]
fn test_paddr_arithmetic() {
    let addr = Paddr::new(0x1000);
    assert_eq!(addr.add(0x100).as_u64(), 0x1100);
    assert_eq!(addr.sub(0x100).as_u64(), 0x0F00);
    assert_eq!((addr + 0x100).as_u64(), 0x1100);
    assert_eq!((addr - 0x100).as_u64(), 0x0F00);
}

#[test]
fn test_paddr_alignment() {
    let addr = Paddr::new(0x1234);
    assert_eq!(addr.align_up(0x1000).map(Paddr::as_u64), Some(0x2000));
    assert_eq!(addr.align_down(0x1000).map(Paddr::as_u64), Some(0x1000));
    assert_eq!(addr.is_aligned(0x1000), Some(false));
    assert_eq!(Paddr::new(0x2000).is_aligned(0x1000), Some(true));
    assert_eq!(addr.align_up(0), None);
    assert_eq!(addr.align_up(3), None);
}

#[test]
fn test_vaddr_basic() {
    let addr = Vaddr::new(0x4000_0000);
    assert_eq!(addr.as_u64(), 0x4000_0000);
    assert!(!addr.is_null());
    assert!(Vaddr::null().is_null());
}

#[test]
fn test_vaddr_arithmetic() {
    let addr = Vaddr::new(0x4000_0000);
    assert_eq!(addr.add(0x1000).as_u64(), 0x4000_1000);
    assert_eq!(addr.sub(0x1000).as_u64(), 0x3FFF_F000);
}

#[test]
fn test_vaddr_alignment() {
    let addr = Vaddr::new(0x4000_1234);
    assert_eq!(addr.align_up(0x1000).map(Vaddr::as_u64), Some(0x4000_2000));
    assert_eq!(
        addr.align_down(0x1000).map(Vaddr::as_u64),
        Some(0x4000_1000)
    );
}

#[test]
fn test_vaddr_diff() {
    let a = Vaddr::new(0x5000);
    let b = Vaddr::new(0x3000);
    assert_eq!(a.diff(b), 0x2000);
}

#[test]
fn test_address_debug_format() {
    let paddr = Paddr::new(0x1234);
    let vaddr = Vaddr::new(0x5678);
    assert_eq!(format!("{paddr:?}"), "Paddr(0x1234)");
    assert_eq!(format!("{vaddr:?}"), "Vaddr(0x5678)");
}
