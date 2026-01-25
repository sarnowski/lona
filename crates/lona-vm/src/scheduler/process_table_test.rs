// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `ProcessTable`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::large_stack_frames)]

use super::process_table::ProcessTable;
use crate::Vaddr;
use crate::process::{Process, ProcessId};

/// Create a test process with default heap configuration.
fn create_test_process() -> Process {
    Process::new(Vaddr::new(0x1000), 0x1000, Vaddr::new(0x2000), 0x1000)
}

#[test]
fn process_table_new_is_empty() {
    let table = ProcessTable::new();
    assert_eq!(table.count(), 0);
    assert!(!table.is_full());
}

#[test]
fn process_table_allocate_and_insert() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    assert_eq!(table.count(), 1);
    assert!(table.get(pid).is_some());
}

#[test]
fn process_table_get_stale_generation() {
    let mut table = ProcessTable::new();

    // Allocate, insert, remove
    let (index, gen1) = table.allocate().unwrap();
    let pid1 = ProcessId::new(index, gen1);
    let mut p1 = create_test_process();
    p1.pid = pid1;
    table.insert(p1);
    table.remove(pid1);

    // Allocate again (reuses same slot with new generation)
    let (index2, gen2) = table.allocate().unwrap();
    assert_eq!(index, index2); // Same slot
    assert_eq!(gen2, gen1 + 1); // Generation incremented

    let pid2 = ProcessId::new(index2, gen2);
    let mut p2 = create_test_process();
    p2.pid = pid2;
    table.insert(p2);

    // Old PID should not work
    assert!(table.get(pid1).is_none());
    // New PID should work
    assert!(table.get(pid2).is_some());
}

#[test]
fn process_table_remove_returns_process() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    let removed = table.remove(pid);
    assert!(removed.is_some());
    assert_eq!(table.count(), 0);
    assert!(table.get(pid).is_none());
}

#[test]
fn process_table_get_mut() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    process.ip = 0;
    table.insert(process);

    // Modify via get_mut
    table.get_mut(pid).unwrap().ip = 42;

    // Verify modification persisted
    assert_eq!(table.get(pid).unwrap().ip, 42);
}

#[test]
fn process_table_remove_invalid_pid() {
    let mut table = ProcessTable::new();

    // Remove from empty table
    let pid = ProcessId::new(0, 0);
    assert!(table.remove(pid).is_none());
}

#[test]
fn process_table_get_null_pid() {
    let table = ProcessTable::new();
    assert!(table.get(ProcessId::NULL).is_none());
}

#[test]
fn process_table_multiple_allocations() {
    let mut table = ProcessTable::new();

    let (idx1, gen1) = table.allocate().unwrap();
    let (idx2, gen2) = table.allocate().unwrap();
    let (idx3, gen3) = table.allocate().unwrap();

    // Should get different indices
    assert_ne!(idx1, idx2);
    assert_ne!(idx2, idx3);
    assert_ne!(idx1, idx3);

    // Initial generation should be 0
    assert_eq!(gen1, 0);
    assert_eq!(gen2, 0);
    assert_eq!(gen3, 0);
}

#[test]
fn process_table_free_list_reuse() {
    let mut table = ProcessTable::new();

    // Allocate 3 slots
    let (idx1, _) = table.allocate().unwrap();
    let (idx2, _) = table.allocate().unwrap();
    let (idx3, _) = table.allocate().unwrap();

    // Insert processes
    let pid1 = ProcessId::new(idx1, 0);
    let pid2 = ProcessId::new(idx2, 0);
    let pid3 = ProcessId::new(idx3, 0);

    let mut p1 = create_test_process();
    p1.pid = pid1;
    table.insert(p1);

    let mut p2 = create_test_process();
    p2.pid = pid2;
    table.insert(p2);

    let mut p3 = create_test_process();
    p3.pid = pid3;
    table.insert(p3);

    // Remove middle one
    table.remove(pid2);
    assert_eq!(table.count(), 2);

    // Next allocation should reuse idx2's slot
    let (idx_new, gen_new) = table.allocate().unwrap();
    assert_eq!(idx_new, idx2);
    assert_eq!(gen_new, 1); // Generation incremented
}
