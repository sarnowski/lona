// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for execution state (chunks, reset).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::allocation_test::setup;
use super::*;
use crate::bytecode::{Chunk, encode_abx, op};
use crate::value::Value;

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
    proc.x_regs[0] = Value::int(42);
    proc.status = ProcessStatus::Running;

    // Reset
    proc.reset();

    assert_eq!(proc.ip, 0);
    assert_eq!(proc.x_regs[0], Value::Nil);
    assert_eq!(proc.status, ProcessStatus::Ready);
}
