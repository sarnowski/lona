// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! E2E test cases for Lona on seL4.
//!
//! Each test function receives the same heap, memory space, and UART
//! that the REPL uses, ensuring tests exercise the exact same code paths.
//!
//! Test functions return `Ok(())` on success or `Err(message)` on failure.

use core::option::Option::{None, Some};
use core::result::Result::{self, Err, Ok};

use crate::heap::Heap;
use crate::platform::MemorySpace;
use crate::reader::read;
use crate::types::{Paddr, Vaddr};
use crate::uart::Uart;
use crate::value::print_value;

/// Test that VM initialization succeeds.
pub fn test_vm_init<M: MemorySpace, U: Uart>(
    _heap: &mut Heap,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    // VM initialization is a no-op - just verify the test framework works
    Ok(())
}

/// Test that serial output works.
pub fn test_serial_output<M: MemorySpace, U: Uart>(
    _heap: &mut Heap,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    // If we got here, serial output is already working
    // This test mostly exists to verify the test framework itself
    sel4::debug_println!("  [serial test message]");
    Ok(())
}

/// Test memory type newtype wrappers.
pub fn test_memory_types<M: MemorySpace, U: Uart>(
    _heap: &mut Heap,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
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

/// Test address type operations.
pub fn test_address_types<M: MemorySpace, U: Uart>(
    _heap: &mut Heap,
    _mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
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

/// Maximum size for output buffer when capturing printed values.
const OUTPUT_BUFFER_SIZE: usize = 256;

/// A simple buffer that implements Uart for capturing output.
struct OutputBuffer {
    data: [u8; OUTPUT_BUFFER_SIZE],
    len: usize,
}

impl OutputBuffer {
    const fn new() -> Self {
        Self {
            data: [0; OUTPUT_BUFFER_SIZE],
            len: 0,
        }
    }

    fn as_str(&self) -> Result<&str, &'static str> {
        core::str::from_utf8(&self.data[..self.len]).map_err(|_| "output not valid UTF-8")
    }
}

impl Uart for OutputBuffer {
    fn write_byte(&mut self, byte: u8) {
        if self.len < self.data.len() {
            self.data[self.len] = byte;
            self.len += 1;
        }
    }

    fn read_byte(&mut self) -> u8 {
        0 // Not used for output capture
    }

    fn can_read(&self) -> bool {
        false
    }

    fn can_write(&self) -> bool {
        self.len < self.data.len()
    }
}

/// Test reading and printing a quoted list.
///
/// This test exercises the same code path as the REPL:
/// 1. `read()` parses the input string into a Value
/// 2. `print_value()` converts the Value back to a string
///
/// Input: `'(1 2 3)` (quoted list)
/// Expected output: `(quote (1 2 3))`
pub fn test_read_quoted_list<M: MemorySpace, U: Uart>(
    heap: &mut Heap,
    mem: &mut M,
    _uart: &mut U,
) -> Result<(), &'static str> {
    // Read the input using the same function as the REPL
    let value = match read("'(1 2 3)", heap, mem) {
        Ok(Some(v)) => v,
        Ok(None) => return Err("read returned None"),
        Err(_) => return Err("read failed"),
    };

    // Print the value using the same function as the REPL
    let mut output = OutputBuffer::new();
    print_value(value, heap, mem, &mut output);

    // Verify the output matches expected
    let printed = output.as_str()?;
    if printed != "(quote (1 2 3))" {
        sel4::debug_println!("  Expected: (quote (1 2 3))");
        sel4::debug_println!("  Got: {}", printed);
        return Err("output mismatch");
    }

    Ok(())
}
