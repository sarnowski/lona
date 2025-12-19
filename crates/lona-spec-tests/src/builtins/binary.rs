// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Binary Operations (Planned).
//!
//! Section 9.4 of the Lonala specification.

use crate::{SpecTestContext, spec_ref};

// ============================================================================
// Section 9.4: Binary Operations
// Reference: docs/lonala.md#94-binary-operations
// ============================================================================

/// [IGNORED] Spec 9.4: make-binary allocates zeroed buffer
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_make_binary() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_binary(
        "(make-binary 4)",
        &spec_ref("9.4", "make-binary", "allocate zeroed buffer"),
    );
}

/// [IGNORED] Spec 9.4: binary-len returns buffer length
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_binary_len() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(binary-len (make-binary 4))",
        4,
        &spec_ref("9.4", "binary-len", "get buffer length"),
    );
}

/// [IGNORED] Spec 9.4: binary-get/binary-set operations
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_binary_get_set() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def buf (make-binary 4))").unwrap();
    let _res = ctx.eval("(binary-set buf 0 0xFF)").unwrap();
    ctx.assert_int(
        "(binary-get buf 0)",
        255,
        &spec_ref("9.4", "binary-get/set", "get byte at index"),
    );
}

/// [IGNORED] Spec 9.4: binary-slice zero-copy view
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_binary_slice() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def buf (make-binary 20))").unwrap();
    ctx.assert_int(
        "(binary-len (binary-slice buf 10 20))",
        10,
        &spec_ref("9.4", "binary-slice", "zero-copy slice"),
    );
}

/// [IGNORED] Spec 9.4: binary-copy! copies bytes
/// Tracking: Binary operations not yet implemented
#[test]
#[ignore]
fn test_9_4_binary_copy() {
    let mut ctx = SpecTestContext::new();
    let _res = ctx.eval("(def src (make-binary 4))").unwrap();
    let _res = ctx.eval("(def dst (make-binary 4))").unwrap();
    let _res = ctx.eval("(binary-set src 0 42)").unwrap();
    let _res = ctx.eval("(binary-copy! dst 0 src 0 1)").unwrap();
    ctx.assert_int(
        "(binary-get dst 0)",
        42,
        &spec_ref("9.4", "binary-copy!", "copy bytes between buffers"),
    );
}
