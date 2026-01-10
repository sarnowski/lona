// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for basic heap allocation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::Vaddr;
use crate::platform::MockVSpace;

/// Create a test process with `MockVSpace`.
pub(super) fn setup() -> (Process, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mem_size = 128 * 1024;
    let mem = MockVSpace::new(mem_size, base);

    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;

    let proc = Process::new(1, young_base, young_size, old_base, old_size);
    (proc, mem)
}

#[test]
fn process_initial_state() {
    let (proc, _mem) = setup();

    assert_eq!(proc.pid, 1);
    assert_eq!(proc.status, ProcessStatus::Ready);
    assert_eq!(proc.ip, 0);
    assert!(proc.chunk.is_none());

    // Young heap should be empty initially
    assert_eq!(proc.htop, proc.heap);
    assert_eq!(proc.stop, proc.hend);
    assert_eq!(proc.heap_used(), 0);
    assert_eq!(proc.stack_used(), 0);

    // Old heap should be empty
    assert_eq!(proc.old_htop, proc.old_heap);
}

#[test]
fn alloc_basic() {
    let (mut proc, _mem) = setup();

    // Allocate 100 bytes
    let addr = proc.alloc(100, 1).unwrap();
    assert_eq!(addr, proc.heap);
    assert_eq!(proc.heap_used(), 100);
    assert_eq!(proc.htop, proc.heap.add(100));
}

#[test]
fn alloc_aligned() {
    let (mut proc, _mem) = setup();

    // First allocation: 5 bytes
    let addr1 = proc.alloc(5, 1).unwrap();
    assert_eq!(addr1, proc.heap);
    assert_eq!(proc.htop, proc.heap.add(5));

    // Second allocation: 16 bytes, 8-byte aligned
    let addr2 = proc.alloc(16, 8).unwrap();
    // Should be aligned to 8-byte boundary
    assert_eq!(addr2.as_u64() % 8, 0);
    assert!(addr2.as_u64() >= proc.heap.as_u64() + 5);
}

#[test]
fn alloc_zero() {
    let (mut proc, _mem) = setup();

    // Zero-size allocation should succeed and not change htop
    let addr = proc.alloc(0, 1).unwrap();
    assert_eq!(addr, proc.heap);
    assert_eq!(proc.heap_used(), 0);
}

#[test]
fn alloc_multiple() {
    let (mut proc, _mem) = setup();

    // Allocate several chunks
    let addr1 = proc.alloc(100, 1).unwrap();
    let addr2 = proc.alloc(200, 1).unwrap();
    let addr3 = proc.alloc(300, 1).unwrap();

    // They should be sequential
    assert_eq!(addr1, proc.heap);
    assert_eq!(addr2, proc.heap.add(100));
    assert_eq!(addr3, proc.heap.add(300));

    assert_eq!(proc.heap_used(), 600);
}

#[test]
fn alloc_oom() {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(256, base);
    let mut proc = Process::new(1, base, 100, base.add(100), 50);

    // First allocation should succeed
    let addr1 = proc.alloc(50, 1);
    assert!(addr1.is_some());

    // Second allocation should succeed
    let addr2 = proc.alloc(40, 1);
    assert!(addr2.is_some());

    // Third allocation (would exceed) should fail
    let addr3 = proc.alloc(20, 1);
    assert!(addr3.is_none());

    let _ = mem; // Suppress unused warning
}
