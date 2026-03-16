// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for segmented `ProcessTable`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::process_table::{ProcessTable, SEGMENT_SIZE};
use crate::Vaddr;
use crate::process::heap_fragment::HeapFragment;
use crate::process::{Process, ProcessId};

/// Create a test process with default heap configuration.
fn create_test_process() -> Process {
    Process::new(Vaddr::new(0x1000), 0x1000, Vaddr::new(0x2000), 0x1000)
}

/// Create a `ProcessTable` with one segment pre-allocated.
fn table_with_one_segment() -> ProcessTable {
    let mut table = ProcessTable::new();
    let segment = ProcessTable::alloc_test_segment();
    unsafe { table.grow_segment(segment) };
    table
}

// ============================================================================
// Segmented table initialization tests
// ============================================================================

#[test]
fn new_table_is_empty() {
    let table = ProcessTable::new();
    assert_eq!(table.count(), 0);
    assert_eq!(table.num_segments(), 0);
    assert_eq!(table.capacity(), 0);
    assert!(table.is_full()); // No segments means no free slots
    assert!(!table.has_free_slots());
}

#[test]
fn grow_segment_adds_capacity() {
    let mut table = ProcessTable::new();
    assert_eq!(table.capacity(), 0);

    let segment = ProcessTable::alloc_test_segment();
    unsafe { table.grow_segment(segment) };

    assert_eq!(table.num_segments(), 1);
    assert_eq!(table.capacity(), SEGMENT_SIZE);
    assert!(table.has_free_slots());
    assert!(!table.is_full());
}

#[test]
fn grow_multiple_segments() {
    let mut table = ProcessTable::new();

    let seg1 = ProcessTable::alloc_test_segment();
    let seg2 = ProcessTable::alloc_test_segment();
    unsafe {
        table.grow_segment(seg1);
        table.grow_segment(seg2);
    }

    assert_eq!(table.num_segments(), 2);
    assert_eq!(table.capacity(), 2 * SEGMENT_SIZE);
}

// ============================================================================
// Allocate / insert / get tests
// ============================================================================

#[test]
fn allocate_and_insert() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    assert_eq!(table.count(), 1);
    assert!(table.get(pid).is_some());
}

#[test]
fn allocate_returns_none_when_empty() {
    let mut table = ProcessTable::new();
    assert!(table.allocate().is_none());
}

#[test]
fn get_stale_generation() {
    let mut table = table_with_one_segment();

    // Allocate, insert, remove
    let (index, gen1) = table.allocate().unwrap();
    let pid1 = ProcessId::new(index, gen1);
    let mut p1 = create_test_process();
    p1.pid = pid1;
    table.insert(p1);
    table.remove(pid1);

    // Allocate again (reuses same slot with new generation)
    let (index2, gen2) = table.allocate().unwrap();
    assert_eq!(index, index2);
    assert_eq!(gen2, gen1 + 1);

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
fn remove_returns_process() {
    let mut table = table_with_one_segment();
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
fn get_mut() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    process.ip = 0;
    table.insert(process);

    table.get_mut(pid).unwrap().ip = 42;
    assert_eq!(table.get(pid).unwrap().ip, 42);
}

#[test]
fn remove_invalid_pid() {
    let mut table = table_with_one_segment();
    let pid = ProcessId::new(0, 0);
    assert!(table.remove(pid).is_none());
}

#[test]
fn get_null_pid() {
    let table = ProcessTable::new();
    assert!(table.get(ProcessId::NULL).is_none());
}

#[test]
fn multiple_allocations() {
    let mut table = table_with_one_segment();

    let (idx1, gen1) = table.allocate().unwrap();
    let (idx2, gen2) = table.allocate().unwrap();
    let (idx3, gen3) = table.allocate().unwrap();

    assert_ne!(idx1, idx2);
    assert_ne!(idx2, idx3);
    assert_ne!(idx1, idx3);

    assert_eq!(gen1, 0);
    assert_eq!(gen2, 0);
    assert_eq!(gen3, 0);
}

// ============================================================================
// Take / Put Back Tests
// ============================================================================

#[test]
fn take_extracts_process() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    process.ip = 42;
    table.insert(process);

    let taken = table.take(pid).unwrap();
    assert_eq!(taken.ip, 42);

    assert!(table.get(pid).is_none());
    assert!(table.is_taken(pid));
    assert_eq!(table.count(), 1);
}

