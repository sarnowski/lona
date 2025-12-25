// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7.5 - Numeric Type Coercion
//!
//! Reference: docs/lonala/operators.md#75-numeric-type-coercion

use crate::{SpecTestContext, spec_ref};

/// Spec 7.5: Integer + Integer = Integer
#[test]
fn test_7_5_coercion_int_int() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+ 1 2)",
        3,
        &spec_ref("7.5", "Coercion", "int + int = int"),
    );
}

/// Spec 7.5: Integer + Float = Float
#[test]
fn test_7_5_coercion_int_float() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_float(
        "(+ 1 2.0)",
        3.0,
        &spec_ref("7.5", "Coercion", "int + float = float"),
    );
}

/// [IGNORED] Spec 7.5: Integer + Ratio = Ratio
/// Tracking: Ratio literals not yet implemented
#[test]
#[ignore]
fn test_7_5_coercion_int_ratio() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(+ 1 1/2)",
        3,
        2,
        &spec_ref("7.5", "Coercion", "int + ratio = ratio"),
    );
}

/// Spec 7.5: Integer / Integer (exact) = Integer
#[test]
fn test_7_5_coercion_division_exact() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(/ 6 2)",
        3,
        &spec_ref("7.5", "Coercion", "int / int exact = int"),
    );
}

/// Spec 7.5: Integer / Integer (inexact) = Ratio
#[test]
fn test_7_5_coercion_division_inexact() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(/ 5 2)",
        5,
        2,
        &spec_ref("7.5", "Coercion", "int / int inexact = ratio"),
    );
}
