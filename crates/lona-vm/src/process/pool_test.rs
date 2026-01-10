// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `ProcessPool`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::pool::ProcessPool;
use crate::Vaddr;

#[test]
fn pool_initial_state() {
    let base = Vaddr::new(0x1_0000);
    let pool = ProcessPool::new(base, 1024);

    assert_eq!(pool.next(), base);
    assert_eq!(pool.limit(), base.add(1024));
    assert_eq!(pool.remaining(), 1024);
}

#[test]
fn pool_allocate_process() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    let (young_base, old_base) = pool.allocate_process_memory(512, 256).unwrap();

    assert_eq!(young_base, base);
    assert_eq!(old_base, base.add(512));
    assert_eq!(pool.remaining(), 256);
}

#[test]
fn pool_allocate_multiple() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 2048);

    // First process
    let (young1, old1) = pool.allocate_process_memory(512, 256).unwrap();
    assert_eq!(young1, base);
    assert_eq!(old1, base.add(512));

    // Second process
    let (young2, old2) = pool.allocate_process_memory(512, 256).unwrap();
    assert_eq!(young2, base.add(768));
    assert_eq!(old2, base.add(1280));

    assert_eq!(pool.remaining(), 512);
}

#[test]
fn pool_oom() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 100);

    // Should fail - not enough space
    let result = pool.allocate_process_memory(80, 40);
    assert!(result.is_none());
}