#[test]
fn put_back_restores_process() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    process.ip = 42;
    table.insert(process);

    let taken = table.take(pid).unwrap();
    table.put_back(pid, taken);

    let proc = table.get(pid).unwrap();
    assert_eq!(proc.ip, 42);
    assert!(!table.is_taken(pid));
}

#[test]
fn free_taken_slot_reclaims() {
    let mut table = table_with_one_segment();
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
    let mut table = table_with_one_segment();
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
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    let taken = table.take(pid).unwrap();
    table.put_back(pid, taken);

    assert!(table.get(pid).is_some());
}

#[test]
fn is_taken_false_for_unallocated() {
    let table = table_with_one_segment();
    assert!(!table.is_taken(ProcessId::new(0, 0)));
}

#[test]
fn is_taken_false_for_occupied() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    assert!(!table.is_taken(pid));
}

// ============================================================================
// Free List Reuse Tests
// ============================================================================

#[test]
fn free_list_reuse() {
    let mut table = table_with_one_segment();

    let (idx1, _) = table.allocate().unwrap();
    let (idx2, _) = table.allocate().unwrap();
    let (idx3, _) = table.allocate().unwrap();

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
    assert_eq!(gen_new, 1);
}

// ============================================================================
// Cross-segment Tests
// ============================================================================

#[test]
fn allocations_span_segments() {
    let mut table = ProcessTable::new();

    // Add first segment and exhaust it
    let seg1 = ProcessTable::alloc_test_segment();
    unsafe { table.grow_segment(seg1) };

    for i in 0..SEGMENT_SIZE {
        let (index, generation) = table.allocate().unwrap();
        let pid = ProcessId::new(index, generation);
        let mut p = create_test_process();
        p.pid = pid;
        table.insert(p);
        assert_eq!(index as usize, i);
    }

    // Table is now full — add second segment
    assert!(!table.has_free_slots());
    let seg2 = ProcessTable::alloc_test_segment();
    unsafe { table.grow_segment(seg2) };

    // Next allocation should be in second segment
    let (index, generation) = table.allocate().unwrap();
    assert_eq!(index as usize, SEGMENT_SIZE);
    let pid = ProcessId::new(index, generation);
    let mut p = create_test_process();
    p.pid = pid;
    table.insert(p);

    assert_eq!(table.count(), SEGMENT_SIZE + 1);
}

#[test]
fn free_list_chains_across_segments() {
    let mut table = ProcessTable::new();

    // Add first segment, allocate one slot
    let seg1 = ProcessTable::alloc_test_segment();
    unsafe { table.grow_segment(seg1) };
    let (idx1, _) = table.allocate().unwrap();
    let pid1 = ProcessId::new(idx1, 0);
    let mut p1 = create_test_process();
    p1.pid = pid1;
    table.insert(p1);

    // Add second segment (its slots are prepended to free list)
    let seg2 = ProcessTable::alloc_test_segment();
    unsafe { table.grow_segment(seg2) };

    // Next allocation should come from second segment (prepended)
    let (idx2, _) = table.allocate().unwrap();
    assert_eq!(idx2 as usize, SEGMENT_SIZE); // First slot of second segment

    // Remove from first segment
    table.remove(pid1);

    // Should be able to allocate from the freed slot
    let (idx3, gen3) = table.allocate().unwrap();
    assert_eq!(idx3, idx1);
    assert_eq!(gen3, 1); // Generation incremented
}

#[test]
fn get_across_segments() {
    let mut table = ProcessTable::new();
    let seg1 = ProcessTable::alloc_test_segment();
    let seg2 = ProcessTable::alloc_test_segment();
    unsafe {
        table.grow_segment(seg1);
        table.grow_segment(seg2);
    }

    // Fill first segment
    let mut pids = Vec::new();
    for _ in 0..SEGMENT_SIZE {
        let (idx, generation) = table.allocate().unwrap();
        let pid = ProcessId::new(idx, generation);
        let mut p = create_test_process();
        p.pid = pid;
        table.insert(p);
        pids.push(pid);
    }

    // Allocate in second segment
    let (idx, generation) = table.allocate().unwrap();
    let pid_seg2 = ProcessId::new(idx, generation);
    let mut p = create_test_process();
    p.pid = pid_seg2;
    p.ip = 999;
    table.insert(p);

    // Verify access in both segments
    assert!(table.get(pids[0]).is_some());
    assert!(table.get(pids[SEGMENT_SIZE - 1]).is_some());
    assert_eq!(table.get(pid_seg2).unwrap().ip, 999);
}

