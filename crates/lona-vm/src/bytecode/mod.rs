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

use crate::term::Term;

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
    /// Build tuple: `X(A) := [X(B)..X(B+C-1)]` (C elements starting at X(B))
    pub const BUILD_TUPLE: u8 = 8;
    /// Build map: `X(A) := %{X(B)..X(B+C*2-1)}` (C key-value pairs starting at X(B))
    pub const BUILD_MAP: u8 = 9;
    /// Call function: `X0 := X(A)(X1..X(B))` where X(A) is a callable value.
    ///
    /// Format: `CALL fn_reg, argc` (result goes to X0).
    pub const CALL: u8 = 10;
    /// Build closure: `X(A) := Closure(X(B), X(C))` where X(B) is `CompiledFn`, X(C) is captures tuple.
    ///
    /// Format: `BUILD_CLOSURE target, fn_reg, captures_reg`
    pub const BUILD_CLOSURE: u8 = 11;
    /// Build vector: `X(A) := {X(B)..X(B+C-1)}` (C elements starting at X(B))
    pub const BUILD_VECTOR: u8 = 12;

    // --- Y Register Instructions ---

    /// Allocate Y registers for current frame (uninitialized).
    ///
    /// Format: `ALLOCATE A, B`
    /// - A: number of Y registers to allocate (0-63)
    /// - B: number of live X registers (for future GC)
    ///
    /// The frame must already exist (created by CALL). This extends it with Y register slots.
    /// Y registers are NOT initialized - use `ALLOCATE_ZERO` for GC-safe initialization.
    pub const ALLOCATE: u8 = 13;

    /// Allocate Y registers and initialize to nil (GC-safe).
    ///
    /// Format: `ALLOCATE_ZERO A, B`
    /// - A: number of Y registers to allocate (0-63)
    /// - B: number of live X registers (for future GC)
    ///
    /// Same as ALLOCATE, but initializes all Y registers to nil.
    /// Use this when Y registers may not be assigned before a potential GC point.
    pub const ALLOCATE_ZERO: u8 = 14;

    /// Deallocate Y registers before return.
    ///
    /// Format: `DEALLOCATE A`
    /// - A: number of Y registers to release (must match ALLOCATE)
    ///
    /// Releases Y register space. Must be called before RETURN if ALLOCATE was used.
    pub const DEALLOCATE: u8 = 15;

    /// Move X register to Y register: `Y(A) := X(B)`.
    ///
    /// Format: `MOVE_XY A, B`
    /// - A: Y register index (destination)
    /// - B: X register index (source)
    ///
    /// Saves a value from an X register to a Y register for preservation across calls.
    pub const MOVE_XY: u8 = 16;

    /// Move Y register to X register: `X(A) := Y(B)`.
    ///
    /// Format: `MOVE_YX A, B`
    /// - A: X register index (destination)
    /// - B: Y register index (source)
    ///
    /// Restores a value from a Y register to an X register after a call.
    pub const MOVE_YX: u8 = 17;

    // --- Pattern Matching Instructions ---

    /// Test if X(A) is nil. Jump to Bx if NOT nil.
    ///
    /// Format: `IS_NIL A, Bx`
    /// - A: register to test
    /// - Bx: fail label (instruction offset to jump to if test fails)
    pub const IS_NIL: u8 = 18;

    /// Test if X(A) is a boolean. Jump to Bx if NOT boolean.
    ///
    /// Format: `IS_BOOL A, Bx`
    pub const IS_BOOL: u8 = 19;

    /// Test if X(A) is an integer. Jump to Bx if NOT integer.
    ///
    /// Format: `IS_INT A, Bx`
    pub const IS_INT: u8 = 20;

    /// Test if X(A) is a tuple. Jump to Bx if NOT tuple.
    ///
    /// Format: `IS_TUPLE A, Bx`
    pub const IS_TUPLE: u8 = 21;

    /// Test if X(A) is a vector. Jump to Bx if NOT vector.
    ///
    /// Format: `IS_VECTOR A, Bx`
    pub const IS_VECTOR: u8 = 22;

    /// Test if X(A) is a map. Jump to Bx if NOT map.
    ///
    /// Format: `IS_MAP A, Bx`
    pub const IS_MAP: u8 = 23;

    /// Test if X(A) is a keyword. Jump to Bx if NOT keyword.
    ///
    /// Format: `IS_KEYWORD A, Bx`
    pub const IS_KEYWORD: u8 = 24;

    /// Test if X(A) is a string. Jump to Bx if NOT string.
    ///
    /// Format: `IS_STRING A, Bx`
    pub const IS_STRING: u8 = 25;

    /// Test tuple arity: Jump to C if X(A) is not a tuple of length B.
    ///
    /// Format: `TEST_ARITY A, B, C`
    /// - A: register containing tuple
    /// - B: expected arity
    /// - C: fail label (jump if arity mismatch or not a tuple)
    pub const TEST_ARITY: u8 = 26;

    /// Test vector length: Jump to C if X(A) is not a vector of length B.
    ///
    /// Format: `TEST_VEC_LEN A, B, C`
    /// - A: register containing vector
    /// - B: expected length
    /// - C: fail label (jump if length mismatch or not a vector)
    pub const TEST_VEC_LEN: u8 = 27;

    /// Get tuple element: `X(A) := tuple(X(B))[C]`.
    ///
    /// Format: `GET_TUPLE_ELEM A, B, C`
    /// - A: destination register
    /// - B: register containing tuple
    /// - C: element index (0-based)
    ///
    /// Extracts element at index C from the tuple in X(B) and stores it in X(A).
    /// Caller must ensure X(B) is a tuple and C is within bounds.
    pub const GET_TUPLE_ELEM: u8 = 28;

    /// Get vector element: `X(A) := vector(X(B))[C]`.
    ///
    /// Format: `GET_VEC_ELEM A, B, C`
    /// - A: destination register
    /// - B: register containing vector
    /// - C: element index (0-based)
    ///
    /// Extracts element at index C from the vector in X(B) and stores it in X(A).
    /// Caller must ensure X(B) is a vector and C is within bounds.
    pub const GET_VEC_ELEM: u8 = 29;

    /// Exact equality test: Jump to C if X(A) != X(B).
    ///
    /// Format: `IS_EQ A, B, C`
    /// - A: first register
    /// - B: second register
    /// - C: fail label (jump if not equal)
    ///
    /// Tests structural equality. Falls through if equal, jumps to C if not equal.
    pub const IS_EQ: u8 = 30;

    /// Unconditional jump: `IP := Bx`.
    ///
    /// Format: `JUMP _, Bx`
    /// - Bx: target instruction offset (absolute)
    ///
    /// Jumps unconditionally to the instruction at offset Bx.
    pub const JUMP: u8 = 31;

    /// Conditional jump: if X(A) is falsy, `IP := Bx`.
    ///
    /// Format: `JUMP_IF_FALSE A, Bx`
    /// - A: register to test
    /// - Bx: target instruction offset (absolute)
    ///
    /// Falsy values are nil and false. All other values are truthy.
    /// Jumps to Bx if X(A) is falsy, otherwise falls through.
    pub const JUMP_IF_FALSE: u8 = 32;

    /// Extract tuple slice: `X(A) := tuple(X(B))[C..]`.
    ///
    /// Format: `TUPLE_SLICE A, B, C`
    /// - A: destination register for new tuple
    /// - B: register containing source tuple
    /// - C: start index (0-based)
    ///
    /// Creates a new tuple containing elements from index C to end of tuple.
    /// If C >= tuple length, creates an empty tuple.
    /// Caller must ensure X(B) is a tuple.
    pub const TUPLE_SLICE: u8 = 33;

    /// Test minimum tuple arity: Jump to C if X(A) has fewer than B elements.
    ///
    /// Format: `TEST_ARITY_GE A, B, C`
    /// - A: register containing tuple
    /// - B: minimum required arity
    /// - C: fail label (jump if arity < B or not a tuple)
    ///
    /// Used for tuple rest patterns like `[a b & t]` which require at least 2 elements.
    pub const TEST_ARITY_GE: u8 = 34;

    /// Raise badmatch error: process exits with `[:error :badmatch %{:value X(A)}]`.
    ///
    /// Format: `BADMATCH A`
    /// - A: register containing the value that failed to match
    ///
    /// Terminates the process with `RuntimeError::Badmatch`.
    pub const BADMATCH: u8 = 35;
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
    pub constants: Vec<Term>,
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
    pub fn add_constant(&mut self, term: Term) -> Option<u32> {
        let index = self.constants.len();
        if index > BX_MASK as usize {
            return None;
        }
        self.constants.push(term);
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
