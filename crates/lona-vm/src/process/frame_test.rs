// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for stack-based frame storage and Y register access.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::allocation_test::setup;
use super::*;
use crate::Vaddr;
use crate::platform::{MemorySpace, MockVSpace};
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

// --- allocate_frame tests ---

#[test]
fn allocate_frame_basic() {
    let (mut proc, mut mem) = setup();

    // Initial state
    assert!(proc.at_top_level());
    let initial_stop = proc.stop;

    // Allocate frame
    let result = proc.allocate_frame(&mut mem, 42, Vaddr::new(0x2000));
    assert!(result.is_ok());

    // Frame should exist
    assert!(!proc.at_top_level());
    assert!(proc.frame_base.is_some());
    assert_eq!(proc.current_y_count, 0);

    // Stack pointer should have moved down by header size
    assert_eq!(
        initial_stop.as_u64() - proc.stop.as_u64(),
        FRAME_HEADER_SIZE as u64
    );

    // frame_base should equal stop (no Y registers yet)
    assert_eq!(proc.frame_base, Some(proc.stop));
}

#[test]
fn allocate_frame_stores_correct_header() {
    let (mut proc, mut mem) = setup();

    proc.allocate_frame(&mut mem, 0xABCD, Vaddr::new(0xDEAD_BEEF))
        .unwrap();

    // Read header directly from memory
    let base = proc.frame_base.unwrap().as_u64();
    assert_eq!(
        mem.read::<u64>(Vaddr::new(base + frame_offset::RETURN_IP as u64)),
        0xABCD
    );
    assert_eq!(
        mem.read::<u64>(Vaddr::new(base + frame_offset::CHUNK_ADDR as u64)),
        0xDEAD_BEEF
    );
    // caller_frame_base should be 0 (top level)
    assert_eq!(
        mem.read::<u64>(Vaddr::new(base + frame_offset::CALLER_FRAME_BASE as u64)),
        0
    );
    assert_eq!(
        mem.read::<u64>(Vaddr::new(base + frame_offset::Y_COUNT as u64)),
        0
    );
}

#[test]
fn allocate_frame_nested_stores_caller_frame_base() {
    let (mut proc, mut mem) = setup();

    // First frame
    proc.allocate_frame(&mut mem, 10, Vaddr::new(0x1000))
        .unwrap();
    let first_frame_base = proc.frame_base.unwrap();

    // Second (nested) frame
    proc.allocate_frame(&mut mem, 20, Vaddr::new(0x2000))
        .unwrap();

    // Second frame should store first frame's base
    let base = proc.frame_base.unwrap().as_u64();
    assert_eq!(
        mem.read::<u64>(Vaddr::new(base + frame_offset::CALLER_FRAME_BASE as u64)),
        first_frame_base.as_u64()
    );
}

// --- deallocate_frame tests ---

#[test]
fn deallocate_frame_restores_context() {
    let (mut proc, mut mem) = setup();
    let initial_stop = proc.stop;

    // Allocate frame
    proc.allocate_frame(&mut mem, 100, Vaddr::new(0x5000))
        .unwrap();

    // Deallocate
    let result = proc.deallocate_frame(&mem);
    assert!(result.is_some());

    let (return_ip, chunk_addr) = result.unwrap();
    assert_eq!(return_ip, 100);
    assert_eq!(chunk_addr, Vaddr::new(0x5000));

    // Stack pointer restored
    assert_eq!(proc.stop, initial_stop);
    assert!(proc.at_top_level());
}

#[test]
fn deallocate_frame_at_top_level_returns_none() {
    let (mut proc, mem) = setup();

    // No frame allocated
    let result = proc.deallocate_frame(&mem);
    assert!(result.is_none());
}

#[test]
fn deallocate_nested_frames_restores_correctly() {
    let (mut proc, mut mem) = setup();

    // First frame
    proc.allocate_frame(&mut mem, 10, Vaddr::new(0x1000))
        .unwrap();
    let first_frame_base = proc.frame_base.unwrap();

    // Second frame
    proc.allocate_frame(&mut mem, 20, Vaddr::new(0x2000))
        .unwrap();

    // Deallocate second frame
    let (ip, addr) = proc.deallocate_frame(&mem).unwrap();
    assert_eq!(ip, 20);
    assert_eq!(addr, Vaddr::new(0x2000));

    // Should restore first frame's context
    assert_eq!(proc.frame_base, Some(first_frame_base));

    // Deallocate first frame
    let (ip, addr) = proc.deallocate_frame(&mem).unwrap();
    assert_eq!(ip, 10);
    assert_eq!(addr, Vaddr::new(0x1000));
    assert!(proc.at_top_level());
}

