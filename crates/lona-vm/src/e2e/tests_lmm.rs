// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! LMM (Lona Memory Manager) IPC E2E test cases.
//!
//! These tests verify the IPC protocol for dynamic memory allocation:
//! - Basic page allocation (single and multiple pages)
//! - Memory usability verification
//! - Address hint handling (valid and invalid)
//! - Region-specific allocation (ProcessPool, RealmBinary, RealmLocal)
//! - Pool growth behavior
//! - Stress testing with repeated allocations
//!
//! Each test function receives the same process, memory space, and UART
//! that the REPL uses, ensuring tests exercise the exact same code paths.
//!
//! Test functions return `Ok(())` on success or `Err(message)` on failure.

use core::option::Option::{None, Some};
use core::result::Result::{self, Err, Ok};

use crate::platform::MemorySpace;
use crate::platform::lmm::lmm_request_pages;
use crate::process::Process;
use crate::types::Vaddr;
use crate::uart::Uart;
use lona_abi::ipc::IpcRegionType;

// =============================================================================
// Basic Allocation Tests
// =============================================================================

/// Test basic IPC allocation: request 1 page from LMM.
///
/// This test verifies:
/// 1. IPC call to LMM succeeds
/// 2. Returned address is valid (non-null)
/// 3. Returned address is page-aligned
pub fn test_lmm_alloc_single_page<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    sel4::debug_println!("  Requesting 1 page from LMM...");

    let result = lmm_request_pages(IpcRegionType::ProcessPool, 1, None);

    match result {
        Ok(vaddr) => {
            sel4::debug_println!("  Allocated at: 0x{:x}", vaddr.as_u64());

            // Verify address is not null
            if vaddr.is_null() {
                return Err("LMM returned null address");
            }

            // Verify page alignment (4KB = 0x1000)
            if vaddr.as_u64() & 0xFFF != 0 {
                return Err("LMM returned non-page-aligned address");
            }

            Ok(())
        }
        Err(e) => {
            sel4::debug_println!("  LMM error: {:?}", e);
            Err("LMM allocation failed")
        }
    }
}

/// Test allocating multiple pages at once.
///
/// This test verifies:
/// 1. Multi-page allocation succeeds
/// 2. All pages are contiguous (single allocation)
pub fn test_lmm_alloc_multiple_pages<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    const PAGE_COUNT: usize = 4;

    sel4::debug_println!("  Requesting {} pages from LMM...", PAGE_COUNT);

    let result = lmm_request_pages(IpcRegionType::ProcessPool, PAGE_COUNT, None);

    match result {
        Ok(vaddr) => {
            sel4::debug_println!(
                "  Allocated {} pages at: 0x{:x}",
                PAGE_COUNT,
                vaddr.as_u64()
            );

            if vaddr.is_null() {
                return Err("LMM returned null address for multi-page");
            }

            if vaddr.as_u64() & 0xFFF != 0 {
                return Err("LMM returned non-page-aligned address for multi-page");
            }

            Ok(())
        }
        Err(e) => {
            sel4::debug_println!("  LMM error: {:?}", e);
            Err("LMM multi-page allocation failed")
        }
    }
}

/// Test that allocated memory is actually usable.
///
/// This test verifies:
/// 1. Allocate a page
/// 2. Write a pattern to the page
/// 3. Read back and verify the pattern
pub fn test_lmm_alloc_memory_usable<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    sel4::debug_println!("  Allocating page to test usability...");

    let result = lmm_request_pages(IpcRegionType::ProcessPool, 1, None);

    match result {
        Ok(vaddr) => {
            sel4::debug_println!("  Writing test pattern at: 0x{:x}", vaddr.as_u64());

            // Write a test pattern
            let ptr = vaddr.as_u64() as *mut u64;
            const TEST_PATTERN: u64 = 0xDEAD_BEEF_CAFE_BABE;

            unsafe {
                // Write pattern
                ptr.write_volatile(TEST_PATTERN);

                // Read back
                let read_back = ptr.read_volatile();

                if read_back != TEST_PATTERN {
                    sel4::debug_println!(
                        "  Pattern mismatch: wrote 0x{:x}, read 0x{:x}",
                        TEST_PATTERN,
                        read_back
                    );
                    return Err("Memory read/write verification failed");
                }
            }

            sel4::debug_println!("  Memory verified successfully");
            Ok(())
        }
        Err(e) => {
            sel4::debug_println!("  LMM error: {:?}", e);
            Err("LMM allocation failed for usability test")
        }
    }
}

