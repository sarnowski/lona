// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `ProcessTable`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::large_stack_frames)]

use super::process_table::ProcessTable;
use crate::Vaddr;
use crate::process::heap_fragment::HeapFragment;
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

// ============================================================================
// Take / Put Back Tests
// ============================================================================

#[test]
fn take_extracts_process() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    process.ip = 42;
    table.insert(process);

    let taken = table.take(pid).unwrap();
    assert_eq!(taken.ip, 42);

    // get returns None (process extracted)
    assert!(table.get(pid).is_none());
    // is_taken returns true
    assert!(table.is_taken(pid));
    // count unchanged — slot still logically occupied
    assert_eq!(table.count(), 1);
}

#[test]
fn put_back_restores_process() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    process.ip = 42;
    table.insert(process);

    let taken = table.take(pid).unwrap();
    table.put_back(pid, taken);

    // Process is back
    let proc = table.get(pid).unwrap();
    assert_eq!(proc.ip, 42);
    assert!(!table.is_taken(pid));
}

#[test]
fn free_taken_slot_reclaims() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    let _taken = table.take(pid);
    table.free_taken_slot(pid);

    assert_eq!(table.count(), 0);
    assert!(!table.is_taken(pid));
    assert!(table.get(pid).is_none());

    // Slot can be reused
    let (idx2, gen2) = table.allocate().unwrap();
    assert_eq!(idx2, index);
    assert_eq!(gen2, generation + 1);
}

#[test]
fn take_invalid_pid_returns_none() {
    let mut table = ProcessTable::new();
    let pid = ProcessId::new(0, 0);
    assert!(table.take(pid).is_none());
}

#[test]
fn take_null_pid_returns_none() {
    let mut table = ProcessTable::new();
    assert!(table.take(ProcessId::NULL).is_none());
}

#[test]
fn take_put_back_preserves_generation() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    let taken = table.take(pid).unwrap();
    table.put_back(pid, taken);

    // Same generation — stale references still work
    assert!(table.get(pid).is_some());
}

#[test]
fn is_taken_false_for_unallocated() {
    let table = ProcessTable::new();
    assert!(!table.is_taken(ProcessId::new(0, 0)));
}

#[test]
fn is_taken_false_for_occupied() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    // Slot occupied, not taken
    assert!(!table.is_taken(pid));
}

// ============================================================================
// Free List Reuse Tests
// ============================================================================

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

// ============================================================================
// Fragment Inbox Tests
// ============================================================================

/// Helper to create a fragment with a given base address and message.
fn make_fragment(base: u64, msg: crate::term::Term) -> HeapFragment {
    let mut frag = HeapFragment::new(Vaddr::new(base), 64);
    frag.set_message(msg);
    frag
}

#[test]
fn push_fragment_and_take() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    let msg = crate::term::Term::TRUE;
    table.push_fragment(pid, Box::new(make_fragment(0x5000, msg)));

    let frags = table.take_fragments(pid).unwrap();
    assert_eq!(frags.message(), msg);
    assert!(frags.next.is_none());

    // Inbox is now empty
    assert!(table.take_fragments(pid).is_none());
}

#[test]
fn push_multiple_fragments_forms_linked_list() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    let msg1 = crate::term::Term::small_int(1).unwrap();
    let msg2 = crate::term::Term::small_int(2).unwrap();
    let msg3 = crate::term::Term::small_int(3).unwrap();

    table.push_fragment(pid, Box::new(make_fragment(0x5000, msg1)));
    table.push_fragment(pid, Box::new(make_fragment(0x6000, msg2)));
    table.push_fragment(pid, Box::new(make_fragment(0x7000, msg3)));

    // Fragments are prepended, so order is reversed: 3, 2, 1
    let head = table.take_fragments(pid).unwrap();
    assert_eq!(head.message(), msg3);
    let f2 = head.next.as_ref().unwrap();
    assert_eq!(f2.message(), msg2);
    let f1 = f2.next.as_ref().unwrap();
    assert_eq!(f1.message(), msg1);
    assert!(f1.next.is_none());
}

#[test]
fn push_fragment_to_taken_slot() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    // Take process (simulates execution on another worker)
    let _taken = table.take(pid).unwrap();
    assert!(table.is_taken(pid));

    // Push fragment to taken slot — this should work
    let msg = crate::term::Term::TRUE;
    table.push_fragment(pid, Box::new(make_fragment(0x5000, msg)));

    let frags = table.take_fragments(pid).unwrap();
    assert_eq!(frags.message(), msg);
}

#[test]
fn push_fragment_to_invalid_pid_ignored() {
    let mut table = ProcessTable::new();

    // Push to unallocated slot — silent no-op
    let pid = ProcessId::new(0, 99);
    table.push_fragment(
        pid,
        Box::new(make_fragment(0x5000, crate::term::Term::TRUE)),
    );

    // Push to null — silent no-op
    table.push_fragment(
        ProcessId::NULL,
        Box::new(make_fragment(0x5000, crate::term::Term::TRUE)),
    );
}

#[test]
fn take_fragments_from_empty_inbox() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    assert!(table.take_fragments(pid).is_none());
}

#[test]
fn take_fragments_stale_pid() {
    let mut table = ProcessTable::new();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    table.push_fragment(
        pid,
        Box::new(make_fragment(0x5000, crate::term::Term::TRUE)),
    );

    // Remove process (increments generation)
    table.remove(pid);

    // Stale PID should not retrieve fragments
    assert!(table.take_fragments(pid).is_none());
}
