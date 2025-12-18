// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Lona Runtime - The root task for Lona on seL4.
//!
//! This crate implements the initial root task that runs on the seL4 microkernel.
//! It receives all system capabilities from the kernel and is responsible for
//! bootstrapping Lona.
//!
//! # Architecture
//!
//! The Lona runtime is the first userspace process started by seL4. It:
//!
//! 1. Receives the boot info structure containing all initial capabilities
//! 2. Initializes the memory allocator using untyped memory capabilities
//! 3. Sets up the Lonala language compiler/interpreter
//! 4. Spawns the init process and other system services
//! 5. Enters the main scheduler loop
//!
//! # Safety
//!
//! As the root task, this code runs with maximum privilege in userspace.
//! All capability operations must be carefully validated to maintain
//! the security guarantees of the seL4 microkernel.

#![no_std]
#![no_main]

extern crate alloc;

#[cfg(feature = "integration-test")]
mod integration_tests;
mod memory;
mod platform;
mod repl;

use alloc::vec;

use lona_core::allocator::Allocator;
use sel4_root_task::{Never, root_task};

use crate::memory::Sel4PageProvider;

/// Global page provider for seL4 memory allocation.
static PAGE_PROVIDER: Sel4PageProvider = Sel4PageProvider::new();

/// Global allocator for Rust's `alloc` crate.
///
/// Initialized in `main` before any heap allocation occurs.
///
/// TODO: This global allocator is a temporary bootstrap solution for Phase 1.
/// In Phase 7 (Process Data Structure) and Phase 9 (Garbage Collection),
/// this will be replaced with per-process heaps to enable independent GC
/// and proper memory isolation between domains.
#[global_allocator]
static ALLOCATOR: Allocator<&Sel4PageProvider> = Allocator::new(&PAGE_PROVIDER);

/// Entry point for the Lona runtime.
///
/// This function is called by the seL4 kernel after boot. It receives the
/// boot info structure containing all initial capabilities and memory
/// information needed to bootstrap the system.
///
/// # Arguments
///
/// * `bootinfo` - Pointer to the seL4 boot info structure containing:
///   - Initial thread's TCB, `CNode`, `VSpace`, and ASID pool capabilities
///   - Untyped memory capabilities for dynamic allocation
///   - Device memory regions
///   - Kernel reserved memory regions
#[root_task]
fn main(bootinfo: &sel4::BootInfoPtr) -> sel4::Result<Never> {
    sel4::debug_println!("Lona runtime starting...");

    // Print basic boot information
    sel4::debug_println!("Boot info at: {:p}", bootinfo.ptr());
    sel4::debug_println!("Untyped memory regions: {}", bootinfo.untyped_list().len());

    // Initialize the memory allocator
    // SAFETY: Called once at startup, bootinfo remains valid, single-threaded
    unsafe {
        PAGE_PROVIDER.init(bootinfo);
    }
    sel4::debug_println!("Memory allocator initialized");

    // Initialize UART for real serial output
    init_uart(bootinfo);

    // Test heap allocation to verify the allocator works
    test_allocation();

    println!("Lona runtime initialized successfully");
    println!("Hello from allocator + UART");

    // Run integration tests if enabled, otherwise start the REPL
    #[cfg(feature = "integration-test")]
    {
        integration_tests::run_integration_tests();
        integration_tests::halt_loop()
    }

    // Start the interactive REPL (never returns)
    #[cfg(not(feature = "integration-test"))]
    {
        let mut interactive = repl::InteractiveRepl::new(repl::UartConsole);
        interactive.run()
    }
}

/// Initializes the UART driver for serial output.
///
/// Discovers UART address from FDT and maps device memory.
fn init_uart(bootinfo: &sel4::BootInfoPtr) {
    // Discover UART from FDT in bootinfo
    let uart_info = match platform::fdt::discover_uart(bootinfo) {
        Ok(info) => {
            sel4::debug_println!(
                "Found UART at paddr 0x{:x}, size 0x{:x}",
                info.paddr,
                info.size
            );
            info
        }
        Err(err) => {
            sel4::debug_println!("Warning: UART discovery failed: {:?}", err);
            return;
        }
    };

    // Initialize the UART driver
    // SAFETY: PAGE_PROVIDER is initialized, single-threaded context
    let success = unsafe { platform::uart::init(uart_info, &PAGE_PROVIDER) };

    if success {
        // First message via UART!
        println!("UART initialized successfully");
    } else {
        sel4::debug_println!("Warning: UART initialization failed");
    }
}

/// Tests that heap allocation is working correctly.
fn test_allocation() {
    sel4::debug_println!("Testing heap allocation...");

    // Create a vector to test allocation
    let test_vec = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];

    // Verify the contents
    sel4::debug_println!("Allocated vector: {:?}", test_vec.as_slice());

    // Check allocator stats
    let stats = ALLOCATOR.stats();
    sel4::debug_println!(
        "Allocator stats: {} bytes in {} pages ({} bytes reserved)",
        stats.total_allocated,
        stats.pages_allocated,
        stats.total_reserved()
    );

    // Verify page provider stats match
    sel4::debug_println!("Page provider frames: {}", PAGE_PROVIDER.frames_allocated());

    // Allocate some more to verify ongoing allocation works
    let another_vec: alloc::vec::Vec<u32> = (0..100).collect();
    sel4::debug_println!("Second allocation: {} elements", another_vec.len());

    let stats = ALLOCATOR.stats();
    sel4::debug_println!(
        "Final allocator stats: {} bytes in {} pages",
        stats.total_allocated,
        stats.pages_allocated
    );
}
