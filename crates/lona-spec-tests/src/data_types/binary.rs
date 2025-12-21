// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Binary type.
//!
//! Section 3.8 of the Lonala specification.
//!
//! Note: The Binary VALUE TYPE is implemented (Task 1.1.5), but the native
//! OPERATIONS (make-binary, binary-get, etc.) are pending (Task 1.8.6).

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 3.8: Binary
// Reference: docs/lonala.md#38-binary
// ============================================================================

/// [IGNORED] Spec 3.8: make-binary allocates zeroed buffer
/// Tracking: Native operation pending (Task 1.8.6)
#[test]
#[ignore]
fn test_3_8_make_binary() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_binary(
        "(make-binary 4)",
        &spec_ref("3.8", "Binary", "allocate 4-byte buffer"),
    );
}

/// [IGNORED] Spec 3.8: binary-get retrieves byte at index
/// Tracking: Native operation pending (Task 1.8.6)
#[test]
#[ignore]
fn test_3_8_binary_get() {
    let mut ctx = SpecTestContext::new();
    // Buffer is zeroed by default
    ctx.assert_int(
        "(binary-get (make-binary 4) 0)",
        0,
        &spec_ref("3.8", "Binary", "get byte at index 0"),
    );
}

/// [IGNORED] Spec 3.8: binary-set modifies byte at index
/// Tracking: Native operation pending (Task 1.8.6)
#[test]
#[ignore]
fn test_3_8_binary_set() {
    let mut ctx = SpecTestContext::new();
    // Set byte and read it back
    let _res = ctx.eval("(def buf (make-binary 4))").unwrap();
    let _res = ctx.eval("(binary-set buf 0 0xFF)").unwrap();
    ctx.assert_int(
        "(binary-get buf 0)",
        255,
        &spec_ref("3.8", "Binary", "set byte returns 255"),
    );
}

/// [IGNORED] Spec 3.8: binary-len returns buffer length
/// Tracking: Native operation pending (Task 1.8.6)
#[test]
#[ignore]
fn test_3_8_binary_len() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(binary-len (make-binary 1024))",
        1024,
        &spec_ref("3.8", "Binary", "buffer length"),
    );
}

/// [IGNORED] Spec 3.8: binary-slice creates zero-copy view
/// Tracking: Native operation pending (Task 1.8.6)
#[test]
#[ignore]
fn test_3_8_binary_slice() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def buf (make-binary 20))").unwrap();
    ctx.assert_int(
        "(binary-len (binary-slice buf 10 20))",
        10,
        &spec_ref("3.8", "Binary", "slice length"),
    );
}

/// [IGNORED] Spec 3.8: Binary is mutable (unlike other Lonala types)
/// Tracking: Native operation pending (Task 1.8.6)
#[test]
#[ignore]
fn test_3_8_binary_mutable() {
    let mut ctx = SpecTestContext::new();
    // Create buffer, modify, check modification persists
    let _res = ctx.eval("(def buf (make-binary 4))").unwrap();
    let _res = ctx.eval("(binary-set buf 1 42)").unwrap();
    ctx.assert_int(
        "(binary-get buf 1)",
        42,
        &spec_ref("3.8", "Binary", "mutations persist"),
    );
}

/// [IGNORED] Spec 3.8: binary? predicate
/// Tracking: Type predicates not fully exposed yet
#[test]
#[ignore]
fn test_3_8_binary_predicate() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_bool(
        "(binary? (make-binary 4))",
        true,
        &spec_ref("3.8", "Binary", "binary? returns true for binary"),
    );
    ctx.assert_bool(
        "(binary? \"hello\")",
        false,
        &spec_ref("3.8", "Binary", "binary? returns false for string"),
    );
}
