// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Error Tuples.
//!
//! Section 3.17 of the Lonala specification.
//!
//! Note: Error tuples are a convention using maps with :ok/:error keys.
//! The data structure tests here verify this pattern works with maps.
//! The accessor functions (ok?, error?, unwrap!, unwrap-or, unwrap-error)
//! are implemented in Lonala, not as native primitives.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.17: Error Tuples - Data Structure Convention
// Reference: docs/lonala.md#317-error-tuples
// ============================================================================

/// Spec 3.17: {:ok value} tuple for success
#[test]
fn test_3_17_ok_tuple() {
    let mut ctx = SpecTestContext::new();
    // Success tuple pattern - verify it's a map with extractable value
    ctx.assert_map(
        "{:ok 42}",
        &spec_ref("3.17", "Error Tuples", "{:ok value} is a map"),
    );
    ctx.assert_int(
        "(get {:ok 42} :ok)",
        42,
        &spec_ref("3.17", "Error Tuples", "can extract value from {:ok value}"),
    );
}

/// Spec 3.17: {:error reason} tuple for failure with keyword reason
#[test]
fn test_3_17_error_tuple_keyword() {
    let mut ctx = SpecTestContext::new();
    // Error tuple pattern with keyword reason
    ctx.assert_bool(
        "(= (get {:error :not-found} :error) :not-found)",
        true,
        &spec_ref(
            "3.17",
            "Error Tuples",
            "can extract reason from {:error :keyword}",
        ),
    );
}

/// Spec 3.17: {:error reason} with string reason
#[test]
fn test_3_17_error_tuple_string() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_string(
        "(get {:error \"file not found\"} :error)",
        "file not found",
        &spec_ref("3.17", "Error Tuples", "error reason can be a string"),
    );
}

/// Spec 3.17: Error tuple with map reason for rich context
#[test]
fn test_3_17_error_with_map_reason() {
    let mut ctx = SpecTestContext::new();
    // Rich error context - nested map
    ctx.assert_map(
        "(get {:error {:type :validation :field :email}} :error)",
        &spec_ref("3.17", "Error Tuples", "error reason can be a map"),
    );
}
