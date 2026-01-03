// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! VM state management tests.
//!
//! Tests that verify the stateful behavior of the test VM,
//! including heap isolation and multi-operation sequences.

// Test code prioritizes clarity over defensive programming
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use common::TestVm;

// ============================================================================
// VM Isolation
// ============================================================================

#[test]
fn vm_instances_are_isolated() {
    let mut vm1 = TestVm::new();
    let mut vm2 = TestVm::new();

    // Allocate in vm1
    let _ = vm1.read_and_eval("(1 2 3)").unwrap();
    let used1 = vm1.heap_used();

    // vm2 should still be fresh
    assert_eq!(vm2.heap_used(), 0, "vm2 should have empty heap");

    // Allocate same thing in vm2
    let _ = vm2.read_and_eval("(1 2 3)").unwrap();
    let used2 = vm2.heap_used();

    // Both should have used same amount (independent heaps)
    assert_eq!(used1, used2, "both VMs should use same heap space");
}

#[test]
fn fresh_vm_has_empty_heap() {
    let vm = TestVm::new();
    assert_eq!(vm.heap_used(), 0);
    assert!(vm.heap_remaining() > 0);
}

// ============================================================================
// Stateful Operations
// ============================================================================

#[test]
fn multiple_reads_accumulate_heap_usage() {
    let mut vm = TestVm::new();

    let initial = vm.heap_used();
    assert_eq!(initial, 0);

    // First allocation
    let _ = vm.read_and_eval("(1 2 3)").unwrap();
    let after_first = vm.heap_used();
    assert!(
        after_first > 0,
        "heap should be used after first allocation"
    );

    // Second allocation
    let _ = vm.read_and_eval("(4 5 6)").unwrap();
    let after_second = vm.heap_used();
    assert!(
        after_second > after_first,
        "heap usage should increase with more allocations"
    );
}

#[test]
fn heap_remaining_decreases_with_allocations() {
    let mut vm = TestVm::new();

    let initial_remaining = vm.heap_remaining();

    let _ = vm.read_and_eval("(1 2 3)").unwrap();
    let after_alloc = vm.heap_remaining();

    assert!(
        after_alloc < initial_remaining,
        "remaining heap should decrease after allocation"
    );
}

#[test]
fn values_persist_across_operations() {
    let mut vm = TestVm::new();

    // Read multiple values - they should all be valid
    let v1 = vm.read_and_eval("42").unwrap();
    let v2 = vm.read_and_eval("(1 2)").unwrap();
    let v3 = vm.read_and_eval("\"hello\"").unwrap();

    // All values should still print correctly
    assert_eq!(vm.print(v1), "42");
    assert_eq!(vm.print(v2), "(1 2)");
    assert_eq!(vm.print(v3), "\"hello\"");
}

// ============================================================================
// Custom Heap Size
// ============================================================================

#[test]
fn custom_heap_size() {
    let small_heap_size = 1024;
    let vm = TestVm::with_heap_size(small_heap_size);

    // Should have roughly the specified heap size available
    assert!(vm.heap_remaining() <= small_heap_size);
    assert!(vm.heap_remaining() > 0);
}
