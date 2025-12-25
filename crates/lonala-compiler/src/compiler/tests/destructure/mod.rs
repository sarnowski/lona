// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for destructuring pattern parsing and bytecode compilation.
//!
//! Organized into submodules by pattern type:
//! - `seq_parse_tests` - sequential pattern parsing tests
//! - `map_parse_tests` - map pattern parsing tests
//! - `seq_compile_tests` - sequential bytecode compilation tests
//! - `map_compile_tests` - map bytecode compilation tests

mod map_compile_tests;
mod map_parse_tests;
mod seq_compile_tests;
mod seq_parse_tests;

use alloc::boxed::Box;
use alloc::vec::Vec;

use lona_core::source;
use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use crate::compiler::{Compiler, MacroRegistry};

/// Test source ID for all destructure tests.
pub(super) fn source_id() -> source::Id {
    source::Id::new(0_u32)
}

/// Helper to create a spanned AST node at position 0..1.
pub(super) fn spanned<T>(node: T) -> Spanned<T> {
    Spanned::new(node, Span::new(0_usize, 1_usize))
}

/// Helper to create a spanned AST node with specific span.
pub(super) fn spanned_at<T>(node: T, start: usize, end: usize) -> Spanned<T> {
    Spanned::new(node, Span::new(start, end))
}

/// Helper to create map elements (key, value) as a flat vec for Ast::Map.
/// Ast::Map stores elements as [key1, val1, key2, val2, ...].
pub(super) fn map_elements(pairs: Vec<(Ast, Ast)>) -> Vec<Spanned<Ast>> {
    let mut result = Vec::new();
    for (key, value) in pairs {
        result.push(spanned(key));
        result.push(spanned(value));
    }
    result
}

/// Creates a test compiler with a pre-allocated source register.
///
/// Returns (compiler, source_register) where source_register is the
/// register containing the "collection" to destructure.
pub(super) fn setup_compiler() -> (Compiler<'static, 'static, 'static>, u8) {
    // We need static lifetime interner and registry for test setup
    // Use Box::leak to create 'static references (memory leak is OK in tests)
    let interner: &'static symbol::Interner = Box::leak(Box::new(symbol::Interner::new()));
    let registry: &'static mut MacroRegistry = Box::leak(Box::new(MacroRegistry::new()));

    let mut compiler = Compiler::new(interner, registry, source_id());

    // Push a scope for local bindings
    compiler.locals.push_scope();

    // Allocate a register to serve as the "source collection"
    // This simulates having compiled the value expression first
    let source_reg = compiler
        .alloc_register(Span::new(0_usize, 1_usize))
        .expect("register allocation");

    (compiler, source_reg)
}
