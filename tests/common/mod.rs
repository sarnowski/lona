// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Shared test infrastructure for integration tests.
//!
//! This module provides:
//! - [`TestVm`] - A stateful VM for testing language features
//! - [`ValueMatcher`] - Trait and implementations for structural assertions
//!
//! # Design
//!
//! This module is **not** a test file, so it must comply with full clippy rules.
//! Test-specific allowances (like `unwrap_used`) are only permitted in `*_test.rs` files.
//! Macros defined here are expanded at call sites, so they can use unwrap in test files.

#![expect(unused_imports, reason = "re-exports used by test files")]

pub mod matchers;
pub mod test_vm;

pub use matchers::{
    IsBool, IsInt, IsList, IsNil, IsString, IsSymbol, PrintsAs, ValueMatcher, assert_value_matches,
};
pub use test_vm::{TestVm, TestVmError};
