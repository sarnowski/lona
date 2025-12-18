// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Lonala bytecode compiler.
//!
//! This module is organized into focused submodules:
//! - `literal_tests` - literal and symbol compilation
//! - `operator_tests` - arithmetic, comparison, and unary operators
//! - `call_tests` - function calls and error handling
//! - `special_form_tests` - do, if, def, let, quote, syntax-quote
//! - `macro_tests` - defmacro and macro expansion

extern crate alloc;

mod call_tests;
mod literal_tests;
mod macro_tests;
mod operator_tests;
mod special_form_tests;

use lona_core::chunk::Chunk;
use lona_core::symbol;

use crate::compiler::compile;

/// Helper to compile source and return the chunk.
pub(super) fn compile_source(source: &str) -> Chunk {
    let mut interner = symbol::Interner::new();
    compile(source, &mut interner).expect("compilation should succeed")
}

/// Helper to compile and return chunk + interner for symbol checks.
pub(super) fn compile_with_interner(source: &str) -> (Chunk, symbol::Interner) {
    let mut interner = symbol::Interner::new();
    let chunk = compile(source, &mut interner).expect("compilation should succeed");
    (chunk, interner)
}
