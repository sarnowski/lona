// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Bytecode format for the Lona VM.
//!
//! The VM uses fixed 32-bit instructions with two encoding formats:
//!
//! - Format A: `[opcode:6][A:8][B:9][C:9]` - for register operations
//! - Format B: `[opcode:6][A:8][Bx:18]` - for immediates and constants
//!
//! See `docs/architecture/virtual-machine.md` for the full specification.

#[cfg(test)]
mod bytecode_test;

use crate::value::Value;

#[cfg(any(test, feature = "std"))]
use std::vec::Vec;

#[cfg(not(any(test, feature = "std")))]
use alloc::vec::Vec;

/// Bytecode opcodes (6 bits, values 0-63).
pub mod op {
    /// Load nil into register: `X(A) := nil`
    pub const LOADNIL: u8 = 0;
    /// Load boolean into register: `X(A) := (Bx != 0)`
    pub const LOADBOOL: u8 = 1;
    /// Load small integer: `X(A) := Bx` (18-bit signed)
    pub const LOADINT: u8 = 2;
    /// Load constant: `X(A) := constants[Bx]`
    pub const LOADK: u8 = 3;
    /// Move register: `X(A) := X(B)`
    pub const MOVE: u8 = 4;
    /// Call intrinsic: `X0 := intrinsic(A)(X1..X(B))`
    pub const INTRINSIC: u8 = 5;
    /// Return: `return X0`
    pub const RETURN: u8 = 6;
    /// Halt execution: stop VM
    pub const HALT: u8 = 7;
}

/// Bit widths for instruction fields.
const OPCODE_BITS: u32 = 6;
const A_BITS: u32 = 8;
const B_BITS: u32 = 9;
const C_BITS: u32 = 9;
const BX_BITS: u32 = 18;

/// Bit positions for instruction fields.
const OPCODE_SHIFT: u32 = 32 - OPCODE_BITS; // 26
const A_SHIFT: u32 = OPCODE_SHIFT - A_BITS; // 18
const B_SHIFT: u32 = A_SHIFT - B_BITS; // 9
const C_SHIFT: u32 = 0;
const BX_SHIFT: u32 = 0;

/// Bit masks for instruction fields.
const OPCODE_MASK: u32 = (1 << OPCODE_BITS) - 1; // 0x3F
const A_MASK: u32 = (1 << A_BITS) - 1; // 0xFF
const B_MASK: u32 = (1 << B_BITS) - 1; // 0x1FF
const C_MASK: u32 = (1 << C_BITS) - 1; // 0x1FF
/// Mask for the Bx field (18 bits).
pub const BX_MASK: u32 = (1 << BX_BITS) - 1; // 0x3FFFF

/// Maximum value for signed 18-bit immediate.
pub const MAX_SIGNED_BX: i32 = (1 << (BX_BITS - 1)) - 1; // 131071
/// Minimum value for signed 18-bit immediate.
pub const MIN_SIGNED_BX: i32 = -(1 << (BX_BITS - 1)); // -131072

/// Encode a Format A instruction: `[opcode:6][A:8][B:9][C:9]`
#[inline]
#[must_use]
pub const fn encode_abc(opcode: u8, a: u8, b: u16, c: u16) -> u32 {
    ((opcode as u32 & OPCODE_MASK) << OPCODE_SHIFT)
        | ((a as u32 & A_MASK) << A_SHIFT)
        | ((b as u32 & B_MASK) << B_SHIFT)
        | ((c as u32 & C_MASK) << C_SHIFT)
}

/// Encode a Format B instruction: `[opcode:6][A:8][Bx:18]`
#[inline]
#[must_use]
pub const fn encode_abx(opcode: u8, a: u8, bx: u32) -> u32 {
    ((opcode as u32 & OPCODE_MASK) << OPCODE_SHIFT)
        | ((a as u32 & A_MASK) << A_SHIFT)
        | ((bx & BX_MASK) << BX_SHIFT)
}

/// Decode the opcode from an instruction.
#[inline]
#[must_use]
pub const fn decode_opcode(instr: u32) -> u8 {
    ((instr >> OPCODE_SHIFT) & OPCODE_MASK) as u8
}

/// Decode the A field from an instruction.
#[inline]
#[must_use]
pub const fn decode_a(instr: u32) -> u8 {
    ((instr >> A_SHIFT) & A_MASK) as u8
}

/// Decode the B field from a Format A instruction.
#[inline]
#[must_use]
pub const fn decode_b(instr: u32) -> u16 {
    ((instr >> B_SHIFT) & B_MASK) as u16
}

/// Decode the C field from a Format A instruction.
#[inline]
#[must_use]
pub const fn decode_c(instr: u32) -> u16 {
    ((instr >> C_SHIFT) & C_MASK) as u16
}

/// Decode the Bx field (unsigned 18-bit) from a Format B instruction.
#[inline]
#[must_use]
pub const fn decode_bx(instr: u32) -> u32 {
    (instr >> BX_SHIFT) & BX_MASK
}

/// Decode the Bx field as a signed 18-bit integer.
///
/// Sign-extends the 18-bit value to i32. The casts from u32 to i32 are
/// intentional for two's complement sign extension.
#[inline]
#[must_use]
#[expect(
    clippy::cast_possible_wrap,
    reason = "intentional sign extension for 18-bit signed immediate"
)]
pub const fn decode_sbx(instr: u32) -> i32 {
    let bx = decode_bx(instr);
    // Sign bit is bit 17. If set, sign-extend by ORing with upper bits.
    if bx & (1 << 17) != 0 {
        // Negative: set upper 14 bits to 1
        (bx | !BX_MASK) as i32
    } else {
        bx as i32
    }
}

/// A compiled bytecode chunk.
///
/// Contains the instruction sequence and constant pool for a compiled expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// The instruction sequence (fixed 32-bit instructions).
    pub code: Vec<u32>,
    /// Constant pool (strings, large integers, etc.).
    pub constants: Vec<Value>,
}

impl Chunk {
    /// Create a new empty chunk.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    /// Emit an instruction to the code buffer.
    #[inline]
    pub fn emit(&mut self, instr: u32) {
        self.code.push(instr);
    }

    /// Add a constant to the pool and return its index.
    ///
    /// Returns `None` if the constant pool is full (max 262143 entries for 18-bit index).
    pub fn add_constant(&mut self, value: Value) -> Option<u32> {
        let index = self.constants.len();
        if index > BX_MASK as usize {
            return None;
        }
        self.constants.push(value);
        Some(index as u32)
    }

    /// Get the current code offset (number of instructions emitted).
    #[inline]
    #[must_use]
    pub fn code_len(&self) -> usize {
        self.code.len()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
