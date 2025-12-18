// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for opcode definitions and instruction encoding/decoding.

use super::*;

// =============================================================================
// Opcode Tests
// =============================================================================

#[test]
fn opcode_from_u8_valid() {
    assert_eq!(Opcode::from_u8(0), Some(Opcode::Move));
    assert_eq!(Opcode::from_u8(1), Some(Opcode::LoadK));
    assert_eq!(Opcode::from_u8(24), Some(Opcode::Return));
}

#[test]
fn opcode_from_u8_invalid() {
    assert_eq!(Opcode::from_u8(25), None);
    assert_eq!(Opcode::from_u8(100), None);
    assert_eq!(Opcode::from_u8(255), None);
}

#[test]
fn opcode_name() {
    assert_eq!(Opcode::Move.name(), "Move");
    assert_eq!(Opcode::LoadK.name(), "LoadK");
    assert_eq!(Opcode::Add.name(), "Add");
    assert_eq!(Opcode::Return.name(), "Return");
}

#[test]
fn opcode_repr() {
    assert_eq!(Opcode::Move as u8, 0);
    assert_eq!(Opcode::LoadK as u8, 1);
    assert_eq!(Opcode::Return as u8, 24);
}

// =============================================================================
// iABC Encoding/Decoding Tests
// =============================================================================

#[test]
fn encode_abc_basic() {
    let instr = encode_abc(Opcode::Add, 0, 1, 2);
    assert_eq!(decode_op(instr), Some(Opcode::Add));
    assert_eq!(decode_a(instr), 0);
    assert_eq!(decode_b(instr), 1);
    assert_eq!(decode_c(instr), 2);
}

#[test]
fn encode_abc_max_values() {
    let instr = encode_abc(Opcode::Move, 255, 255, 255);
    assert_eq!(decode_op(instr), Some(Opcode::Move));
    assert_eq!(decode_a(instr), 255);
    assert_eq!(decode_b(instr), 255);
    assert_eq!(decode_c(instr), 255);
}

#[test]
fn encode_abc_roundtrip() {
    for op in 0_u8..=Opcode::MAX {
        let opcode = Opcode::from_u8(op).unwrap();
        for dest in [0_u8, 1, 127, 255] {
            for op_b in [0_u8, 1, 127, 255] {
                for op_c in [0_u8, 1, 127, 255] {
                    let instr = encode_abc(opcode, dest, op_b, op_c);
                    assert_eq!(decode_op(instr), Some(opcode));
                    assert_eq!(decode_a(instr), dest);
                    assert_eq!(decode_b(instr), op_b);
                    assert_eq!(decode_c(instr), op_c);
                }
            }
        }
    }
}

// =============================================================================
// iABx Encoding/Decoding Tests
// =============================================================================

#[test]
fn encode_abx_basic() {
    let instr = encode_abx(Opcode::LoadK, 5, 1000);
    assert_eq!(decode_op(instr), Some(Opcode::LoadK));
    assert_eq!(decode_a(instr), 5);
    assert_eq!(decode_bx(instr), 1000);
}

#[test]
fn encode_abx_max_values() {
    let instr = encode_abx(Opcode::GetGlobal, 255, 65535);
    assert_eq!(decode_op(instr), Some(Opcode::GetGlobal));
    assert_eq!(decode_a(instr), 255);
    assert_eq!(decode_bx(instr), 65535);
}

#[test]
fn encode_abx_roundtrip() {
    for bx in [0_u16, 1, 1000, 32767, 65535] {
        let instr = encode_abx(Opcode::LoadK, 42, bx);
        assert_eq!(decode_op(instr), Some(Opcode::LoadK));
        assert_eq!(decode_a(instr), 42);
        assert_eq!(decode_bx(instr), bx);
    }
}

// =============================================================================
// iAsBx Encoding/Decoding Tests
// =============================================================================

