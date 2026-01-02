//! Lona Root Task
//!
//! This is the initial task that seL4 loads at boot.
//! It initializes the Lona VM runtime and starts the root realm.
//!
//! When compiled with the `e2e-test` feature, this instead runs
//! the E2E test suite and reports results via serial output.

#![no_std]
#![no_main]
#![allow(clippy::panic)] // Entry point needs to handle errors somehow

#[allow(unused_imports)]
use core::result::Result::Err;

use sel4_root_task::root_task;

/// Entry point for the Lona root task.
///
/// This function is called by seL4 after the kernel has initialized.
/// It receives boot information containing memory regions, capabilities,
/// and other system configuration.
#[root_task]
fn main(bootinfo: &sel4::BootInfoPtr) -> ! {
    // When e2e-test feature is enabled, run tests instead of normal boot
    #[cfg(feature = "e2e-test")]
    {
        run_e2e_tests(bootinfo);
    }

    // Normal boot path (when not running tests)
    #[cfg(not(feature = "e2e-test"))]
    {
        normal_boot(bootinfo);
    }
}

/// Normal boot sequence for production use.
#[cfg(not(feature = "e2e-test"))]
fn normal_boot(bootinfo: &sel4::BootInfoPtr) -> ! {
    // Initialize the Lona VM runtime
    if let Err(_e) = lona_vm::init() {
        // TODO: Log error once we have serial output
        // For now, just halt
        sel4::init_thread::suspend_self()
    }

    // Get boot info for memory setup
    let _info = bootinfo.inner();

    // TODO: Future initialization steps:
    // 1. Parse boot info to find available memory
    // 2. Set up memory allocator
    // 3. Create root realm
    // 4. Load and execute init process

    sel4::debug_println!("Lona booted successfully");
    sel4::debug_println!("Version: {}", lona_vm::VERSION);

    // For now, just suspend
    sel4::init_thread::suspend_self()
}

/// Run E2E tests and report results.
#[cfg(feature = "e2e-test")]
fn run_e2e_tests(bootinfo: &sel4::BootInfoPtr) -> ! {
    sel4::debug_println!("");
    sel4::debug_println!("========================================");
    sel4::debug_println!("  Lona E2E Test Suite");
    sel4::debug_println!("  Version: {}", lona_vm::VERSION);
    sel4::debug_println!("========================================");
    sel4::debug_println!("");

    // Verify we have boot info (this is a basic sanity check)
    let _info = bootinfo.inner();
    sel4::debug_println!("Boot info received successfully");
    sel4::debug_println!("");

    // Run all E2E tests
    let all_passed = lona_vm::e2e::run_all_tests();

    sel4::debug_println!("");
    if all_passed {
        sel4::debug_println!("All tests passed!");
    } else {
        sel4::debug_println!("Some tests failed!");
    }
    sel4::debug_println!("");

    // Suspend - test harness will detect completion and kill QEMU
    sel4::init_thread::suspend_self()
}