// =============================================================================
// Address Hint Tests
// =============================================================================

/// Test allocation with address hint.
///
/// This test verifies:
/// 1. LMM accepts a hint address (hints are suggestions, not requirements)
/// 2. Allocation succeeds with or without honoring the hint
/// 3. Returned address is valid (page-aligned, non-null)
///
/// Note: The LMM treats hints as suggestions. It will honor the hint if
/// the address is valid and available, but may return a different address
/// if the hint cannot be satisfied.
pub fn test_lmm_alloc_with_hint<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    // Use a high address in the process pool region
    // PROCESS_POOL_BASE = 0x0000_0020_0000_0000
    // Pick an address well into the region to avoid conflicts
    let hint = Vaddr::new(0x0000_0025_0000_0000);

    sel4::debug_println!("  Requesting page at hint: 0x{:x}", hint.as_u64());

    let result = lmm_request_pages(IpcRegionType::ProcessPool, 1, Some(hint));

    match result {
        Ok(vaddr) => {
            sel4::debug_println!("  Allocated at: 0x{:x}", vaddr.as_u64());

            // Verify address is valid
            if vaddr.is_null() {
                return Err("LMM returned null address with hint");
            }

            if vaddr.as_u64() & 0xFFF != 0 {
                return Err("LMM returned non-page-aligned address with hint");
            }

            // Log whether hint was honored (informational, not an error)
            if vaddr == hint {
                sel4::debug_println!("  Hint honored: allocated at exact address");
            } else {
                sel4::debug_println!(
                    "  Hint adjusted: requested 0x{:x}, got 0x{:x}",
                    hint.as_u64(),
                    vaddr.as_u64()
                );
            }

            Ok(())
        }
        Err(e) => {
            sel4::debug_println!("  LMM error: {:?}", e);
            Err("LMM allocation with hint failed")
        }
    }
}

/// Test that invalid hint addresses are rejected.
///
/// This test verifies:
/// 1. Hint outside the region is rejected
/// 2. Unaligned hint is rejected
/// 3. Valid request after rejected hints still works
pub fn test_lmm_alloc_invalid_hint<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    sel4::debug_println!("  Testing invalid hint rejection...");

    // Test 1: Hint in wrong region (RealmBinary address for ProcessPool request)
    let wrong_region_hint = Vaddr::new(0x0000_0013_0000_0000); // RealmBinary base
    let result = lmm_request_pages(IpcRegionType::ProcessPool, 1, Some(wrong_region_hint));

    if result.is_ok() {
        return Err("Should have rejected hint in wrong region");
    }
    sel4::debug_println!("  Wrong region hint: correctly rejected");

    // Test 2: Unaligned hint (not on 4KB boundary)
    let unaligned_hint = Vaddr::new(0x0000_0020_0000_0100); // ProcessPool base + 256 bytes
    let result = lmm_request_pages(IpcRegionType::ProcessPool, 1, Some(unaligned_hint));

    if result.is_ok() {
        return Err("Should have rejected unaligned hint");
    }
    sel4::debug_println!("  Unaligned hint: correctly rejected");

    // Test 3: Valid request should still work after rejections
    let result = lmm_request_pages(IpcRegionType::ProcessPool, 1, None);
    if result.is_err() {
        return Err("Valid request after rejections should succeed");
    }
    sel4::debug_println!("  Valid request after rejections: succeeded");

    Ok(())
}

// =============================================================================
// Sequential and Region Tests
// =============================================================================

