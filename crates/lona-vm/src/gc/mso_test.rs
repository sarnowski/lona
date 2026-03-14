// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for MSO (Mark-Sweep Objects) list handling.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::gc::mso::{MsoEntry, MsoList};
use crate::platform::MemorySpace;
use crate::platform::MockVSpace;

/// Create mock memory for testing.
fn setup() -> MockVSpace {
    MockVSpace::new(1024 * 1024, Vaddr::new(0)) // 1 MB
}

// =============================================================================
// MsoEntry Tests
// =============================================================================

#[test]
fn mso_entry_size_is_16_bytes() {
    // MSO entry: next (8) + object_addr (8) = 16 bytes
    assert_eq!(core::mem::size_of::<MsoEntry>(), 16);
}

#[test]
fn mso_entry_roundtrip() {
    let entry = MsoEntry {
        next: Vaddr::new(0x1000),
        object_addr: Vaddr::new(0x2000),
    };

    assert_eq!(entry.next, Vaddr::new(0x1000));
    assert_eq!(entry.object_addr, Vaddr::new(0x2000));
}

// =============================================================================
// MsoList Tests
// =============================================================================

#[test]
fn mso_list_starts_empty() {
    let list = MsoList::new();
    assert!(list.is_empty());
    assert_eq!(list.head(), Vaddr::null());
}

#[test]
fn mso_list_push_single() {
    let mut mem = setup();
    let mut list = MsoList::new();

    // Allocate space for MSO entry
    let entry_addr = Vaddr::new(0x1000);
    let obj_addr = Vaddr::new(0x2000);

    list.push(&mut mem, entry_addr, obj_addr);

    assert!(!list.is_empty());
    assert_eq!(list.head(), entry_addr);

    // Read back the entry
    let entry: MsoEntry = mem.read(entry_addr);
    assert_eq!(entry.next, Vaddr::null());
    assert_eq!(entry.object_addr, obj_addr);
}

#[test]
fn mso_list_push_multiple() {
    let mut mem = setup();
    let mut list = MsoList::new();

    // Push three entries
    let entry1 = Vaddr::new(0x1000);
    let entry2 = Vaddr::new(0x1010);
    let entry3 = Vaddr::new(0x1020);

    list.push(&mut mem, entry1, Vaddr::new(0x2000));
    list.push(&mut mem, entry2, Vaddr::new(0x2010));
    list.push(&mut mem, entry3, Vaddr::new(0x2020));

    // Head should be most recent
    assert_eq!(list.head(), entry3);

    // Check chain
    let e3: MsoEntry = mem.read(entry3);
    assert_eq!(e3.next, entry2);

    let e2: MsoEntry = mem.read(entry2);
    assert_eq!(e2.next, entry1);

    let e1: MsoEntry = mem.read(entry1);
    assert_eq!(e1.next, Vaddr::null());
}

#[test]
fn mso_list_iterate() {
    let mut mem = setup();
    let mut list = MsoList::new();

    // Push entries
    list.push(&mut mem, Vaddr::new(0x1000), Vaddr::new(0x2000));
    list.push(&mut mem, Vaddr::new(0x1010), Vaddr::new(0x2010));
    list.push(&mut mem, Vaddr::new(0x1020), Vaddr::new(0x2020));

    // Collect object addresses through iteration
    let mut objects = Vec::new();
    let mut current = list.head();
    while current != Vaddr::null() {
        let entry: MsoEntry = mem.read(current);
        objects.push(entry.object_addr);
        current = entry.next;
    }

    // Should be in reverse order (stack)
    assert_eq!(objects.len(), 3);
    assert_eq!(objects[0], Vaddr::new(0x2020));
    assert_eq!(objects[1], Vaddr::new(0x2010));
    assert_eq!(objects[2], Vaddr::new(0x2000));
}

// =============================================================================
// Note: MSO sweep tests require Process, Worker, and Realm integration
// Those tests are in minor_test.rs and major_test.rs after Phase 7 is complete
// =============================================================================
