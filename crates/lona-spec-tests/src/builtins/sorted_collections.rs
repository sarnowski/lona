// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Sorted Collections (Planned).
//!
//! Section 9.7 of the Lonala specification.
//!
//! Note: Only the native constructors (sorted-map, sorted-set, sorted-map-by,
//! sorted-set-by) are tested here. The subseq and rsubseq operations are
//! implemented in Lonala using iteration primitives.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.7: Sorted Collections - Native Primitives
// Reference: docs/lonala.md#97-sorted-collections
// ============================================================================

/// [IGNORED] Spec 9.7: sorted-map creates sorted map
/// Tracking: Sorted collections not yet implemented
#[test]
#[ignore]
fn test_9_7_sorted_map() {
    let mut _ctx = SpecTestContext::new();
    // sorted-map maintains keys in natural order
    // (sorted-map :c 3 :a 1 :b 2) => {:a 1 :b 2 :c 3}
}

/// [IGNORED] Spec 9.7: sorted-set creates sorted set
/// Tracking: Sorted collections not yet implemented
#[test]
#[ignore]
fn test_9_7_sorted_set() {
    let mut _ctx = SpecTestContext::new();
    // sorted-set maintains elements in natural order
    // (sorted-set 3 1 4 1 5 9) => #{1 3 4 5 9}
}

/// [IGNORED] Spec 9.7: sorted-map-by with custom comparator
/// Tracking: Sorted collections not yet implemented
#[test]
#[ignore]
fn test_9_7_sorted_map_by() {
    let mut ctx = SpecTestContext::new();
    // Descending order
    ctx.assert_map(
        "(sorted-map-by > :c 3 :a 1 :b 2)",
        &spec_ref("9.7", "sorted-map-by", "custom comparator"),
    );
}

/// [IGNORED] Spec 9.7: sorted-set-by with custom comparator
/// Tracking: Sorted collections not yet implemented
#[test]
#[ignore]
fn test_9_7_sorted_set_by() {
    let mut _ctx = SpecTestContext::new();
    // (sorted-set-by > 3 1 4) => #{4 3 1}
}
