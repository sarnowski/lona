// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Metadata Operations (Planned).
//!
//! Section 9.6 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.6: Metadata Operations
// Reference: docs/lonala.md#96-metadata-operations
// ============================================================================

/// [IGNORED] Spec 9.6: meta returns nil when no metadata
/// Tracking: Metadata operations not yet implemented
#[test]
#[ignore]
fn test_9_6_meta_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(meta [1 2 3])",
        &spec_ref("9.6", "meta", "returns nil when no metadata"),
    );
}

/// [IGNORED] Spec 9.6: with-meta attaches metadata
/// Tracking: Metadata operations not yet implemented
#[test]
#[ignore]
fn test_9_6_with_meta() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def v (with-meta [1 2 3] {:source \"test\"}))")
        .unwrap();
    ctx.assert_map(
        "(meta v)",
        &spec_ref("9.6", "with-meta", "attaches metadata map"),
    );
}

/// [IGNORED] Spec 9.6: vary-meta transforms metadata
/// Tracking: Metadata operations not yet implemented
#[test]
#[ignore]
fn test_9_6_vary_meta() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def v (with-meta [1 2 3] {:a 1}))").unwrap();
    let _res = ctx.eval("(def v2 (vary-meta v assoc :b 2))").unwrap();
    ctx.assert_map(
        "(meta v2)",
        &spec_ref("9.6", "vary-meta", "transforms existing metadata"),
    );
}
