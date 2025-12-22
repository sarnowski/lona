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

mod encoding;
#[cfg(test)]
mod tests;

pub use encoding::{
    RK_CONSTANT_BIT, RK_MAX_CONSTANT, RK_MAX_REGISTER, decode_a, decode_b, decode_bx, decode_c,
    decode_op, decode_opcode_byte, decode_sbx, encode_abc, encode_abx, encode_asbx, rk_constant,
    rk_index, rk_is_constant, rk_register,
};

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

    // =========================================================================
    // Var and Metadata Operations
    // =========================================================================
    /// Set global metadata: `globals[K[Bx]].merge_meta(R[A])`
    ///
    /// Format: iABx
    /// - A: register containing metadata Map
    /// - Bx: constant index of target Symbol
    ///
    /// Merges the metadata map in `R[A]` into the existing Var's metadata.
    /// If `R[A]` is nil, this is a no-op.
    SetGlobalMeta = 25,

    /// Get global var (not value): `R[A] = globals.get_var(K[Bx])`
    ///
    /// Format: iABx
    /// - A: destination register for the Var
    /// - Bx: constant index of Symbol
    ///
    /// Unlike `GetGlobal` which returns the Var's value, this returns the
    /// Var itself. Used for `(var x)` and `#'x` syntax to access metadata.
    GetGlobalVar = 26,

    // =========================================================================
    // Closure Operations
    // =========================================================================
    /// Get upvalue: `R[A] = Upvalues[B]`
    ///
    /// Format: iABC (C unused)
    /// - A: destination register
    /// - B: upvalue index
    ///
    /// Reads a captured value from the current closure's upvalue array.
    GetUpvalue = 27,

    /// Create closure: `R[A] = closure(K[Bx])`
    ///
    /// Format: iABx
    /// - A: destination register
    /// - Bx: constant index of function template
    ///
    /// Creates a closure by:
    /// 1. Loading the function template from K\[Bx\]
    /// 2. Copying captured values according to `upvalue_sources`
    /// 3. Storing the new Function in R\[A\]
    Closure = 28,
}

impl Opcode {
    /// Maximum valid opcode value.
    pub const MAX: u8 = 28;

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
            25 => Some(Self::SetGlobalMeta),
            26 => Some(Self::GetGlobalVar),
            27 => Some(Self::GetUpvalue),
            28 => Some(Self::Closure),
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
            Self::SetGlobalMeta => "SetGlobalMeta",
            Self::GetGlobalVar => "GetGlobalVar",
            Self::GetUpvalue => "GetUpvalue",
            Self::Closure => "Closure",
        }
    }
}

impl fmt::Display for Opcode {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
