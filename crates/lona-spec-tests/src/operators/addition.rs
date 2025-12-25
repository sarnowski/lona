// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7.1.1 - Addition (+)
//!
//! Reference: docs/lonala/operators.md#711-addition-

use crate::{SpecTestContext, spec_ref};

/// Spec 7.1.1: "With no arguments, returns 0"
#[test]
fn test_7_1_1_addition_zero_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+)",
        0,
        &spec_ref("7.1.1", "+", "zero arguments returns 0"),
    );
}

/// Spec 7.1.1: One argument
#[test]
fn test_7_1_1_addition_one_arg() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+ 5)",
        5,
        &spec_ref("7.1.1", "+", "one argument returns itself"),
    );
}

/// Spec 7.1.1: Two arguments
#[test]
fn test_7_1_1_addition_two_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(+ 1 2)", 3, &spec_ref("7.1.1", "+", "two arguments"));
}

/// Spec 7.1.1: Variadic
#[test]
fn test_7_1_1_addition_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(+ 1 2 3 4)",
        10,
        &spec_ref("7.1.1", "+", "variadic addition"),
    );
}

/// [IGNORED] Spec 7.1.1: Mixed types
/// Tracking: Ratio literals not yet implemented
#[test]
#[ignore]
fn test_7_1_1_addition_mixed_types() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_float(
        "(+ 1 2.0)",
        3.0,
        &spec_ref("7.1.1", "+", "int + float = float"),
    );
    ctx.assert_ratio(
        "(+ 1 1/2)",
        3,
        2,
        &spec_ref("7.1.1", "+", "int + ratio = ratio"),
    );
}
