// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7.3 - Bitwise Operators
//!
//! Reference: docs/lonala/operators.md#73-bitwise-operators

use crate::{SpecTestContext, spec_ref};

/// [IGNORED] Spec 7.3.1: Bitwise AND
/// Tracking: Bitwise operators not yet implemented
#[test]
#[ignore]
fn test_7_3_1_bit_and() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(bit-and 0xFF 0x0F)",
        15,
        &spec_ref("7.3.1", "bit-and", "0xFF AND 0x0F = 0x0F"),
    );
    ctx.assert_int(
        "(bit-and 0b1100 0b1010)",
        8,
        &spec_ref("7.3.1", "bit-and", "0b1100 AND 0b1010 = 0b1000"),
    );
}

/// [IGNORED] Spec 7.3.2: Bitwise OR
/// Tracking: Bitwise operators not yet implemented
#[test]
#[ignore]
fn test_7_3_2_bit_or() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(bit-or 0b1100 0b0011)",
        15,
        &spec_ref("7.3.2", "bit-or", "0b1100 OR 0b0011 = 0b1111"),
    );
}

/// [IGNORED] Spec 7.3.3: Bitwise XOR
/// Tracking: Bitwise operators not yet implemented
#[test]
#[ignore]
fn test_7_3_3_bit_xor() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(bit-xor 0b1100 0b1010)",
        6,
        &spec_ref("7.3.3", "bit-xor", "0b1100 XOR 0b1010 = 0b0110"),
    );
}

/// [IGNORED] Spec 7.3.4: Bitwise NOT
/// Tracking: Bitwise operators not yet implemented
#[test]
#[ignore]
fn test_7_3_4_bit_not() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(bit-not 0)",
        -1,
        &spec_ref("7.3.4", "bit-not", "NOT 0 = -1"),
    );
}

/// [IGNORED] Spec 7.3.5: Shift Left
/// Tracking: Bitwise operators not yet implemented
#[test]
#[ignore]
fn test_7_3_5_bit_shift_left() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(bit-shift-left 1 4)",
        16,
        &spec_ref("7.3.5", "bit-shift-left", "1 << 4 = 16"),
    );
    ctx.assert_int(
        "(bit-shift-left 0xFF 8)",
        0xFF00,
        &spec_ref("7.3.5", "bit-shift-left", "0xFF << 8 = 0xFF00"),
    );
}

/// [IGNORED] Spec 7.3.6: Shift Right
/// Tracking: Bitwise operators not yet implemented
#[test]
#[ignore]
fn test_7_3_6_bit_shift_right() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(bit-shift-right 16 2)",
        4,
        &spec_ref("7.3.6", "bit-shift-right", "16 >> 2 = 4"),
    );
    ctx.assert_int(
        "(bit-shift-right 0xFF00 8)",
        0xFF,
        &spec_ref("7.3.6", "bit-shift-right", "0xFF00 >> 8 = 0xFF"),
    );
}
