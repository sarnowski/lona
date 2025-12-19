// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for I/O Operations (Planned).
//!
//! Section 9.13 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.13: I/O (print)
// Reference: docs/lonala.md#913-io
// ============================================================================

/// [IGNORED] Spec 9.13: print returns nil
/// Tracking: print function behavior
#[test]
#[ignore]
fn test_9_13_print_returns_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(print \"hello\")",
        &spec_ref("9.13", "print", "returns nil"),
    );
}

/// [IGNORED] Spec 9.13: print with multiple arguments
/// Tracking: print function behavior
#[test]
#[ignore]
fn test_9_13_print_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(print 1 2 3)",
        &spec_ref("9.13", "print", "variadic arguments"),
    );
}