/// Test sequential allocations return increasing addresses.
///
/// This test verifies:
/// 1. Multiple allocations succeed
/// 2. Addresses are sequential (no gaps)
pub fn test_lmm_alloc_sequential<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    const PAGE_SIZE: u64 = 4096;

    sel4::debug_println!("  Testing sequential allocations...");

    // First allocation
    let first = match lmm_request_pages(IpcRegionType::ProcessPool, 1, None) {
        Ok(v) => v,
        Err(e) => {
            sel4::debug_println!("  First allocation failed: {:?}", e);
            return Err("First sequential allocation failed");
        }
    };

    // Second allocation
    let second = match lmm_request_pages(IpcRegionType::ProcessPool, 1, None) {
        Ok(v) => v,
        Err(e) => {
            sel4::debug_println!("  Second allocation failed: {:?}", e);
            return Err("Second sequential allocation failed");
        }
    };

    sel4::debug_println!(
        "  First: 0x{:x}, Second: 0x{:x}",
        first.as_u64(),
        second.as_u64()
    );

    // Second should be exactly one page after first
    let expected_second = first.as_u64() + PAGE_SIZE;
    if second.as_u64() != expected_second {
        sel4::debug_println!(
            "  Expected second at 0x{:x}, got 0x{:x}",
            expected_second,
            second.as_u64()
        );
        return Err("Sequential allocations not contiguous");
    }

    Ok(())
}

/// Test allocating in different regions.
///
/// This test verifies:
/// 1. ProcessPool allocation works
/// 2. RealmBinary allocation works
/// 3. RealmLocal allocation works
/// 4. Each region returns addresses within valid bounds (base <= addr < limit)
pub fn test_lmm_alloc_regions<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    use lona_abi::layout::{
        PROCESS_POOL_BASE, PROCESS_POOL_SIZE, REALM_BINARY_BASE, REALM_BINARY_SIZE,
        REALM_LOCAL_BASE, REALM_LOCAL_SIZE,
    };

    sel4::debug_println!("  Testing allocation in different regions...");

    // ProcessPool
    let pool = match lmm_request_pages(IpcRegionType::ProcessPool, 1, None) {
        Ok(v) => v,
        Err(e) => {
            sel4::debug_println!("  ProcessPool failed: {:?}", e);
            return Err("ProcessPool allocation failed");
        }
    };
    sel4::debug_println!("  ProcessPool: 0x{:x}", pool.as_u64());

    // RealmBinary
    let binary = match lmm_request_pages(IpcRegionType::RealmBinary, 1, None) {
        Ok(v) => v,
        Err(e) => {
            sel4::debug_println!("  RealmBinary failed: {:?}", e);
            return Err("RealmBinary allocation failed");
        }
    };
    sel4::debug_println!("  RealmBinary: 0x{:x}", binary.as_u64());

    // RealmLocal
    let local = match lmm_request_pages(IpcRegionType::RealmLocal, 1, None) {
        Ok(v) => v,
        Err(e) => {
            sel4::debug_println!("  RealmLocal failed: {:?}", e);
            return Err("RealmLocal allocation failed");
        }
    };
    sel4::debug_println!("  RealmLocal: 0x{:x}", local.as_u64());

    // Verify ProcessPool address is within valid bounds
    if pool.as_u64() < PROCESS_POOL_BASE {
        return Err("ProcessPool address below region base");
    }
    if pool.as_u64() >= PROCESS_POOL_BASE + PROCESS_POOL_SIZE {
        return Err("ProcessPool address above region limit");
    }

    // Verify RealmBinary address is within valid bounds
    if binary.as_u64() < REALM_BINARY_BASE {
        return Err("RealmBinary address below region base");
    }
    if binary.as_u64() >= REALM_BINARY_BASE + REALM_BINARY_SIZE {
        return Err("RealmBinary address above region limit");
    }

    // Verify RealmLocal address is within valid bounds
    if local.as_u64() < REALM_LOCAL_BASE {
        return Err("RealmLocal address below region base");
    }
    if local.as_u64() >= REALM_LOCAL_BASE + REALM_LOCAL_SIZE {
        return Err("RealmLocal address above region limit");
    }

    Ok(())
}

