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

use sel4_root_task::{Never, root_task};

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
    // Print startup message to the debug console;
    sel4::debug_println!("Lona runtime initialized");
    sel4::debug_println!("Boot info at: {:p}", bootinfo.ptr());

    // Print basic boot information
    sel4::debug_println!("Untyped memory regions: {}", bootinfo.untyped_list().len());

    sel4::debug_println!("Lona starting...");

    // For now, just halt. In the future, this will:
    // 1. Initialize the memory allocator
    // 2. Set up the Lonala compiler/interpreter
    // 3. Start the init process
    // 4. Enter the scheduler loop
    //
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
