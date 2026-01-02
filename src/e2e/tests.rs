//! E2E test cases for Lona on seL4.
//!
//! Each test function returns `Ok(())` on success or `Err(message)` on failure.

#[allow(unused_imports)]
use core::option::Option::{None, Some};
use core::result::Result::{self, Err, Ok};

use crate::types::{Paddr, Pid, Vaddr};

/// Test that VM initialization succeeds.
pub fn test_vm_init() -> Result<(), &'static str> {
    crate::init().map_err(|_| "VM init failed")
}

/// Test that boot info is available (placeholder - actual check needs bootinfo).
pub fn test_bootinfo_available() -> Result<(), &'static str> {
    // This is a placeholder - actual test would check bootinfo
    // The real test happens in root-task.rs which has access to bootinfo
    Ok(())
}

/// Test that serial output works.
pub fn test_serial_output() -> Result<(), &'static str> {
    // If we got here, serial output is already working
    // This test mostly exists to verify the test framework itself
    sel4::debug_println!("  [serial test message]");
    Ok(())
}

/// Test memory type newtype wrappers.
pub fn test_memory_types() -> Result<(), &'static str> {
    // Test Paddr
    let paddr = Paddr::new(0x1000);
    if paddr.as_u64() != 0x1000 {
        return Err("Paddr::new/as_u64 mismatch");
    }

    // Test Vaddr
    let vaddr = Vaddr::new(0x2000);
    if vaddr.as_u64() != 0x2000 {
        return Err("Vaddr::new/as_u64 mismatch");
    }

    // Test null addresses
    if !Paddr::null().is_null() {
        return Err("Paddr::null should be null");
    }
    if !Vaddr::null().is_null() {
        return Err("Vaddr::null should be null");
    }

    Ok(())
}

/// Test PID creation and manipulation.
pub fn test_pid_creation() -> Result<(), &'static str> {
    // Create a PID
    let pid = Pid::new(1, 42);

    // Verify components
    if pid.realm_id() != 1 {
        return Err("Pid::realm_id mismatch");
    }
    if pid.local_id() != 42 {
        return Err("Pid::local_id mismatch");
    }

    // Test null PID
    let null_pid = Pid::null();
    if !null_pid.is_null() {
        return Err("Pid::null should be null");
    }

    // Test same_realm
    let same_realm_pid = Pid::new(1, 100);
    if !pid.same_realm(same_realm_pid) {
        return Err("same_realm should be true for same realm_id");
    }

    let diff_realm_pid = Pid::new(2, 42);
    if pid.same_realm(diff_realm_pid) {
        return Err("same_realm should be false for different realm_id");
    }

    Ok(())
}

/// Test address type operations.
pub fn test_address_types() -> Result<(), &'static str> {
    // Test address arithmetic
    let addr = Vaddr::new(0x1000);
    let added = addr.add(0x500);
    if added.as_u64() != 0x1500 {
        return Err("Vaddr::add failed");
    }

    let subbed = added.sub(0x500);
    if subbed.as_u64() != 0x1000 {
        return Err("Vaddr::sub failed");
    }

    // Test alignment
    let unaligned = Vaddr::new(0x1001);
    match unaligned.is_aligned(0x1000) {
        Some(false) => {} // expected
        _ => return Err("0x1001 should not be 0x1000-aligned"),
    }

    match unaligned.align_up(0x1000) {
        Some(aligned) if aligned.as_u64() == 0x2000 => {}
        _ => return Err("align_up to 0x1000 from 0x1001 should be 0x2000"),
    }

    match Vaddr::new(0x1500).align_down(0x1000) {
        Some(down) if down.as_u64() == 0x1000 => {}
        _ => return Err("align_down to 0x1000 from 0x1500 should be 0x1000"),
    }

    Ok(())
}
