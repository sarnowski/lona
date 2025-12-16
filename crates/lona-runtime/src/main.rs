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

mod memory;

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

    // Test heap allocation to verify the allocator works
    test_allocation();

    sel4::debug_println!("Lona runtime initialized successfully");

    // For now, just halt. In the future, this will:
    // 1. Set up the Lonala compiler/interpreter
    // 2. Start the init process
    // 3. Enter the scheduler loop
    #[expect(
        clippy::infinite_loop,
        reason = "Root task must never exit - seL4 expects this to run forever"
    )]
    loop {
        // SAFETY: WFI (Wait For Interrupt) is safe to execute - it simply
        // puts the CPU into a low-power state until an interrupt occurs.
        // Since we have no interrupt handlers set up yet, this effectively
        // halts the system cleanly.
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Tests that heap allocation is working correctly.
fn test_allocation() {
    sel4::debug_println!("Testing heap allocation...");

    // Create a vector to test allocation
    let test_vec = vec![1, 2, 3, 4, 5];

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
