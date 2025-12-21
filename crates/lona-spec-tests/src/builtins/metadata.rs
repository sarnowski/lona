// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Metadata Operations.
//!
//! Section 9.6 of the Lonala specification.
//!
//! Metadata is a map of data about a value that does not affect its equality
//! or hash code. This follows Clojure's metadata semantics.
//!
//! Types supporting metadata: Symbol, List, Vector, Map, Set
//! Types NOT supporting metadata: Keyword, nil, bool, numbers, strings, binaries, functions

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.6: Metadata Operations
// Reference: docs/lonala.md#96-metadata-operations
// ============================================================================

// ============================================================================
// meta: returns nil when no metadata
// ============================================================================

/// Spec 9.6: meta returns nil for vector without metadata
#[test]
fn test_9_6_meta_vector_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(meta [])",
        &spec_ref("9.6", "meta", "returns nil for empty vector"),
    );
}

/// Spec 9.6: meta returns nil for list without metadata
#[test]
fn test_9_6_meta_list_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(meta '(1 2 3))",
        &spec_ref("9.6", "meta", "returns nil for list"),
    );
}

/// Spec 9.6: meta returns nil for map without metadata
#[test]
fn test_9_6_meta_map_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(meta {})",
        &spec_ref("9.6", "meta", "returns nil for empty map"),
    );
}

/// Spec 9.6: meta returns nil for set without metadata
#[test]
fn test_9_6_meta_set_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(meta #{})",
        &spec_ref("9.6", "meta", "returns nil for empty set"),
    );
}

/// Spec 9.6: meta returns nil for types that don't support metadata
#[test]
fn test_9_6_meta_unsupported_types() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_nil(
        "(meta 42)",
        &spec_ref("9.6", "meta", "returns nil for integer"),
    );
    ctx.assert_nil(
        "(meta \"hello\")",
        &spec_ref("9.6", "meta", "returns nil for string"),
    );
    ctx.assert_nil(
        "(meta :keyword)",
        &spec_ref("9.6", "meta", "returns nil for keyword"),
    );
}

// ============================================================================
// with-meta: attaches metadata
// ============================================================================

/// Spec 9.6: with-meta attaches metadata to vector
#[test]
fn test_9_6_with_meta_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(meta (with-meta [] {:foo 1}))",
        "{:foo 1}",
        &spec_ref("9.6", "with-meta", "attaches metadata to vector"),
    );
}

/// Spec 9.6: with-meta attaches metadata to list
#[test]
fn test_9_6_with_meta_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(meta (with-meta '(1 2) {:type :list}))",
        "{:type :list}",
        &spec_ref("9.6", "with-meta", "attaches metadata to list"),
    );
}

/// Spec 9.6: with-meta attaches metadata to map
#[test]
fn test_9_6_with_meta_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(meta (with-meta {} {:type :map}))",
        "{:type :map}",
        &spec_ref("9.6", "with-meta", "attaches metadata to map"),
    );
}

/// Spec 9.6: with-meta attaches metadata to set
#[test]
fn test_9_6_with_meta_set() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(meta (with-meta #{} {:type :set}))",
        "{:type :set}",
        &spec_ref("9.6", "with-meta", "attaches metadata to set"),
    );
}

/// Spec 9.6: with-meta with nil clears metadata
#[test]
fn test_9_6_with_meta_nil_clears() {
    let mut ctx = SpecTestContext::new();
    ctx.eval("(def v (with-meta [1 2 3] {:foo 1}))").unwrap();
    ctx.assert_nil(
        "(meta (with-meta v nil))",
        &spec_ref("9.6", "with-meta", "nil clears metadata"),
    );
}

// ============================================================================
// Metadata does not affect equality
// ============================================================================

/// Spec 9.6: metadata does not affect vector equality
#[test]
fn test_9_6_equality_ignores_metadata_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= [] (with-meta [] {:foo 1}))",
        true,
        &spec_ref("9.6", "equality", "ignores metadata on vectors"),
    );
}

/// Spec 9.6: metadata does not affect list equality
#[test]
fn test_9_6_equality_ignores_metadata_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= '(1 2) (with-meta '(1 2) {:foo 1}))",
        true,
        &spec_ref("9.6", "equality", "ignores metadata on lists"),
    );
}

/// Spec 9.6: metadata does not affect map equality
#[test]
fn test_9_6_equality_ignores_metadata_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= {:a 1} (with-meta {:a 1} {:foo 1}))",
        true,
        &spec_ref("9.6", "equality", "ignores metadata on maps"),
    );
}

/// Spec 9.6: metadata does not affect set equality
#[test]
fn test_9_6_equality_ignores_metadata_set() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(= #{1 2} (with-meta #{1 2} {:foo 1}))",
        true,
        &spec_ref("9.6", "equality", "ignores metadata on sets"),
    );
}

// ============================================================================
// Metadata preserved through collection operations
// ============================================================================

/// Spec 9.6: metadata preserved through conj on vector
#[test]
fn test_9_6_metadata_preserved_conj_vector() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(meta (conj (with-meta [] {:foo 1}) 42))",
        "{:foo 1}",
        &spec_ref("9.6", "preservation", "conj preserves metadata on vector"),
    );
}

/// Spec 9.6: metadata preserved through conj on list
#[test]
fn test_9_6_metadata_preserved_conj_list() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(meta (conj (with-meta '() {:foo 1}) 42))",
        "{:foo 1}",
        &spec_ref("9.6", "preservation", "conj preserves metadata on list"),
    );
}

/// Spec 9.6: metadata preserved through conj on set
#[test]
fn test_9_6_metadata_preserved_conj_set() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(meta (conj (with-meta #{} {:foo 1}) 42))",
        "{:foo 1}",
        &spec_ref("9.6", "preservation", "conj preserves metadata on set"),
    );
}

/// [IGNORED] Spec 9.6: metadata preserved through assoc on map
/// Tracking: assoc is not yet implemented
#[test]
#[ignore]
fn test_9_6_metadata_preserved_assoc_map() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_map_eq(
        "(meta (assoc (with-meta {} {:foo 1}) :a 1))",
        "{:foo 1}",
        &spec_ref("9.6", "preservation", "assoc preserves metadata on map"),
    );
}

// ============================================================================
// Error cases
// ============================================================================

/// Spec 9.6: with-meta requires map or nil as second argument
#[test]
fn test_9_6_with_meta_requires_map_or_nil() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(with-meta [] 42)",
        &spec_ref("9.6", "error", "rejects non-map metadata"),
    );
}

/// Spec 9.6: with-meta rejects types that don't support metadata
#[test]
fn test_9_6_with_meta_rejects_unsupported() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "(with-meta 42 {:foo 1})",
        &spec_ref("9.6", "error", "rejects integers (no metadata support)"),
    );
}

// ============================================================================
// vary-meta: DEFERRED until apply is implemented (Task 1.8.20)
// ============================================================================

/// [IGNORED] Spec 9.6: vary-meta transforms metadata
/// Tracking: vary-meta requires apply (Task 1.8.20)
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