#[test]
fn encode_asbx_positive() {
    let instr = encode_asbx(Opcode::Jump, 0, 100);
    assert_eq!(decode_op(instr), Some(Opcode::Jump));
    assert_eq!(decode_a(instr), 0);
    assert_eq!(decode_sbx(instr), 100);
}

#[test]
fn encode_asbx_negative() {
    let instr = encode_asbx(Opcode::Jump, 0, -100);
    assert_eq!(decode_op(instr), Some(Opcode::Jump));
    assert_eq!(decode_a(instr), 0);
    assert_eq!(decode_sbx(instr), -100);
}

#[test]
fn encode_asbx_zero() {
    let instr = encode_asbx(Opcode::JumpIf, 5, 0);
    assert_eq!(decode_op(instr), Some(Opcode::JumpIf));
    assert_eq!(decode_a(instr), 5);
    assert_eq!(decode_sbx(instr), 0);
}

#[test]
fn encode_asbx_extremes() {
    // Maximum positive
    let instr = encode_asbx(Opcode::Jump, 0, i16::MAX);
    assert_eq!(decode_sbx(instr), i16::MAX);

    // Maximum negative
    let instr = encode_asbx(Opcode::Jump, 0, i16::MIN);
    assert_eq!(decode_sbx(instr), i16::MIN);
}

#[test]
fn encode_asbx_roundtrip() {
    for sbx in [i16::MIN, -1000, -1, 0, 1, 1000, i16::MAX] {
        let instr = encode_asbx(Opcode::Jump, 10, sbx);
        assert_eq!(decode_op(instr), Some(Opcode::Jump));
        assert_eq!(decode_a(instr), 10);
        assert_eq!(decode_sbx(instr), sbx);
    }
}

// =============================================================================
// RK (Register/Constant) Tests
// =============================================================================

#[test]
fn rk_register_valid() {
    assert_eq!(rk_register(0), Some(0));
    assert_eq!(rk_register(127), Some(127));
}

#[test]
fn rk_register_invalid() {
    assert_eq!(rk_register(128), None);
    assert_eq!(rk_register(255), None);
}

#[test]
fn rk_constant_valid() {
    assert_eq!(rk_constant(0), Some(0x80));
    assert_eq!(rk_constant(127), Some(0xFF));
}

#[test]
fn rk_constant_invalid() {
    assert_eq!(rk_constant(128), None);
    assert_eq!(rk_constant(255), None);
}

#[test]
fn rk_is_constant_check() {
    // Registers (0-127)
    assert!(!rk_is_constant(0));
    assert!(!rk_is_constant(127));

    // Constants (128-255)
    assert!(rk_is_constant(128));
    assert!(rk_is_constant(255));
}

#[test]
fn rk_index_extraction() {
    // Register index
    assert_eq!(rk_index(0), 0);
    assert_eq!(rk_index(127), 127);

    // Constant index (mask off high bit)
    assert_eq!(rk_index(128), 0);
    assert_eq!(rk_index(255), 127);
}

#[test]
fn rk_roundtrip() {
    // Register roundtrip
    for reg in 0_u8..=127 {
        let encoded = rk_register(reg).unwrap();
        assert!(!rk_is_constant(encoded));
        assert_eq!(rk_index(encoded), reg);
    }

    // Constant roundtrip
    for const_idx in 0_u8..=127 {
        let encoded = rk_constant(const_idx).unwrap();
        assert!(rk_is_constant(encoded));
        assert_eq!(rk_index(encoded), const_idx);
    }
}

// =============================================================================
// Invalid Opcode Decoding
// =============================================================================

#[test]
fn decode_invalid_opcode() {
    // Create instruction with invalid opcode byte
    let invalid_instr = 0xFF_u32; // opcode = 255, which is invalid
    assert_eq!(decode_op(invalid_instr), None);
    assert_eq!(decode_opcode_byte(invalid_instr), 255);
}
