// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Metadata (Planned).
//!
//! Section 3.16 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.16: Metadata (Planned)
// Reference: docs/lonala.md#316-metadata
// ============================================================================

/// [IGNORED] Spec 3.16: meta returns nil for values without metadata
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_16_meta_returns_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(meta [1 2 3])",
        &spec_ref("3.16", "Metadata", "meta returns nil when no metadata"),
    );
}

/// [IGNORED] Spec 3.16: with-meta attaches metadata
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_16_with_meta() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def v (with-meta [1 2 3] {:source \"test\"}))")
        .unwrap();
    ctx.assert_map(
        "(meta v)",
        &spec_ref("3.16", "Metadata", "with-meta attaches map"),
    );
}

/// [IGNORED] Spec 3.16: Metadata does NOT affect equality
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_16_metadata_equality() {
    let mut ctx = SpecTestContext::new();
    // Two values that differ only in metadata should be equal
    ctx.assert_bool(
        "(= [1 2 3] (with-meta [1 2 3] {:foo :bar}))",
        true,
        &spec_ref("3.16", "Metadata", "metadata does not affect equality"),
    );
}

/// [IGNORED] Spec 3.16: vary-meta transforms metadata
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_16_vary_meta() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def v (with-meta [1 2 3] {:a 1}))").unwrap();
    let _res = ctx.eval("(def v2 (vary-meta v assoc :b 2))").unwrap();
    // v2's metadata should have both :a and :b
    ctx.assert_map(
        "(meta v2)",
        &spec_ref("3.16", "Metadata", "vary-meta transforms metadata"),
    );
}

/// [IGNORED] Spec 3.16: Primitives do not support metadata
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_16_primitives_no_metadata() {
    let mut ctx = SpecTestContext::new();
    // Numbers, strings, nil, booleans cannot have metadata
    ctx.assert_error(
        "(with-meta 42 {:foo :bar})",
        &spec_ref("3.16", "Metadata", "integers cannot have metadata"),
    );
}

/// [IGNORED] Spec 3.16: Symbols can have metadata
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_16_symbol_metadata() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx
        .eval("(def s (with-meta 'foo {:doc \"a symbol\"}))")
        .unwrap();
    ctx.assert_map(
        "(meta s)",
        &spec_ref("3.16", "Metadata", "symbols can have metadata"),
    );
}

/// [IGNORED] Spec 3.16: Metadata hash does not affect value hash
/// Tracking: Metadata not yet implemented
#[test]
#[ignore]
fn test_3_16_metadata_hash() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= (hash [1 2 3]) (hash (with-meta [1 2 3] {:foo :bar})))",
        true,
        &spec_ref("3.16", "Metadata", "metadata does not affect hash"),
    );
}

/// [IGNORED] Spec 3.16: Reader metadata syntax ^{:key val}
/// Tracking: Metadata on collections (^{...} before vector/list/map) not yet
/// implemented. This requires the compiler to emit `with-meta` calls.
/// Task 1.1.7 deferred this as "can be a follow-up task".
#[test]
#[ignore]
fn test_3_16_reader_metadata_full() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map(
        "(meta ^{:doc \"A vector\"} [1 2 3])",
        &spec_ref("3.16", "Metadata", "reader syntax attaches metadata"),
    );
}

/// [IGNORED] Spec 3.16: Reader metadata shorthand ^:keyword
/// Tracking: Metadata on symbols (^:key before 'symbol) not yet implemented.
/// This requires the compiler to emit `with-meta` calls for quoted symbols.
/// Task 1.1.7 deferred this as "can be a follow-up task".
#[test]
#[ignore]
fn test_3_16_reader_metadata_shorthand() {
    let mut ctx = SpecTestContext::new();
    // ^:private expands to ^{:private true}
    ctx.assert_bool(
        "(get (meta ^:private 'my-var) :private)",
        true,
        &spec_ref("3.16", "Metadata", "^:keyword shorthand sets true"),
    );
}
