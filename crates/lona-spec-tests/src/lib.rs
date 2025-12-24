// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Lonala Language Specification Tests
//!
//! This crate tests the Lonala language against its specification
//! at `docs/lonala.md`. Each test file corresponds to a spec section.
//!
//! ## Test Structure
//!
//! Test files are organized by specification section:
//! - `data_types` - Section 3: Data Types
//! - `literals` - Section 4: Literals
//! - `evaluation` - Section 5: Symbols and Evaluation
//! - `special_forms` - Section 6: Special Forms
//! - `operators` - Section 7: Operators
//! - `functions` - Section 8: Functions
//! - `builtins` - Section 9: Built-in Functions
//! - `reader_macros` - Section 10: Reader Macros
//! - `macros` - Section 11: Macros
//!
//! ## Test Naming Convention
//!
//! Tests follow the pattern: `test_<section>_<subsection>_<description>`
//!
//! Examples:
//! - `test_3_2_nil_is_falsy`
//! - `test_6_3_if_no_else_returns_nil`
//! - `test_7_1_1_addition_mixed_types`
//!
//! ## Assertion Messages
//!
//! All assertions include spec references in the format:
//! `[Spec X.Y Topic] description`
//!
//! Example: `[Spec 3.2 Nil] nil should be falsy in conditionals`

#![no_std]

#[cfg(all(test, feature = "alloc"))]
extern crate alloc;

// Test infrastructure and modules - only compiled for tests
#[cfg(test)]
mod builtins;
#[cfg(test)]
mod context;
#[cfg(test)]
mod data_types;
#[cfg(test)]
mod evaluation;
#[cfg(test)]
mod functions;
#[cfg(test)]
mod literals;
#[cfg(test)]
mod macros;
#[cfg(test)]
mod operators;
#[cfg(test)]
mod reader_macros;
#[cfg(test)]
mod special_forms;
#[cfg(test)]
mod tco;

// Re-export test infrastructure for use in test modules
#[cfg(test)]
pub use context::SpecTestContext;

/// Creates a spec reference string for assertion messages.
///
/// Format: `[Spec <section> <topic>] <description>`
///
/// # Examples
///
/// ```ignore
/// spec_ref("3.2", "Nil", "nil is falsy") -> "[Spec 3.2 Nil] nil is falsy"
/// spec_ref("6.3", "if", "true branch") -> "[Spec 6.3 if] true branch"
/// ```
#[cfg(all(test, feature = "alloc"))]
pub fn spec_ref(section: &str, topic: &str, description: &str) -> alloc::string::String {
    alloc::format!("[Spec {section} {topic}] {description}")
}
