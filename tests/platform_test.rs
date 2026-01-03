// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Platform infrastructure tests.
//!
//! Tests for low-level VM infrastructure: memory layout, address types,
//! and mock `VSpace` behavior.

// Test code prioritizes clarity over defensive programming
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use lona_vm::platform::MockVSpace;
use lona_vm::platform::vspace_layout;
use lona_vm::{MemorySpace, Paddr, Vaddr, init};

// ============================================================================
// VM Initialization
// ============================================================================

#[test]
fn vm_init_succeeds() {
    let result = init();
    assert!(result.is_ok());
}

// ============================================================================
// VSpace Layout Constants
// ============================================================================

#[test]
fn vspace_layout_regions_are_ordered() {
    // Verify that VSpace regions are in ascending order
    assert!(vspace_layout::NULL_GUARD < vspace_layout::GLOBAL_CONTROL);
    assert!(vspace_layout::GLOBAL_CONTROL < vspace_layout::SCHEDULER_STATE);
    assert!(vspace_layout::SCHEDULER_STATE < vspace_layout::NAMESPACE_RO);
    assert!(vspace_layout::NAMESPACE_RO < vspace_layout::NAMESPACE_RW);
    assert!(vspace_layout::NAMESPACE_RW < vspace_layout::NAMESPACE_OBJECTS);
    assert!(vspace_layout::NAMESPACE_OBJECTS < vspace_layout::ANCESTOR_CODE);
    assert!(vspace_layout::ANCESTOR_CODE < vspace_layout::LOCAL_CODE);
    assert!(vspace_layout::LOCAL_CODE < vspace_layout::PROCESS_HEAPS);
    assert!(vspace_layout::PROCESS_HEAPS < vspace_layout::SHARED_BINARY);
    assert!(vspace_layout::SHARED_BINARY < vspace_layout::CROSS_REALM_SHARED);
    assert!(vspace_layout::CROSS_REALM_SHARED < vspace_layout::DEVICE_MAPPINGS);
    assert!(vspace_layout::DEVICE_MAPPINGS < vspace_layout::KERNEL_RESERVED);
}

// ============================================================================
// Address Type Safety
// ============================================================================

#[test]
fn paddr_and_vaddr_have_same_value() {
    let paddr = Paddr::new(0x1000);
    let vaddr = Vaddr::new(0x1000);

    // Same numeric value, different types
    assert_eq!(paddr.as_u64(), vaddr.as_u64());
}

#[test]
fn address_arithmetic() {
    let paddr = Paddr::new(0x1000);
    let vaddr = Vaddr::new(0x1000);

    let paddr2 = paddr.add(0x100);
    let vaddr2 = vaddr.add(0x100);

    assert_eq!(paddr2.as_u64(), 0x1100);
    assert_eq!(vaddr2.as_u64(), 0x1100);
}

// ============================================================================
// MockVSpace: Process Simulation
// ============================================================================

/// Process header structure for testing memory layout.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ProcessHeader {
    pid: u64,
    status: u32,
    _pad: u32,
    heap_start: u64,
    heap_ptr: u64,
    stack_start: u64,
    stack_ptr: u64,
}

#[test]
fn mock_vspace_process_simulation() {
    let process_size: usize = 64 * 1024;
    let base = vspace_layout::PROCESS_HEAPS;
    let mut vspace = MockVSpace::new(process_size, base);

    let header_size = core::mem::size_of::<ProcessHeader>();
    let stack_start = base.add(header_size as u64);
    let heap_start = base.add(process_size as u64);

    // Write initial process header
    let header = ProcessHeader {
        pid: 0x0000_0001_0000_002A, // realm=1, local=42
        status: 1,
        _pad: 0,
        heap_start: heap_start.as_u64(),
        heap_ptr: heap_start.as_u64(),
        stack_start: stack_start.as_u64(),
        stack_ptr: stack_start.as_u64(),
    };
    vspace.write(base, header);

    // Simulate heap allocation (grows down)
    let alloc_size: u64 = 256;
    let mut proc: ProcessHeader = vspace.read(base);
    proc.heap_ptr -= alloc_size;
    vspace.write(base, proc);

    // Simulate stack push (grows up)
    let stack_frame_size: u64 = 64;
    let mut proc: ProcessHeader = vspace.read(base);
    proc.stack_ptr += stack_frame_size;
    vspace.write(base, proc);

    // Verify final state
    let final_proc: ProcessHeader = vspace.read(base);
    assert_eq!(final_proc.heap_ptr, heap_start.as_u64() - alloc_size);
    assert_eq!(
        final_proc.stack_ptr,
        stack_start.as_u64() + stack_frame_size
    );
    // Heap and stack should not have collided
    assert!(final_proc.heap_ptr > final_proc.stack_ptr);
}
