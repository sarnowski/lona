// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7.1.4 - Division (/)
//!
//! Reference: docs/lonala/operators.md#714-division-

use crate::{SpecTestContext, spec_ref};

/// Spec 7.1.4: "With one argument, returns its reciprocal"
#[test]
fn test_7_1_4_division_reciprocal() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(/ 2)",
        1,
        2,
        &spec_ref("7.1.4", "/", "one argument returns reciprocal"),
    );
}

/// Spec 7.1.4: Two arguments - division
#[test]
fn test_7_1_4_division_two_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(/ 10 2)",
        5,
        &spec_ref("7.1.4", "/", "exact division yields integer"),
    );
}

/// Spec 7.1.4: Variadic
#[test]
fn test_7_1_4_division_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(/ 10 2 5)",
        1,
        &spec_ref("7.1.4", "/", "variadic division"),
    );
}

/// Spec 7.1.4: "Division of integers that doesn't produce a whole number yields a Ratio"
#[test]
fn test_7_1_4_division_yields_ratio() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(/ 1 3)",
        1,
        3,
        &spec_ref("7.1.4", "/", "inexact division yields ratio"),
    );
}

/// Spec 7.1.4: Float division
#[test]
fn test_7_1_4_division_float() {
    let mut ctx = SpecTestContext::new();
    // Using approximate comparison for floating point
    let result = ctx.eval("(/ 1.0 3)").unwrap();
    match result {
        lona_core::value::Value::Float(float_val) => {
            let expected = 1.0 / 3.0;
            assert!(
                (float_val - expected).abs() < 1e-10,
                "[Spec 7.1.4 /] float division: expected {}, got {}",
                expected,
                float_val
            );
        }
        _ => panic!("[Spec 7.1.4 /] expected float"),
    }
}
