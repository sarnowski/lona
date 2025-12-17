// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Opcode definitions and instruction encoding/decoding.
//!
//! Lonala uses fixed 32-bit instructions in three formats:
//!
//! - `iABC`:  `[opcode:8][A:8][B:8][C:8]` - Three register operands
//! - `iABx`:  `[opcode:8][A:8][Bx:16]` - Register and extended operand
//! - `iAsBx`: `[opcode:8][A:8][sBx:16]` - Register and signed offset
//!
//! See `docs/architecture/register-based-vm.md` (from the repository root) for design rationale.

use core::fmt;

/// Bytecode opcodes for the Lonala virtual machine.
///
/// Each opcode specifies an operation the VM executes. Opcodes are encoded
/// in the lowest 8 bits of each 32-bit instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum Opcode {
    // =========================================================================
    // Data Movement
    // =========================================================================
    /// Copy register: `R[A] = R[B]`
    ///
    /// Format: iABC (C unused)
    Move = 0,

    /// Load constant: `R[A] = K[Bx]`
    ///
    /// Format: iABx
    LoadK = 1,

    /// Load nil to registers: `R[A]..R[A+B] = nil`
    ///
    /// Format: iABC (C unused)
    LoadNil = 2,

    /// Load true: `R[A] = true`
    ///
    /// Format: iABC (B, C unused)
    LoadTrue = 3,

    /// Load false: `R[A] = false`
    ///
    /// Format: iABC (B, C unused)
    LoadFalse = 4,

    // =========================================================================
    // Global Variables
    // =========================================================================
    /// Get global: `R[A] = globals[K[Bx]]`
    ///
    /// Format: iABx
    GetGlobal = 5,

    /// Set global: `globals[K[Bx]] = R[A]`
    ///
    /// Format: iABx
    SetGlobal = 6,

    // =========================================================================
    // Arithmetic
    // =========================================================================
    /// Addition: `R[A] = RK[B] + RK[C]`
    ///
    /// Format: iABC
    Add = 7,

    /// Subtraction: `R[A] = RK[B] - RK[C]`
    ///
    /// Format: iABC
    Sub = 8,

    /// Multiplication: `R[A] = RK[B] * RK[C]`
    ///
    /// Format: iABC
    Mul = 9,

    /// Division: `R[A] = RK[B] / RK[C]`
    ///
    /// Format: iABC
    Div = 10,

    /// Modulo: `R[A] = RK[B] % RK[C]`
    ///
    /// Format: iABC
    Mod = 11,

    /// Negation: `R[A] = -R[B]`
    ///
    /// Format: iABC (C unused)
    Neg = 12,

    // =========================================================================
    // Comparison
    // =========================================================================
    /// Equality: `R[A] = RK[B] == RK[C]`
    ///
    /// Format: iABC
    Eq = 13,

    /// Less than: `R[A] = RK[B] < RK[C]`
    ///
    /// Format: iABC
    Lt = 14,

    /// Less or equal: `R[A] = RK[B] <= RK[C]`
    ///
    /// Format: iABC
    Le = 15,

    /// Greater than: `R[A] = RK[B] > RK[C]`
    ///
    /// Format: iABC
    Gt = 16,

    /// Greater or equal: `R[A] = RK[B] >= RK[C]`
    ///
    /// Format: iABC
    Ge = 17,

    /// Logical not: `R[A] = not R[B]`
    ///
    /// Format: iABC (C unused)
    Not = 18,

    // =========================================================================
    // Control Flow
    // =========================================================================
    /// Unconditional jump: `PC += sBx`
    ///
    /// Format: iAsBx (A unused)
    Jump = 19,

    /// Conditional jump: `if R[A] then PC += sBx`
    ///
    /// Format: iAsBx
    JumpIf = 20,

    /// Conditional jump (negated): `if not R[A] then PC += sBx`
    ///
    /// Format: iAsBx
    JumpIfNot = 21,

    // =========================================================================
    // Function Calls
    // =========================================================================
    /// Function call: `R[A]..R[A+C-1] = R[A](R[A+1]..R[A+B])`
    ///
    /// Format: iABC
    /// - A: base register (function, then results)
    /// - B: argument count
    /// - C: result count (0 = multiple results)
    Call = 22,

    /// Tail call: `return R[A](R[A+1]..R[A+B])`
    ///
    /// Format: iABC (C unused)
    /// - A: base register (function)
    /// - B: argument count
    TailCall = 23,

    /// Return: `return R[A]..R[A+B-1]`
    ///
    /// Format: iABC (C unused)
    /// - A: first return value register
    /// - B: return value count (0 = return to top of stack)
    Return = 24,
}

