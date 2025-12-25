// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Lonala bytecode compiler.
//!
//! This module is organized into focused submodules:
//! - `literal_tests` - literal and symbol compilation
//! - `operator_tests` - arithmetic, comparison, and unary operators
//! - `call_tests` - function calls and error handling
//! - `special_form_tests` - control flow forms (do, if)
//! - `binding_form_tests` - binding forms (def, let)
//! - `quote_form_tests` - quoting forms (quote, syntax-quote)
//! - `macro_tests` - defmacro and macro expansion
//! - `destructure` - destructuring pattern parsing and bytecode compilation
//! - `tail_call_tests` - tail call optimization (TailCall opcode emission)
//! - `case_tests` - case special form pattern matching

extern crate alloc;

mod binding_form_tests;
mod call_tests;
mod case_tests;
mod destructure;
mod literal_tests;
mod macro_tests;
mod operator_tests;
mod quote_form_tests;
mod special_form_tests;
mod tail_call_tests;

use lona_core::chunk::Chunk;
use lona_core::source;
use lona_core::symbol;

use crate::compiler::compile;

/// Test source ID for all compiler tests.
pub(super) const TEST_SOURCE_ID: source::Id = source::Id::new(0_u32);

/// Helper to compile source and return the chunk.
pub(super) fn compile_source(source: &str) -> Chunk {
    let mut interner = symbol::Interner::new();
    compile(source, TEST_SOURCE_ID, &mut interner).expect("compilation should succeed")
}

/// Helper to compile and return chunk + interner for symbol checks.
pub(super) fn compile_with_interner(source: &str) -> (Chunk, symbol::Interner) {
    let mut interner = symbol::Interner::new();
    let chunk = compile(source, TEST_SOURCE_ID, &mut interner).expect("compilation should succeed");
    (chunk, interner)
}
