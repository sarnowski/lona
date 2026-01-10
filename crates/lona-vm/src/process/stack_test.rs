// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for stack operations.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::allocation_test::setup;
use super::*;
use crate::Vaddr;
use crate::platform::MockVSpace;

#[test]
fn stack_push_basic() {
    let (mut proc, _mem) = setup();

    let initial_stop = proc.stop;

    // Push 100 bytes onto stack
    let addr = proc.stack_push(100, 1).unwrap();

    // Stack grows down, so addr should be less than initial stop
    assert!(addr.as_u64() < initial_stop.as_u64());
    assert_eq!(proc.stack_used(), 100);
}

#[test]
fn stack_push_aligned() {
    let (mut proc, _mem) = setup();

    // First push: 5 bytes
    let addr1 = proc.stack_push(5, 1).unwrap();
    assert_eq!(proc.stack_used(), 5);

    // Second push: 16 bytes, 8-byte aligned
    let addr2 = proc.stack_push(16, 8).unwrap();
    // Should be aligned
    assert_eq!(addr2.as_u64() % 8, 0);
    assert!(addr2.as_u64() < addr1.as_u64());

    let _ = addr1; // Suppress unused warning
}

#[test]
fn stack_pop() {
    let (mut proc, _mem) = setup();

    let initial_stop = proc.stop;

    // Push and then pop
    proc.stack_push(100, 1).unwrap();
    assert_eq!(proc.stack_used(), 100);

    proc.stack_pop(100);
    assert_eq!(proc.stop, initial_stop);
    assert_eq!(proc.stack_used(), 0);
}

#[test]
fn stack_pop_partial() {
    let (mut proc, _mem) = setup();

    // Push 100 bytes
    proc.stack_push(100, 1).unwrap();

    // Pop only 30
    proc.stack_pop(30);

    // Should have 70 bytes remaining on stack
    assert_eq!(proc.stack_used(), 70);
}

#[test]
fn heap_stack_collision() {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(256, base);
    let mut proc = Process::new(1, base, 100, base.add(100), 50);

    // Allocate most of the heap
    proc.alloc(40, 1).unwrap();

    // Push most of the stack
    proc.stack_push(40, 1).unwrap();

    // Free space should be reduced
    assert_eq!(proc.free_space(), 20);

    // Another large allocation should fail (would collide)
    assert!(proc.alloc(30, 1).is_none());
    assert!(proc.stack_push(30, 1).is_none());

    let _ = mem; // Suppress unused warning
}
