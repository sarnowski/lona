// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! LMM (Lona Memory Manager) fault handling E2E test cases.
//!
//! These tests verify the production memory allocation strategy:
//! - Explicit IPC allocation for ProcessPool, RealmBinary, RealmLocal
//! - Pre-mapped worker stacks (no faults expected)
//! - Fault-based lazy mapping ONLY for inherited regions
//!
//! NOTE: General demand paging for ProcessPool/stacks was removed because seL4 MCS
//! delivers Timeout faults (label=6) instead of VMFaults (label=5) when budget
//! expires during fault handling, causing infinite timeout loops. The production
//! solution uses explicit IPC for most regions, with lazy mapping only for inherited
//! regions (which is required because parents can't push updates to unknown children).
//!
//! Test functions return `Ok(())` on success or `Err(message)` on failure.

use core::result::Result::{self, Err, Ok};

use crate::platform::MemorySpace;
use crate::platform::lmm::lmm_request_pages;
use crate::process::Process;
use crate::uart::Uart;
use lona_abi::ipc::IpcRegionType;

// =============================================================================
// Explicit IPC Allocation Tests
// =============================================================================

/// Test that explicit IPC allocation is the correct way to get ProcessPool memory.
///
/// This test demonstrates the production pattern: call `lmm_request_pages()` to
/// explicitly allocate memory rather than relying on fault-based demand paging.
/// This is the ONLY supported way to allocate ProcessPool memory.
pub fn test_explicit_ipc_allocation<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    sel4::debug_println!("  Testing explicit IPC allocation (production pattern)...");

    // Allocate via explicit IPC - this is the correct production pattern
    let alloc1 = match lmm_request_pages(IpcRegionType::ProcessPool, 2, None) {
        Ok(v) => v,
        Err(e) => {
            sel4::debug_println!("  First IPC allocation failed: {:?}", e);
            return Err("First IPC allocation failed");
        }
    };
    sel4::debug_println!("  IPC allocation 1: 0x{:x}", alloc1.as_u64());

    // Allocate more via explicit IPC
    let alloc2 = match lmm_request_pages(IpcRegionType::ProcessPool, 2, None) {
        Ok(v) => v,
        Err(e) => {
            sel4::debug_println!("  Second IPC allocation failed: {:?}", e);
            return Err("Second IPC allocation failed");
        }
    };
    sel4::debug_println!("  IPC allocation 2: 0x{:x}", alloc2.as_u64());

    // Verify allocations are usable
    const PATTERN1: u64 = 0xDEAD_BEEF_CAFE_BABE;
    const PATTERN2: u64 = 0x1234_5678_9ABC_DEF0;

    unsafe {
        let ptr1 = alloc1.as_u64() as *mut u64;
        let ptr2 = alloc2.as_u64() as *mut u64;

        ptr1.write_volatile(PATTERN1);
        ptr2.write_volatile(PATTERN2);

        let read1 = ptr1.read_volatile();
        let read2 = ptr2.read_volatile();

        if read1 != PATTERN1 {
            sel4::debug_println!(
                "  ERROR: Read back 0x{:x}, expected 0x{:x}",
                read1,
                PATTERN1
            );
            return Err("First allocation verification failed");
        }
        if read2 != PATTERN2 {
            sel4::debug_println!(
                "  ERROR: Read back 0x{:x}, expected 0x{:x}",
                read2,
                PATTERN2
            );
            return Err("Second allocation verification failed");
        }
    }

    sel4::debug_println!("  Explicit IPC allocation verified");
    Ok(())
}

/// Test that worker stacks are pre-mapped and don't require allocation.
///
/// Worker stacks are fully mapped at realm creation. This test verifies
/// that stack usage works correctly without needing explicit allocation.
///
/// NOTE: This test does NOT rely on demand paging. Stacks are pre-mapped
/// at realm creation to avoid MCS timing issues with fault handling.
pub fn test_premapped_stack<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    sel4::debug_println!("  Testing pre-mapped stack (no demand paging needed)...");

    // Recursive function that uses stack space
    // Each call uses some stack for local variables and return address
    fn recursive_stack_user(depth: u32, accumulator: u64) -> u64 {
        // Use some stack space with local arrays
        let local_data: [u64; 8] = [depth as u64; 8]; // 64 bytes
        let sum: u64 = local_data.iter().sum();

        if depth == 0 {
            accumulator + sum
        } else {
            // Recursive call uses more stack
            recursive_stack_user(depth - 1, accumulator + sum)
        }
    }

    // With pre-mapped stacks (64KB by default), this recursion depth should work fine.
    // With ~128 bytes per frame, 64 calls use about 8KB of stack.
    const RECURSION_DEPTH: u32 = 64;

    let result = recursive_stack_user(RECURSION_DEPTH, 0);

    // Verify we got a sensible result (not corrupted)
    if result == 0 {
        return Err("Stack recursion returned zero (unexpected)");
    }

    sel4::debug_println!(
        "  Recursive function completed (depth={}, result={})",
        RECURSION_DEPTH,
        result
    );
    sel4::debug_println!("  Pre-mapped stack verified (no faults triggered)");

    Ok(())
}

/// Test interleaved explicit IPC allocations.
///
/// This test verifies that multiple explicit IPC allocations work correctly
/// when interleaved with other operations. This is the production pattern
/// for memory allocation.
pub fn test_interleaved_explicit_allocation<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    sel4::debug_println!("  Testing interleaved explicit IPC allocations...");

    // Allocate via explicit IPC
    let ipc_alloc1 = match lmm_request_pages(IpcRegionType::ProcessPool, 2, None) {
        Ok(v) => v,
        Err(e) => {
            sel4::debug_println!("  First IPC allocation failed: {:?}", e);
            return Err("First IPC allocation failed");
        }
    };
    sel4::debug_println!("  IPC allocation 1: 0x{:x}", ipc_alloc1.as_u64());

    // Use stack (stacks are pre-mapped, no demand paging)
    fn use_stack_space(n: u32) -> u64 {
        let data: [u64; 16] = [n as u64; 16];
        data.iter().sum()
    }

    let stack_result1 = use_stack_space(42);

    // Allocate more via explicit IPC
    let ipc_alloc2 = match lmm_request_pages(IpcRegionType::ProcessPool, 2, None) {
        Ok(v) => v,
        Err(e) => {
            sel4::debug_println!("  Second IPC allocation failed: {:?}", e);
            return Err("Second IPC allocation failed");
        }
    };
    sel4::debug_println!("  IPC allocation 2: 0x{:x}", ipc_alloc2.as_u64());

    // Use more stack
    let stack_result2 = use_stack_space(100);

    // Verify IPC allocations are still usable
    unsafe {
        let ptr1 = ipc_alloc1.as_u64() as *mut u64;
        let ptr2 = ipc_alloc2.as_u64() as *mut u64;

        ptr1.write_volatile(stack_result1);
        ptr2.write_volatile(stack_result2);

        let read1 = ptr1.read_volatile();
        let read2 = ptr2.read_volatile();

        if read1 != stack_result1 || read2 != stack_result2 {
            return Err("Interleaved allocation verification failed");
        }
    }

    sel4::debug_println!(
        "  Interleaved allocation verified (stack results: {}, {})",
        stack_result1,
        stack_result2
    );

    Ok(())
}
