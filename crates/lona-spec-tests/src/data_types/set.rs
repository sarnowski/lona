// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Set type (Planned).
//!
//! Section 3.12 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.12: Set (Planned)
// Reference: docs/lonala.md#312-set
// ============================================================================

/// [IGNORED] Spec 3.12: Set literal syntax
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_3_12_set_literal() {
    let mut ctx = SpecTestContext::new();
    // Set literals use #{} syntax
    ctx.assert_bool(
        "(set? #{1 2 3})",
        true,
        &spec_ref("3.12", "Set", "#{} creates a set"),
    );
}

/// [IGNORED] Spec 3.12: Sets automatically remove duplicates
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_3_12_set_duplicate_removed() {
    let mut ctx = SpecTestContext::new();
    // #{1 2 2 3} => #{1 2 3}
    ctx.assert_bool(
        "(= #{1 2 2 3} #{1 2 3})",
        true,
        &spec_ref("3.12", "Set", "duplicates are automatically removed"),
    );
}

/// [IGNORED] Spec 3.12: Set equality (unordered)
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_3_12_set_equality() {
    let mut ctx = SpecTestContext::new();
    // Sets with same elements are equal regardless of order
    ctx.assert_bool(
        "(= #{1 2 3} #{3 2 1})",
        true,
        &spec_ref("3.12", "Set", "sets with same elements are equal"),
    );
    ctx.assert_bool(
        "(= #{1 2} #{1 2 3})",
        false,
        &spec_ref("3.12", "Set", "sets with different elements are not equal"),
    );
}

/// [IGNORED] Spec 3.12: Empty set
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_3_12_empty_set() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(set? #{})",
        true,
        &spec_ref("3.12", "Set", "#{} is an empty set"),
    );
    ctx.assert_bool(
        "(= #{} #{})",
        true,
        &spec_ref("3.12", "Set", "empty sets are equal"),
    );
}

/// [IGNORED] Spec 3.12: Set membership test
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_3_12_set_contains() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(contains? #{1 2 3} 2)",
        true,
        &spec_ref("3.12", "Set", "contains? returns true for members"),
    );
    ctx.assert_bool(
        "(contains? #{1 2 3} 5)",
        false,
        &spec_ref("3.12", "Set", "contains? returns false for non-members"),
    );
}

/// [IGNORED] Spec 3.12: Sets can contain mixed types
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_3_12_set_mixed_types() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(contains? #{1 \"two\" :three} \"two\")",
        true,
        &spec_ref("3.12", "Set", "sets can contain mixed types"),
    );
}

/// [IGNORED] Spec 3.12: conj adds element to set
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_3_12_set_conj() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (conj #{1 2} 3) #{1 2 3})",
        true,
        &spec_ref("3.12", "Set", "conj adds element to set"),
    );
    ctx.assert_bool(
        "(= (conj #{1 2} 2) #{1 2})",
        true,
        &spec_ref("3.12", "Set", "conj with existing element is no-op"),
    );
}

/// [IGNORED] Spec 3.12: disj removes element from set
/// Tracking: Set type not yet implemented
#[test]
#[ignore]
fn test_3_12_set_disj() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (disj #{1 2 3} 2) #{1 3})",
        true,
        &spec_ref("3.12", "Set", "disj removes element from set"),
    );
    ctx.assert_bool(
        "(= (disj #{1 2} 5) #{1 2})",
        true,
        &spec_ref("3.12", "Set", "disj with missing element is no-op"),
    );
}

/// [IGNORED] Spec 3.12: set? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_3_12_set_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(set? #{1 2 3})",
        true,
        &spec_ref("3.12", "Set", "set? returns true for set"),
    );
    ctx.assert_bool(
        "(set? [1 2 3])",
        false,
        &spec_ref("3.12", "Set", "set? returns false for vector"),
    );
    ctx.assert_bool(
        "(set? '(1 2 3))",
        false,
        &spec_ref("3.12", "Set", "set? returns false for list"),
    );
}
