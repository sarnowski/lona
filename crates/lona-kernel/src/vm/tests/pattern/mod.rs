// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the pattern matching engine.
//!
//! Split into modules by pattern type for maintainability.

mod bind_tests;
mod depth_tests;
mod guarded_tests;
mod literal_tests;
mod map_tests;
mod nested_tests;
mod proptest_tests;
mod seq_tests;
mod wildcard_tests;

use lona_core::{symbol, symbol::Interner};

/// Helper to create a test symbol ID.
pub(super) fn make_symbol(interner: &Interner, name: &str) -> symbol::Id {
    interner.intern(name)
}