// --- Y register tests ---

#[test]
fn extend_frame_allocates_y_registers() {
    let (mut proc, mut mem) = setup();

    proc.allocate_frame(&mut mem, 0, Vaddr::new(0)).unwrap();
    let frame_base = proc.frame_base.unwrap();

    // Extend with 3 Y registers
    proc.extend_frame_y_regs(&mut mem, 3).unwrap();

    assert_eq!(proc.current_y_count, 3);
    // stop should be below frame_base
    assert_eq!(
        proc.stop.as_u64(),
        frame_base.as_u64() - 3 * Y_REGISTER_SIZE as u64
    );
}

#[test]
fn extend_frame_zero_initializes() {
    let (mut proc, mut mem) = setup();

    proc.allocate_frame(&mut mem, 0, Vaddr::new(0)).unwrap();
    proc.extend_frame_y_regs_zero(&mut mem, 3).unwrap();

    // All Y registers should be nil
    for i in 0..3 {
        assert_eq!(proc.get_y(&mem, i), Some(Term::NIL));
    }
}

#[test]
fn y_register_get_set() {
    let (mut proc, mut mem) = setup();

    proc.allocate_frame(&mut mem, 0, Vaddr::new(0)).unwrap();
    proc.extend_frame_y_regs_zero(&mut mem, 4).unwrap();

    // Set values
    assert!(proc.set_y(&mut mem, 0, int(42)));
    assert!(proc.set_y(&mut mem, 3, int(99)));

    // Get values
    assert_eq!(proc.get_y(&mem, 0), Some(int(42)));
    assert_eq!(proc.get_y(&mem, 3), Some(int(99)));

    // Unset registers should still be nil
    assert_eq!(proc.get_y(&mem, 1), Some(Term::NIL));
    assert_eq!(proc.get_y(&mem, 2), Some(Term::NIL));
}

#[test]
fn y_register_out_of_bounds() {
    let (mut proc, mut mem) = setup();

    proc.allocate_frame(&mut mem, 0, Vaddr::new(0)).unwrap();
    proc.extend_frame_y_regs(&mut mem, 2).unwrap();

    // Out of bounds access
    assert_eq!(proc.get_y(&mem, 2), None);
    assert_eq!(proc.get_y(&mem, 100), None);
    assert!(!proc.set_y(&mut mem, 2, int(1)));
    assert!(!proc.set_y(&mut mem, 100, int(1)));
}

#[test]
fn y_register_no_frame() {
    let (proc, mem) = setup();

    // No frame allocated
    assert_eq!(proc.get_y(&mem, 0), None);
}

#[test]
fn shrink_frame_releases_y_registers() {
    let (mut proc, mut mem) = setup();

    proc.allocate_frame(&mut mem, 0, Vaddr::new(0)).unwrap();
    let frame_base = proc.frame_base.unwrap();

    proc.extend_frame_y_regs(&mut mem, 3).unwrap();
    assert_eq!(proc.current_y_count, 3);

    // Shrink frame
    proc.shrink_frame_y_regs(&mut mem, 3).unwrap();

    assert_eq!(proc.current_y_count, 0);
    assert_eq!(proc.stop, frame_base);
}

#[test]
fn nested_frames_preserve_y_registers() {
    let (mut proc, mut mem) = setup();

    // Outer frame with Y registers
    proc.allocate_frame(&mut mem, 10, Vaddr::new(0x1000))
        .unwrap();
    proc.extend_frame_y_regs_zero(&mut mem, 2).unwrap();
    proc.set_y(&mut mem, 0, int(111));
    proc.set_y(&mut mem, 1, int(222));

    // Inner frame
    proc.allocate_frame(&mut mem, 20, Vaddr::new(0x2000))
        .unwrap();
    proc.extend_frame_y_regs_zero(&mut mem, 1).unwrap();
    proc.set_y(&mut mem, 0, int(333));

    // Inner frame has different Y0
    assert_eq!(proc.get_y(&mem, 0), Some(int(333)));

    // Shrink and pop inner frame
    proc.shrink_frame_y_regs(&mut mem, 1).unwrap();
    proc.deallocate_frame(&mem).unwrap();

    // Outer frame's Y registers preserved
    assert_eq!(proc.get_y(&mem, 0), Some(int(111)));
    assert_eq!(proc.get_y(&mem, 1), Some(int(222)));
}

// --- Stack overflow tests ---

