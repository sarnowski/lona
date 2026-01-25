// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for execution state (chunks, reset).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::allocation_test::setup;
use super::*;
use crate::bytecode::{Chunk, encode_abx, op};

#[test]
fn process_set_chunk() {
    let (mut proc, _mem) = setup();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42));
    chunk.emit(encode_abx(op::HALT, 0, 0));

    proc.set_chunk(chunk);

    assert!(proc.chunk.is_some());
    assert_eq!(proc.ip, 0);
}

#[test]
fn process_reset() {
    let (mut proc, _mem) = setup();

    // Modify state
    proc.ip = 100;
    proc.status = ProcessStatus::Running;

    // Reset
    proc.reset();

    assert_eq!(proc.ip, 0);
    assert_eq!(proc.status, ProcessStatus::Ready);
}

/// Regression test: `set_chunk()` must clear `chunk_addr` to prevent stale heap references.
///
/// Before the fix, consecutive function calls in spec tests would fail with
/// `:ip-out-of-bounds` because `ensure_chunk_on_heap()` would skip saving the new
/// chunk when a stale `chunk_addr` remained from a previous call.
#[test]
fn regression_set_chunk_clears_chunk_addr() {
    let (mut proc, _mem) = setup();

    // Simulate a heap-saved chunk address from a previous function call
    proc.chunk_addr = Some(crate::Vaddr::new(0x1234_5678));

    // Set a new chunk (as happens when entering a new function)
    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::HALT, 0, 0));
    proc.set_chunk(chunk);

    // Verify chunk_addr was cleared - this is critical for correct function call behavior
    assert!(
        proc.chunk_addr.is_none(),
        "set_chunk() must clear chunk_addr to prevent stale heap references"
    );
    assert_eq!(proc.ip, 0, "IP should be reset to start of new chunk");
}
