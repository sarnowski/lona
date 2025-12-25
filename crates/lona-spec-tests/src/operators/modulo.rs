// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7.1.5 - Modulo (mod)
//!
//! Reference: docs/lonala/operators.md#715-modulo-mod

use crate::{SpecTestContext, spec_ref};

/// Spec 7.1.5: Basic modulo
#[test]
fn test_7_1_5_mod_basic() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int("(mod 10 3)", 1, &spec_ref("7.1.5", "mod", "basic modulo"));
    ctx.assert_int("(mod 10 5)", 0, &spec_ref("7.1.5", "mod", "exact divisor"));
}

/// Spec 7.1.5: Negative modulo
#[test]
fn test_7_1_5_mod_negative() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(mod -10 3)",
        -1,
        &spec_ref("7.1.5", "mod", "negative dividend"),
    );
}
