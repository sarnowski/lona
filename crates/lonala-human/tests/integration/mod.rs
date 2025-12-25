// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Integration tests for end-to-end error formatting.
//!
//! These tests verify that the `lonala-human` crate correctly formats errors
//! from all error sources (parser, compiler, VM) into Rust-style diagnostic
//! messages with source context, underlines, and helpful notes.
//!
//! # Test Modules
//!
//! - `parse_errors` - Parser error tests
//! - `compile_errors` - Compiler error tests
//! - `vm_errors` - VM error tests
//! - `format_tests` - Output format verification tests

#![cfg(feature = "alloc")]

extern crate alloc;

use alloc::string::{String, ToString};

use lona_core::source::{Id as SourceId, Location, Registry};
use lona_core::span::Span;

mod compile_errors;
mod format_tests;
mod parse_errors;
mod vm_errors;

// =============================================================================
// Helper Functions
// =============================================================================

/// Creates a test source registry with a single source.
pub fn create_registry(name: &str, content: &str) -> (Registry, SourceId) {
    let mut registry = Registry::new();
    let source_id = registry
        .add(name.to_string(), content.to_string())
        .expect("should add source");
    (registry, source_id)
}

/// Creates a location for a span in the given source.
pub fn loc(source_id: SourceId, start: usize, end: usize) -> Location {
    Location::new(source_id, Span::new(start, end))
}
