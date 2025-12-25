// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7.1.2 - Subtraction (-)
//!
//! Reference: docs/lonala/operators.md#712-subtraction--

use crate::{SpecTestContext, spec_ref};

/// Spec 7.1.2: "With one argument, returns its negation"
#[test]
fn test_7_1_2_subtraction_negation() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(- 5)", -5, &spec_ref("7.1.2", "-", "one argument negates"));
    ctx.assert_float("(- 1.5)", -1.5, &spec_ref("7.1.2", "-", "float negation"));
}

/// Spec 7.1.2: Two arguments - subtraction
#[test]
fn test_7_1_2_subtraction_two_args() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(- 10 3)", 7, &spec_ref("7.1.2", "-", "two arguments"));
}

/// Spec 7.1.2: Variadic - subtracts subsequent from first
#[test]
fn test_7_1_2_subtraction_variadic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(- 10 3 2)",
        5,
        &spec_ref("7.1.2", "-", "variadic subtraction"),
    );
}
