// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the VM interpreter.

use lona_core::chunk::Chunk;
use lona_core::symbol::Interner;

use super::interpreter::Vm;

mod arithmetic_tests;
mod call_tests;
mod comparison_tests;
mod control_flow_tests;
mod literal_tests;

/// Creates a VM with a fresh interner for testing.
pub(super) fn make_vm(interner: &Interner) -> Vm<'_> {
    Vm::new(interner)
}

/// Creates a test chunk.
pub(super) fn make_chunk() -> Chunk {
    Chunk::new()
}