// ============================================================================
// Fragment Inbox Tests
// ============================================================================

fn make_fragment(base: u64, msg: crate::term::Term) -> HeapFragment {
    let mut frag = HeapFragment::new(Vaddr::new(base), 64);
    frag.set_message(msg);
    frag
}

#[test]
fn push_fragment_and_take() {
    let mut table = table_with_one_segment();
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

    assert!(table.take_fragments(pid).is_none());
}

#[test]
fn push_multiple_fragments_forms_linked_list() {
    let mut table = table_with_one_segment();
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
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    let _taken = table.take(pid).unwrap();
    assert!(table.is_taken(pid));

    let msg = crate::term::Term::TRUE;
    table.push_fragment(pid, Box::new(make_fragment(0x5000, msg)));

    let frags = table.take_fragments(pid).unwrap();
    assert_eq!(frags.message(), msg);
}

#[test]
fn push_fragment_to_invalid_pid_ignored() {
    let mut table = table_with_one_segment();

    let pid = ProcessId::new(0, 99);
    table.push_fragment(
        pid,
        Box::new(make_fragment(0x5000, crate::term::Term::TRUE)),
    );

    table.push_fragment(
        ProcessId::NULL,
        Box::new(make_fragment(0x5000, crate::term::Term::TRUE)),
    );
}

#[test]
fn take_fragments_from_empty_inbox() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    assert!(table.take_fragments(pid).is_none());
}

#[test]
fn take_fragments_stale_pid() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    table.push_fragment(
        pid,
        Box::new(make_fragment(0x5000, crate::term::Term::TRUE)),
    );

    table.remove(pid);
    assert!(table.take_fragments(pid).is_none());
}

// ============================================================================
// Pending Signal Tests
// ============================================================================

#[test]
fn push_and_take_pending_signals() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    // Take the process (simulates execution on a worker)
    let _taken = table.take(pid).unwrap();

    let sender = ProcessId::new(99, 0);
    let reason = crate::term::Term::TRUE;
    table.push_pending_signal(pid, sender, reason);

    let signals = table.take_pending_signals(pid);
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].0, sender);
    assert_eq!(signals[0].1, reason);

    // Queue should be empty after take
    let signals2 = table.take_pending_signals(pid);
    assert!(signals2.is_empty());
}

#[test]
fn pending_signals_cleared_on_free_taken_slot() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    let _taken = table.take(pid).unwrap();
    table.push_pending_signal(pid, ProcessId::new(99, 0), crate::term::Term::TRUE);

    table.free_taken_slot(pid);

    // Reuse the slot — should have no pending signals
    let (idx2, gen2) = table.allocate().unwrap();
    assert_eq!(idx2, index);
    let new_pid = ProcessId::new(idx2, gen2);
    let mut p2 = create_test_process();
    p2.pid = new_pid;
    table.insert(p2);

    let signals = table.take_pending_signals(new_pid);
    assert!(signals.is_empty());
}

#[test]
fn pending_signals_cleared_on_remove() {
    let mut table = table_with_one_segment();
    let (index, generation) = table.allocate().unwrap();
    let pid = ProcessId::new(index, generation);

    let mut process = create_test_process();
    process.pid = pid;
    table.insert(process);

    table.push_pending_signal(pid, ProcessId::new(99, 0), crate::term::Term::TRUE);
    table.remove(pid);

    // Reuse the slot — should have no pending signals
    let (idx2, gen2) = table.allocate().unwrap();
    let new_pid = ProcessId::new(idx2, gen2);
    let mut p2 = create_test_process();
    p2.pid = new_pid;
    table.insert(p2);

    let signals = table.take_pending_signals(new_pid);
    assert!(signals.is_empty());
}

#[test]
fn push_pending_signal_to_invalid_pid_ignored() {
    let mut table = table_with_one_segment();
    table.push_pending_signal(
        ProcessId::new(0, 99),
        ProcessId::new(1, 0),
        crate::term::Term::TRUE,
    );
    table.push_pending_signal(
        ProcessId::NULL,
        ProcessId::new(1, 0),
        crate::term::Term::TRUE,
    );
    // No panic = success
}