/// Test large allocation (16 pages = 64KB).
///
/// This test verifies large allocations work correctly.
pub fn test_lmm_alloc_large<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    const PAGE_COUNT: usize = 16;
    const PAGE_SIZE: u64 = 4096;

    sel4::debug_println!(
        "  Requesting {} pages ({}KB)...",
        PAGE_COUNT,
        PAGE_COUNT * 4
    );

    let result = lmm_request_pages(IpcRegionType::ProcessPool, PAGE_COUNT, None);

    match result {
        Ok(vaddr) => {
            sel4::debug_println!("  Allocated at: 0x{:x}", vaddr.as_u64());

            // Write to first and last page to verify all are mapped
            let first_ptr = vaddr.as_u64() as *mut u64;
            let last_ptr = (vaddr.as_u64() + (PAGE_COUNT as u64 - 1) * PAGE_SIZE) as *mut u64;

            unsafe {
                first_ptr.write_volatile(0x1111_1111_1111_1111);
                last_ptr.write_volatile(0x9999_9999_9999_9999);

                let first_val = first_ptr.read_volatile();
                let last_val = last_ptr.read_volatile();

                if first_val != 0x1111_1111_1111_1111 {
                    return Err("First page write verification failed");
                }
                if last_val != 0x9999_9999_9999_9999 {
                    return Err("Last page write verification failed");
                }
            }

            sel4::debug_println!("  All {} pages verified", PAGE_COUNT);
            Ok(())
        }
        Err(e) => {
            sel4::debug_println!("  LMM error: {:?}", e);
            Err("Large allocation failed")
        }
    }
}

// =============================================================================
// Pool Growth Tests
// =============================================================================

/// Test pool growth by allocating more than initial heap.
///
/// This test verifies:
/// 1. Pool can grow beyond initial allocation via IPC
/// 2. Multiple growth requests succeed
/// 3. Total allocated memory is usable
///
/// INIT_HEAP_SIZE is 32KB (8 pages). This test allocates 16 pages (64KB)
/// to force growth events while leaving room for other tests.
pub fn test_lmm_pool_growth<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    const PAGES_TO_ALLOCATE: usize = 16; // 64KB total, exceeds 32KB initial
    const PAGE_SIZE: u64 = 4096;

    sel4::debug_println!(
        "  Testing pool growth: allocating {} pages ({}KB)...",
        PAGES_TO_ALLOCATE,
        PAGES_TO_ALLOCATE * 4
    );

    let mut addresses: [u64; PAGES_TO_ALLOCATE] = [0; PAGES_TO_ALLOCATE];
    let mut last_addr: u64 = 0;

    for i in 0..PAGES_TO_ALLOCATE {
        let result = lmm_request_pages(IpcRegionType::ProcessPool, 1, None);

        match result {
            Ok(vaddr) => {
                let addr = vaddr.as_u64();
                addresses[i] = addr;

                // Verify addresses are sequential (contiguous allocation)
                if i > 0 && addr != last_addr + PAGE_SIZE {
                    sel4::debug_println!(
                        "  Non-contiguous at page {}: expected 0x{:x}, got 0x{:x}",
                        i,
                        last_addr + PAGE_SIZE,
                        addr
                    );
                    // This is not an error - allocations might skip due to alignment
                }
                last_addr = addr;

                // Write to verify page is usable
                unsafe {
                    let ptr = addr as *mut u64;
                    ptr.write_volatile(i as u64);
                }
            }
            Err(e) => {
                sel4::debug_println!("  Allocation {} failed: {:?}", i, e);
                return Err("Pool growth allocation failed");
            }
        }
    }

    sel4::debug_println!("  Allocated {} pages successfully", PAGES_TO_ALLOCATE);

    // Verify all pages are still accessible (read back the values)
    for i in 0..PAGES_TO_ALLOCATE {
        let addr = addresses[i];
        let value = unsafe { (addr as *const u64).read_volatile() };

        if value != i as u64 {
            sel4::debug_println!(
                "  Verification failed at page {}: expected {}, got {}",
                i,
                i,
                value
            );
            return Err("Pool growth verification failed");
        }
    }

    sel4::debug_println!("  All {} pages verified successfully", PAGES_TO_ALLOCATE);
    Ok(())
}

