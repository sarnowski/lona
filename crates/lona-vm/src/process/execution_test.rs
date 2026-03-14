// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for execution state (`chunk_addr`, `write_chunk_to_heap`, reset).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::allocation_test::setup;
use super::*;
use crate::bytecode::{Chunk, encode_abx, op};

#[test]
fn process_write_chunk_to_heap() {
    let (mut proc, mut mem) = setup();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42));
    chunk.emit(encode_abx(op::HALT, 0, 0));

    assert!(proc.write_chunk_to_heap(&mut mem, &chunk));
    assert!(proc.chunk_addr.is_some());
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

/// Regression test: `write_chunk_to_heap()` must set `chunk_addr` to the new chunk.
///
/// Ensures that writing a new chunk replaces any stale `chunk_addr` and resets IP.
#[test]
fn regression_write_chunk_replaces_chunk_addr() {
    let (mut proc, mut mem) = setup();

    // Simulate a heap-saved chunk address from a previous function call
    proc.chunk_addr = Some(crate::Vaddr::new(0x1234_5678));

    // Write a new chunk (as happens when entering a new function)
    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::HALT, 0, 0));
    assert!(proc.write_chunk_to_heap(&mut mem, &chunk));

    // Verify chunk_addr was updated to the new chunk, not the stale one
    assert_ne!(
        proc.chunk_addr,
        Some(crate::Vaddr::new(0x1234_5678)),
        "write_chunk_to_heap() must replace stale chunk_addr"
    );
    assert!(
        proc.chunk_addr.is_some(),
        "chunk_addr must be set after write_chunk_to_heap"
    );
    assert_eq!(proc.ip, 0, "IP should be reset to start of new chunk");
}
