// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Instruction encoding and decoding functions.
//!
//! This module provides functions for encoding and decoding the three instruction
//! formats (iABC, iABx, iAsBx) and the RK (Register/Constant) field encoding.

use super::Opcode;

// =============================================================================
// Instruction Encoding
// =============================================================================

/// Encodes an iABC format instruction.
///
/// Layout: `[opcode:8][A:8][B:8][C:8]`
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "[approved] const fn requires as; widening u8->u32 is safe"
)]
pub const fn encode_abc(op: Opcode, dest: u8, op_b: u8, op_c: u8) -> u32 {
    let op_byte = op as u8;
    (op_byte as u32) | ((dest as u32) << 8) | ((op_b as u32) << 16) | ((op_c as u32) << 24)
}

/// Encodes an iABx format instruction.
///
/// Layout: `[opcode:8][A:8][Bx:16]`
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "[approved] const fn requires as; widening u8/u16->u32 is safe"
)]
pub const fn encode_abx(op: Opcode, dest: u8, bx: u16) -> u32 {
    let op_byte = op as u8;
    (op_byte as u32) | ((dest as u32) << 8) | ((bx as u32) << 16)
}

/// Encodes an iAsBx format instruction with a signed offset.
///
/// Layout: `[opcode:8][A:8][sBx:16]`
///
/// The signed offset is stored as an unsigned value with a bias of 32767,
/// allowing representation of -32768 to +32767.
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    clippy::cast_sign_loss,
    reason = "[approved] const fn requires as; i16->u16 bitcast is intentional"
)]
pub const fn encode_asbx(op: Opcode, dest: u8, sbx: i16) -> u32 {
    let op_byte = op as u8;
    // Store as unsigned (bitwise reinterpretation)
    let sbx_unsigned = sbx as u16;
    (op_byte as u32) | ((dest as u32) << 8) | ((sbx_unsigned as u32) << 16)
}

// =============================================================================
// Instruction Decoding
// =============================================================================

/// Decodes the opcode from an instruction.
///
/// Returns `None` if the opcode byte is invalid.
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "[approved] masking guarantees value fits in u8"
)]
pub const fn decode_op(instruction: u32) -> Option<Opcode> {
    let byte = (instruction & 0xFF) as u8;
    Opcode::from_u8(byte)
}

/// Decodes the opcode byte from an instruction without validation.
///
/// Use `decode_op` for safe decoding with validation.
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "[approved] masking guarantees value fits in u8"
)]
pub const fn decode_opcode_byte(instruction: u32) -> u8 {
    (instruction & 0xFF) as u8
}

/// Decodes field A (bits 8-15) from an instruction.
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "[approved] masking and shifting guarantees value fits in u8"
)]
pub const fn decode_a(instruction: u32) -> u8 {
    ((instruction >> 8) & 0xFF) as u8
}

/// Decodes field B (bits 16-23) from an iABC instruction.
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "[approved] masking and shifting guarantees value fits in u8"
)]
pub const fn decode_b(instruction: u32) -> u8 {
    ((instruction >> 16) & 0xFF) as u8
}

/// Decodes field C (bits 24-31) from an iABC instruction.
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "[approved] masking and shifting guarantees value fits in u8"
)]
pub const fn decode_c(instruction: u32) -> u8 {
    ((instruction >> 24) & 0xFF) as u8
}

/// Decodes field Bx (bits 16-31) from an iABx instruction.
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "[approved] shifting guarantees value fits in u16"
)]
pub const fn decode_bx(instruction: u32) -> u16 {
    (instruction >> 16) as u16
}

/// Decodes field sBx (bits 16-31) as a signed offset from an iAsBx instruction.
#[inline]
#[must_use]
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_wrap,
    reason = "[approved] u16 to i16 bitcast is intentional for signed decoding"
)]
pub const fn decode_sbx(instruction: u32) -> i16 {
    let unsigned = (instruction >> 16_i32) as u16;
    unsigned as i16
}

// =============================================================================
// RK (Register/Constant) Encoding
// =============================================================================

/// The bit that indicates a constant index vs register index in B/C fields.
///
/// Values with this bit set refer to constants (index = value & 0x7F).
/// Values without this bit refer to registers.
pub const RK_CONSTANT_BIT: u8 = 0x80;

/// Maximum register index that can be used in RK fields.
pub const RK_MAX_REGISTER: u8 = 127;

/// Maximum constant index that can be used in RK fields.
pub const RK_MAX_CONSTANT: u8 = 127;

/// Encodes a register index for use in an RK field.
///
/// Returns `None` if the register index is too large (> 127).
#[inline]
#[must_use]
pub const fn rk_register(register: u8) -> Option<u8> {
    if register > RK_MAX_REGISTER {
        None
    } else {
        Some(register)
    }
}

/// Encodes a constant index for use in an RK field.
///
/// Returns `None` if the constant index is too large (> 127).
#[inline]
#[must_use]
pub const fn rk_constant(constant: u8) -> Option<u8> {
    if constant > RK_MAX_CONSTANT {
        None
    } else {
        Some(constant | RK_CONSTANT_BIT)
    }
}

/// Returns `true` if the RK field value refers to a constant.
#[inline]
#[must_use]
pub const fn rk_is_constant(rk: u8) -> bool {
    (rk & RK_CONSTANT_BIT) != 0
}

/// Extracts the index from an RK field value.
///
/// For constants, returns the constant pool index.
/// For registers, returns the register index.
#[inline]
#[must_use]
pub const fn rk_index(rk: u8) -> u8 {
    rk & 0x7F
}
