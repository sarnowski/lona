// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7.1.3 - Multiplication (*)
//!
//! Reference: docs/lonala/operators.md#713-multiplication-

use crate::{SpecTestContext, spec_ref};

/// Spec 7.1.3: "With no arguments, returns 1"
#[test]
fn test_7_1_3_multiplication_zero_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(*)",
        1,
        &spec_ref("7.1.3", "*", "zero arguments returns 1"),
    );
}

/// Spec 7.1.3: One argument
#[test]
fn test_7_1_3_multiplication_one_arg() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(* 5)",
        5,
        &spec_ref("7.1.3", "*", "one argument returns itself"),
    );
}

/// Spec 7.1.3: Two arguments
#[test]
fn test_7_1_3_multiplication_two_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(* 2 3)", 6, &spec_ref("7.1.3", "*", "two arguments"));
}

/// Spec 7.1.3: Variadic
#[test]
fn test_7_1_3_multiplication_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(* 2 3 4)",
        24,
        &spec_ref("7.1.3", "*", "variadic multiplication"),
    );
}

/// [IGNORED] Spec 7.1.3: Ratio multiplication
/// Tracking: Ratio literals not yet implemented
#[test]
#[ignore]
fn test_7_1_3_multiplication_ratio() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_ratio(
        "(* 1/2 1/3)",
        1,
        6,
        &spec_ref("7.1.3", "*", "ratio multiplication"),
    );
}