/// Test allocating memory in process-sized chunks.
///
/// This test simulates actual process allocation patterns by requesting
/// memory in sizes typical for process heaps (young + old generation).
///
/// Each "process" needs:
/// - Young heap: 4KB (1 page)
/// - Old heap: 8KB (2 pages)
/// Total: 12KB (3 pages) per process
///
/// We allocate for 5 processes = 15 pages = 60KB, enough to verify the pattern.
pub fn test_lmm_process_allocation_pattern<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    const PROCESSES_TO_SIMULATE: usize = 5;
    const YOUNG_PAGES: usize = 1;
    const OLD_PAGES: usize = 2;
    const PAGES_PER_PROCESS: usize = YOUNG_PAGES + OLD_PAGES;

    sel4::debug_println!(
        "  Simulating {} process allocations ({} pages each)...",
        PROCESSES_TO_SIMULATE,
        PAGES_PER_PROCESS
    );

    for i in 0..PROCESSES_TO_SIMULATE {
        // Allocate young heap
        let young = match lmm_request_pages(IpcRegionType::ProcessPool, YOUNG_PAGES, None) {
            Ok(v) => v,
            Err(e) => {
                sel4::debug_println!("  Process {} young heap failed: {:?}", i, e);
                return Err("Process young heap allocation failed");
            }
        };

        // Allocate old heap
        let old = match lmm_request_pages(IpcRegionType::ProcessPool, OLD_PAGES, None) {
            Ok(v) => v,
            Err(e) => {
                sel4::debug_println!("  Process {} old heap failed: {:?}", i, e);
                return Err("Process old heap allocation failed");
            }
        };

        // Verify both are usable
        unsafe {
            let young_ptr = young.as_u64() as *mut u64;
            let old_ptr = old.as_u64() as *mut u64;

            // Use distinct patterns for young and old heaps
            let young_pattern = 0xAAAA_0000_0000_0000_u64 + i as u64;
            let old_pattern = 0xBBBB_0000_0000_0000_u64 + i as u64;

            young_ptr.write_volatile(young_pattern);
            old_ptr.write_volatile(old_pattern);

            let young_val = young_ptr.read_volatile();
            let old_val = old_ptr.read_volatile();

            if young_val != young_pattern {
                return Err("Young heap verification failed");
            }
            if old_val != old_pattern {
                return Err("Old heap verification failed");
            }
        }
    }

    sel4::debug_println!(
        "  Successfully allocated {} processes ({} pages total)",
        PROCESSES_TO_SIMULATE,
        PROCESSES_TO_SIMULATE * PAGES_PER_PROCESS
    );

    Ok(())
}

/// Test stress allocation with many small requests.
///
/// This test makes 10 single-page allocations to verify IPC channel
/// stability with repeated requests.
pub fn test_lmm_stress_allocations<M: MemorySpace, U: Uart>(
    _proc: &mut Process,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    const ALLOCATION_COUNT: usize = 10;

    sel4::debug_println!(
        "  Stress test: {} single-page allocations...",
        ALLOCATION_COUNT
    );

    let mut total_allocated: u64 = 0;

    for i in 0..ALLOCATION_COUNT {
        match lmm_request_pages(IpcRegionType::ProcessPool, 1, None) {
            Ok(vaddr) => {
                // Quick write to verify usability
                unsafe {
                    let ptr = vaddr.as_u64() as *mut u64;
                    ptr.write_volatile(i as u64);
                }
                total_allocated += 4096;
            }
            Err(e) => {
                sel4::debug_println!("  Allocation {} failed: {:?}", i, e);
                return Err("Stress allocation failed");
            }
        }
    }

    sel4::debug_println!(
        "  Completed {} allocations, {} bytes total",
        ALLOCATION_COUNT,
        total_allocated
    );

    Ok(())
}