impl Opcode {
    /// Maximum valid opcode value.
    pub const MAX: u8 = 24;

    /// Converts a byte to an opcode, returning `None` for invalid values.
    #[inline]
    #[must_use]
    pub const fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Move),
            1 => Some(Self::LoadK),
            2 => Some(Self::LoadNil),
            3 => Some(Self::LoadTrue),
            4 => Some(Self::LoadFalse),
            5 => Some(Self::GetGlobal),
            6 => Some(Self::SetGlobal),
            7 => Some(Self::Add),
            8 => Some(Self::Sub),
            9 => Some(Self::Mul),
            10 => Some(Self::Div),
            11 => Some(Self::Mod),
            12 => Some(Self::Neg),
            13 => Some(Self::Eq),
            14 => Some(Self::Lt),
            15 => Some(Self::Le),
            16 => Some(Self::Gt),
            17 => Some(Self::Ge),
            18 => Some(Self::Not),
            19 => Some(Self::Jump),
            20 => Some(Self::JumpIf),
            21 => Some(Self::JumpIfNot),
            22 => Some(Self::Call),
            23 => Some(Self::TailCall),
            24 => Some(Self::Return),
            _ => None,
        }
    }

    /// Returns the opcode's name as a static string.
    #[inline]
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::LoadK => "LoadK",
            Self::LoadNil => "LoadNil",
            Self::LoadTrue => "LoadTrue",
            Self::LoadFalse => "LoadFalse",
            Self::GetGlobal => "GetGlobal",
            Self::SetGlobal => "SetGlobal",
            Self::Add => "Add",
            Self::Sub => "Sub",
            Self::Mul => "Mul",
            Self::Div => "Div",
            Self::Mod => "Mod",
            Self::Neg => "Neg",
            Self::Eq => "Eq",
            Self::Lt => "Lt",
            Self::Le => "Le",
            Self::Gt => "Gt",
            Self::Ge => "Ge",
            Self::Not => "Not",
            Self::Jump => "Jump",
            Self::JumpIf => "JumpIf",
            Self::JumpIfNot => "JumpIfNot",
            Self::Call => "Call",
            Self::TailCall => "TailCall",
            Self::Return => "Return",
        }
    }
}

impl fmt::Display for Opcode {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Opcode Tests
    // =========================================================================

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

    // =========================================================================
    // iABC Encoding/Decoding Tests
    // =========================================================================

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

    // =========================================================================
    // iABx Encoding/Decoding Tests
    // =========================================================================

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

    // =========================================================================
    // iAsBx Encoding/Decoding Tests
    // =========================================================================

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

    // =========================================================================
    // RK (Register/Constant) Tests
    // =========================================================================

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

    // =========================================================================
    // Invalid Opcode Decoding
    // =========================================================================

    #[test]
    fn decode_invalid_opcode() {
        // Create instruction with invalid opcode byte
        let invalid_instr = 0xFF_u32; // opcode = 255, which is invalid
        assert_eq!(decode_op(invalid_instr), None);
        assert_eq!(decode_opcode_byte(invalid_instr), 255);
    }
}
