// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Atoms (Planned).
//!
//! Section 9.12 of the Lonala specification.
//!
//! Note: Only the native primitives (atom, deref, reset!, compare-and-set!)
//! are tested here. Higher-level operations (swap!, add-watch, remove-watch,
//! set-validator!) are implemented in Lonala using these primitives.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.12: Atoms - Native Primitives
// Reference: docs/lonala.md#912-atoms
// ============================================================================

/// [IGNORED] Spec 9.12: atom creates atom with initial value
/// Tracking: Atoms not yet implemented
#[test]
#[ignore]
fn test_9_12_atom_create() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def counter (atom 0))").unwrap();
    ctx.assert_int(
        "@counter",
        0,
        &spec_ref("9.12", "atom", "deref returns initial value"),
    );
}

/// [IGNORED] Spec 9.12: deref reads current value
/// Tracking: Atoms not yet implemented
#[test]
#[ignore]
fn test_9_12_deref() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def a (atom 42))").unwrap();
    ctx.assert_int("(deref a)", 42, &spec_ref("9.12", "deref", "reads value"));
    ctx.assert_int("@a", 42, &spec_ref("9.12", "@", "reader macro for deref"));
}

/// [IGNORED] Spec 9.12: reset! sets value directly
/// Tracking: Atoms not yet implemented
#[test]
#[ignore]
fn test_9_12_reset() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def a (atom 0))").unwrap();
    ctx.assert_int(
        "(reset! a 100)",
        100,
        &spec_ref("9.12", "reset!", "returns new value"),
    );
    ctx.assert_int("@a", 100, &spec_ref("9.12", "reset!", "atom is reset"));
}

/// [IGNORED] Spec 9.12: compare-and-set! conditional update
/// Tracking: Atoms not yet implemented
#[test]
#[ignore]
fn test_9_12_compare_and_set() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def a (atom 100))").unwrap();
    ctx.assert_bool(
        "(compare-and-set! a 100 200)",
        true,
        &spec_ref("9.12", "compare-and-set!", "succeeds when value matches"),
    );
    ctx.assert_int(
        "@a",
        200,
        &spec_ref("9.12", "compare-and-set!", "atom updated"),
    );
}

/// [IGNORED] Spec 9.12: compare-and-set! fails when value doesn't match
/// Tracking: Atoms not yet implemented
#[test]
#[ignore]
fn test_9_12_compare_and_set_fail() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def a (atom 100))").unwrap();
    ctx.assert_bool(
        "(compare-and-set! a 50 200)",
        false,
        &spec_ref("9.12", "compare-and-set!", "fails when value doesn't match"),
    );
    ctx.assert_int(
        "@a",
        100,
        &spec_ref("9.12", "compare-and-set!", "atom unchanged"),
    );
}
