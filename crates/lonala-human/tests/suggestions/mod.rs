// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the "did you mean" suggestion system.
//!
//! These tests verify that the error formatting system correctly handles
//! suggestions for undefined symbols, producing helpful "did you mean X?"
//! messages when a similar symbol exists.
//!
//! # Test Scenarios
//!
//! - Typos in symbol names (fooo → foo)
//! - Case differences (Foo → foo)
//! - Missing/extra characters
//! - No similar symbol exists
//! - Suggestions for both globals and functions
//!
//! # Note on Suggestion Generation
//!
//! The suggestion *generation* (finding similar symbols using Levenshtein
//! distance or similar algorithms) is done by the VM, not by `lonala-human`.
//! These tests verify that suggestions are *formatted* correctly when present.

#![cfg(feature = "alloc")]

extern crate alloc;

mod case;
mod special;
mod typos;

use alloc::string::ToString;

use lona_core::source::{Id as SourceId, Location, Registry};
use lona_core::span::Span;
use lona_core::symbol::Interner;
use lona_kernel::vm::{Error as VmError, ErrorKind as VmKind};
use lonala_human::{Config, render};

// =============================================================================
// Helper Functions
// =============================================================================

/// Creates a test source registry with a single source.
pub(super) fn create_registry(name: &str, content: &str) -> (Registry, SourceId) {
    let mut registry = Registry::new();
    let source_id = registry
        .add(name.to_string(), content.to_string())
        .expect("should add source");
    (registry, source_id)
}

/// Creates a location for a span in the given source.
pub(super) fn loc(source_id: SourceId, start: usize, end: usize) -> Location {
    Location::new(source_id, Span::new(start, end))
}