#[test]
fn stack_overflow_detection() {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(4096, base);
    // Very small heap/stack region
    let mut proc = Process::new(base, 512, base.add(512), 256);

    // Fill the stack by allocating frames
    let mut count = 0;
    loop {
        let result = proc.allocate_frame(&mut mem, 0, Vaddr::new(0));
        if result.is_err() {
            break;
        }
        count += 1;
        assert!(count <= 100, "Stack overflow not detected");
    }

    // We should have allocated some frames before overflow
    assert!(count > 0, "Should allocate at least one frame");
}

#[test]
fn y_register_overflow_detection() {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(4096, base);
    // Small heap/stack region
    let mut proc = Process::new(base, 512, base.add(512), 256);

    proc.allocate_frame(&mut mem, 0, Vaddr::new(0)).unwrap();

    // Try to allocate too many Y registers
    let result = proc.extend_frame_y_regs(&mut mem, MAX_Y_REGISTERS + 1);
    assert!(result.is_err());
}

// --- Call depth tests ---

#[test]
fn call_depth_tracking() {
    let (mut proc, mut mem) = setup();

    assert_eq!(proc.call_depth(), 0);

    proc.allocate_frame(&mut mem, 0, Vaddr::new(0)).unwrap();
    assert_eq!(proc.call_depth(), 1);

    proc.allocate_frame(&mut mem, 0, Vaddr::new(0)).unwrap();
    assert_eq!(proc.call_depth(), 2);

    proc.deallocate_frame(&mem);
    assert_eq!(proc.call_depth(), 1);

    proc.deallocate_frame(&mem);
    assert_eq!(proc.call_depth(), 0);
}

// --- Regression tests ---

/// Regression test: `Y_REGISTER_SIZE` must equal `size_of::<Term>()`.
///
/// Before fix: `STACK_SLOT_SIZE` was 8 bytes but `Value` was 16 bytes.
/// Now Y registers store 8-byte Terms, so this should match.
#[test]
fn regression_y_register_size_matches_term_size() {
    // Verify the constant is defined correctly
    assert_eq!(
        Y_REGISTER_SIZE,
        core::mem::size_of::<Term>(),
        "Y_REGISTER_SIZE must equal size_of::<Term>() to prevent memory corruption"
    );
}

/// Regression test: Y register writes must not corrupt frame header.
///
/// Before fix: Writing to the last Y register would overflow into the
/// frame header, corrupting `return_ip`/`chunk_addr`/`caller_frame_base`.
#[test]
fn regression_y_register_write_does_not_corrupt_header() {
    let (mut proc, mut mem) = setup();

    // Allocate frame with known header values
    let return_ip = 0xDEAD_BEEF_usize;
    let chunk_addr = Vaddr::new(0xCAFE_BABE);
    proc.allocate_frame(&mut mem, return_ip, chunk_addr)
        .unwrap();
    let frame_base = proc.frame_base.unwrap();

    // Allocate Y registers
    proc.extend_frame_y_regs_zero(&mut mem, 4).unwrap();

    // Write to ALL Y registers, including the last one (Y3)
    // Before fix, Y3 write would corrupt return_ip in header
    proc.set_y(&mut mem, 0, int(100));
    proc.set_y(&mut mem, 1, int(200));
    proc.set_y(&mut mem, 2, int(300));
    proc.set_y(&mut mem, 3, int(400));

    // Verify Y registers are correct
    assert_eq!(proc.get_y(&mem, 0), Some(int(100)));
    assert_eq!(proc.get_y(&mem, 1), Some(int(200)));
    assert_eq!(proc.get_y(&mem, 2), Some(int(300)));
    assert_eq!(proc.get_y(&mem, 3), Some(int(400)));

    // CRITICAL: Verify frame header is NOT corrupted
    let stored_return_ip: u64 = mem.read(Vaddr::new(
        frame_base.as_u64() + frame_offset::RETURN_IP as u64,
    ));
    let stored_chunk_addr: u64 = mem.read(Vaddr::new(
        frame_base.as_u64() + frame_offset::CHUNK_ADDR as u64,
    ));

    assert_eq!(
        stored_return_ip, return_ip as u64,
        "Frame header return_ip was corrupted by Y register write"
    );
    assert_eq!(
        stored_chunk_addr,
        chunk_addr.as_u64(),
        "Frame header chunk_addr was corrupted by Y register write"
    );

    // Deallocate should work correctly with uncorrupted header
    let result = proc.deallocate_frame(&mem);
    assert!(
        result.is_some(),
        "deallocate_frame failed after Y register writes"
    );
    let (ip, addr) = result.unwrap();
    assert_eq!(ip, return_ip, "Returned wrong return_ip");
    assert_eq!(addr, chunk_addr, "Returned wrong chunk_addr");
}
