// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Bytecode chunk and constant pool structures.
//!
//! A `Chunk` represents a compiled function body or top-level expression.
//! It contains the bytecode instructions, a constant pool, and metadata
//! for debugging and execution.
//!
//! See `docs/architecture/register-based-vm.md` (from the repository root) for design rationale.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Write as _};

use lona_core::symbol;
use lonala_parser::Span;

use crate::error::Error;
use crate::opcode::{
    Opcode, decode_a, decode_b, decode_bx, decode_c, decode_op, decode_sbx, rk_index,
    rk_is_constant,
};

/// A constant value stored in a chunk's constant pool.
///
/// Constants are referenced by index from `LoadK` instructions and
/// from the high bits of RK operands in arithmetic instructions.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Constant {
    /// The nil value.
    Nil,
    /// A boolean value.
    Bool(bool),
    /// A 64-bit signed integer.
    Integer(i64),
    /// A 64-bit floating-point number.
    Float(f64),
    /// A string value.
    String(String),
    /// An interned symbol identifier.
    Symbol(symbol::Id),
}

impl fmt::Display for Constant {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Nil => write!(f, "nil"),
            Self::Bool(val) => write!(f, "{val}"),
            Self::Integer(num) => write!(f, "{num}"),
            Self::Float(num) => write!(f, "{num}"),
            Self::String(ref text) => write!(f, "\"{text}\""),
            Self::Symbol(id) => write!(f, "sym#{}", id.as_u32()),
        }
    }
}

/// A compiled bytecode chunk.
///
/// Represents a function body or top-level expression. Contains all the
/// information needed for the VM to execute the code.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Chunk {
    /// Bytecode instructions.
    code: Vec<u32>,
    /// Constant pool.
    constants: Vec<Constant>,
    /// Maximum registers used by this chunk.
    max_registers: u8,
    /// Number of parameters (0 for top-level code).
    arity: u8,
    /// Source spans for each instruction (parallel to `code`).
    spans: Vec<Span>,
    /// Function name for debugging (empty for anonymous/top-level).
    name: String,
}

impl Chunk {
    /// Creates a new empty chunk.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            max_registers: 0,
            arity: 0,
            spans: Vec::new(),
            name: String::new(),
        }
    }

    /// Creates a new chunk with the given name.
    #[inline]
    #[must_use]
    pub const fn with_name(name: String) -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            max_registers: 0,
            arity: 0,
            spans: Vec::new(),
            name,
        }
    }

    /// Returns the chunk's name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the chunk's name.
    #[inline]
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Returns the number of parameters this chunk expects.
    #[inline]
    #[must_use]
    pub const fn arity(&self) -> u8 {
        self.arity
    }

    /// Sets the number of parameters.
    #[inline]
    pub const fn set_arity(&mut self, arity: u8) {
        self.arity = arity;
    }

    /// Returns the maximum number of registers used.
    #[inline]
    #[must_use]
    pub const fn max_registers(&self) -> u8 {
        self.max_registers
    }

    /// Sets the maximum number of registers.
    #[inline]
    pub const fn set_max_registers(&mut self, count: u8) {
        self.max_registers = count;
    }

    /// Emits an instruction with its source span, returning the instruction index.
    ///
    /// # Errors
    ///
    /// Returns `Error::TooManyConstants` if the code section would exceed
    /// the maximum size (though this is unlikely in practice).
    #[inline]
    pub fn emit(&mut self, instruction: u32, span: Span) -> usize {
        let index = self.code.len();
        self.code.push(instruction);
        self.spans.push(span);
        index
    }

    /// Patches an instruction at the given index.
    ///
    /// Used for fixing up jump targets after the target is known.
    #[inline]
    pub fn patch(&mut self, index: usize, instruction: u32) {
        if let Some(slot) = self.code.get_mut(index) {
            *slot = instruction;
        }
    }

    /// Returns the current instruction count (next instruction's index).
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.code.len()
    }

    /// Returns `true` if the chunk has no instructions.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    /// Adds a constant to the constant pool, returning its index.
    ///
    /// This method does not track source spans - if you need span-aware
    /// error reporting, use the compiler's `add_constant` wrapper instead.
    ///
    /// # Errors
    ///
    /// Returns `Error::TooManyConstants` if the constant pool is full (> 65535).
    #[inline]
    pub fn add_constant(&mut self, constant: Constant) -> Result<u16, Error> {
        let index = self.constants.len();
        let index_u16 = u16::try_from(index).map_err(|_err| Error::TooManyConstants {
            span: Span::new(0_usize, 0_usize),
        })?;
        self.constants.push(constant);
        Ok(index_u16)
    }

    /// Adds a constant with source span for error reporting.
    ///
    /// # Errors
    ///
    /// Returns `Error::TooManyConstants` with the span if the pool is full.
    #[inline]
    pub fn add_constant_at(&mut self, constant: Constant, span: Span) -> Result<u16, Error> {
        let index = self.constants.len();
        let index_u16 = u16::try_from(index).map_err(|_err| Error::TooManyConstants { span })?;
        self.constants.push(constant);
        Ok(index_u16)
    }

    /// Gets a constant by index.
    #[inline]
    #[must_use]
    pub fn get_constant(&self, index: u16) -> Option<&Constant> {
        self.constants.get(usize::from(index))
    }

    /// Returns the bytecode instructions.
    #[inline]
    #[must_use]
    pub fn code(&self) -> &[u32] {
        &self.code
    }

    /// Returns the constant pool.
    #[inline]
    #[must_use]
    pub fn constants(&self) -> &[Constant] {
        &self.constants
    }

    /// Returns the source spans for instructions.
    #[inline]
    #[must_use]
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// Returns the span for an instruction at the given index.
    #[inline]
    #[must_use]
    pub fn span_at(&self, index: usize) -> Option<Span> {
        self.spans.get(index).copied()
    }

    /// Disassembles the entire chunk to a human-readable string.
    #[inline]
    #[must_use]
    pub fn disassemble(&self) -> String {
        let mut output = String::new();

        // Header
        let display_name = if self.name.is_empty() {
            "<anonymous>"
        } else {
            &self.name
        };
        let _result = writeln!(output, "== {display_name} ==");
        let _result = writeln!(
            output,
            "arity: {}, max_registers: {}",
            self.arity, self.max_registers
        );
        let _result = writeln!(output);

        // Instructions
        for (offset, &instruction) in self.code.iter().enumerate() {
            let line = self.disassemble_instruction(offset, instruction);
            let _result = writeln!(output, "{line}");
        }

        // Constants
        if !self.constants.is_empty() {
            let _result = writeln!(output);
            let _result = writeln!(output, "Constants:");
            for (i, constant) in self.constants.iter().enumerate() {
                let _result = writeln!(output, "  K{i}: {constant}");
            }
        }

        output
    }

    /// Disassembles a single instruction to a human-readable string.
    #[inline]
    #[must_use]
    pub fn disassemble_instruction(&self, offset: usize, instruction: u32) -> String {
        let mut output = String::new();

        // Offset
        let _result = write!(output, "{offset:04}    ");

        // Source line (from span)
        if let Some(span) = self.spans.get(offset) {
            let _result = write!(output, "{:4} ", span.start);
        } else {
            let _result = write!(output, "   ? ");
        }

        // Decode and format
        match decode_op(instruction) {
            Some(op) => {
                let _result = write!(output, "{:<12}", op.name());
                self.format_operands(&mut output, op, instruction, offset);
            }
            None => {
                let _result = write!(output, "INVALID     0x{instruction:08X}");
            }
        }

        output
    }

    /// Formats the operands for an instruction.
    fn format_operands(&self, output: &mut String, op: Opcode, instruction: u32, offset: usize) {
        let reg_a = decode_a(instruction);
        let reg_b = decode_b(instruction);
        let reg_c = decode_c(instruction);
        let bx = decode_bx(instruction);
        let sbx = decode_sbx(instruction);

        match op {
            // iABC with all three operands
            Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Eq
            | Opcode::Lt
            | Opcode::Le
            | Opcode::Gt
            | Opcode::Ge => {
                let _result = write!(output, "R{reg_a}, ");
                Self::format_rk(output, reg_b);
                let _result = write!(output, ", ");
                Self::format_rk(output, reg_c);
            }

            // iABC with A and B
            Opcode::Move | Opcode::Neg | Opcode::Not => {
                let _result = write!(output, "R{reg_a}, R{reg_b}");
            }

            // iABC with A and B (for range)
            Opcode::LoadNil => {
                let _result = write!(
                    output,
                    "R{reg_a}..R{}",
                    u16::from(reg_a).saturating_add(u16::from(reg_b))
                );
            }

            // iABC with just A
            Opcode::LoadTrue | Opcode::LoadFalse => {
                let _result = write!(output, "R{reg_a}");
            }

            // iABx format
            Opcode::LoadK | Opcode::GetGlobal | Opcode::SetGlobal => {
                let _result = write!(output, "R{reg_a}, K{bx}");
                if let Some(constant) = self.get_constant(bx) {
                    let _result = write!(output, "        ; {constant}");
                }
            }

            // iAsBx format (jumps)
            Opcode::Jump => {
                let _result = write!(output, "{sbx}");
                // Show target address
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_possible_wrap,
                    reason = "instruction offset is small; used for display only"
                )]
                let target = (offset as i64)
                    .saturating_add(1)
                    .saturating_add(i64::from(sbx));
                let _result = write!(output, "        ; -> {target}");
            }

            Opcode::JumpIf | Opcode::JumpIfNot => {
                let _result = write!(output, "R{reg_a}, {sbx}");
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_possible_wrap,
                    reason = "instruction offset is small; used for display only"
                )]
                let target = (offset as i64)
                    .saturating_add(1)
                    .saturating_add(i64::from(sbx));
                let _result = write!(output, "        ; -> {target}");
            }

            // Function calls
            Opcode::Call => {
                let _result = write!(output, "R{reg_a}, {reg_b}, {reg_c}");
                let _result = write!(output, "        ; {reg_b} args, {reg_c} results");
            }

            Opcode::TailCall => {
                let _result = write!(output, "R{reg_a}, {reg_b}");
                let _result = write!(output, "        ; {reg_b} args");
            }

            Opcode::Return => {
                let _result = write!(output, "R{reg_a}, {reg_b}");
                if reg_b == 0 {
                    let _result = write!(output, "        ; return all");
                } else {
                    let _result = write!(output, "        ; return {reg_b} values");
                }
            }
        }
    }

    /// Formats an RK operand (register or constant).
    fn format_rk(output: &mut String, rk: u8) {
        if rk_is_constant(rk) {
            let idx = rk_index(rk);
            let _result = write!(output, "K{idx}");
        } else {
            let _result = write!(output, "R{rk}");
        }
    }
}

impl Default for Chunk {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::{encode_abc, encode_abx, encode_asbx, rk_constant};

    // =========================================================================
    // Chunk Construction Tests
    // =========================================================================

    #[test]
    fn new_chunk_is_empty() {
        let chunk = Chunk::new();
        assert!(chunk.is_empty());
        assert_eq!(chunk.len(), 0);
        assert!(chunk.code().is_empty());
        assert!(chunk.constants().is_empty());
        assert_eq!(chunk.arity(), 0);
        assert_eq!(chunk.max_registers(), 0);
        assert!(chunk.name().is_empty());
    }

    #[test]
    fn chunk_with_name() {
        let chunk = Chunk::with_name(String::from("test_func"));
        assert_eq!(chunk.name(), "test_func");
    }

    #[test]
    fn set_chunk_properties() {
        let mut chunk = Chunk::new();
        chunk.set_name(String::from("my_func"));
        chunk.set_arity(3);
        chunk.set_max_registers(10);

        assert_eq!(chunk.name(), "my_func");
        assert_eq!(chunk.arity(), 3);
        assert_eq!(chunk.max_registers(), 10);
    }

    // =========================================================================
    // Instruction Emission Tests
    // =========================================================================

    #[test]
    fn emit_instructions() {
        let mut chunk = Chunk::new();

        let idx0 = chunk.emit(encode_abc(Opcode::LoadTrue, 0, 0, 0), Span::new(0, 4));
        let idx1 = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(4, 10));

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(chunk.len(), 2);
        assert!(!chunk.is_empty());
    }

    #[test]
    fn patch_instruction() {
        let mut chunk = Chunk::new();

        // Emit a placeholder jump
        let jump_idx = chunk.emit(encode_asbx(Opcode::Jump, 0, 0), Span::new(0, 4));

        // Emit some instructions
        let _idx1 = chunk.emit(encode_abc(Opcode::LoadTrue, 0, 0, 0), Span::new(4, 8));
        let _idx2 = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(8, 12));

        // Patch the jump to skip the LoadTrue
        chunk.patch(jump_idx, encode_asbx(Opcode::Jump, 0, 1));

        assert_eq!(decode_sbx(*chunk.code().get(0).unwrap()), 1);
    }

    // =========================================================================
    // Constant Pool Tests
    // =========================================================================

    #[test]
    fn add_and_get_constants() {
        let mut chunk = Chunk::new();

        let idx0 = chunk.add_constant(Constant::Integer(42)).unwrap();
        let idx1 = chunk
            .add_constant(Constant::String(String::from("hello")))
            .unwrap();
        let idx2 = chunk.add_constant(Constant::Nil).unwrap();
        let idx3 = chunk.add_constant(Constant::Bool(true)).unwrap();
        let idx4 = chunk.add_constant(Constant::Float(3.14)).unwrap();

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(idx2, 2);
        assert_eq!(idx3, 3);
        assert_eq!(idx4, 4);

        assert_eq!(chunk.get_constant(0), Some(&Constant::Integer(42)));
        assert_eq!(
            chunk.get_constant(1),
            Some(&Constant::String(String::from("hello")))
        );
        assert_eq!(chunk.get_constant(2), Some(&Constant::Nil));
        assert_eq!(chunk.get_constant(3), Some(&Constant::Bool(true)));
        assert_eq!(chunk.get_constant(4), Some(&Constant::Float(3.14)));
        assert_eq!(chunk.get_constant(5), None);
    }

    #[test]
    fn constant_display() {
        extern crate alloc;
        use alloc::format;

        assert_eq!(format!("{}", Constant::Nil), "nil");
        assert_eq!(format!("{}", Constant::Bool(true)), "true");
        assert_eq!(format!("{}", Constant::Bool(false)), "false");
        assert_eq!(format!("{}", Constant::Integer(42)), "42");
        assert_eq!(format!("{}", Constant::Float(3.14)), "3.14");
        assert_eq!(
            format!("{}", Constant::String(String::from("hello"))),
            "\"hello\""
        );
    }

    // =========================================================================
    // Span Tracking Tests
    // =========================================================================

    #[test]
    fn span_tracking() {
        let mut chunk = Chunk::new();

        let _idx0 = chunk.emit(encode_abc(Opcode::LoadTrue, 0, 0, 0), Span::new(0, 4));
        let _idx1 = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(10, 20));

        assert_eq!(chunk.span_at(0), Some(Span::new(0, 4)));
        assert_eq!(chunk.span_at(1), Some(Span::new(10, 20)));
        assert_eq!(chunk.span_at(2), None);

        let spans = chunk.spans();
        assert_eq!(spans.len(), 2);
    }

    // =========================================================================
    // Disassembler Tests
    // =========================================================================

    #[test]
    fn disassemble_empty_chunk() {
        let chunk = Chunk::new();
        let output = chunk.disassemble();

        assert!(output.contains("<anonymous>"));
        assert!(output.contains("arity: 0"));
        assert!(output.contains("max_registers: 0"));
    }

    #[test]
    fn disassemble_named_chunk() {
        let chunk = Chunk::with_name(String::from("main"));
        let output = chunk.disassemble();

        assert!(output.contains("== main =="));
    }

    #[test]
    fn disassemble_load_k() {
        let mut chunk = Chunk::new();
        let k_idx = chunk.add_constant(Constant::Integer(42)).unwrap();
        let _idx = chunk.emit(encode_abx(Opcode::LoadK, 0, k_idx), Span::new(0, 10));

        let output = chunk.disassemble();

        assert!(output.contains("LoadK"));
        assert!(output.contains("R0"));
        assert!(output.contains("K0"));
        assert!(output.contains("; 42"));
    }

    #[test]
    fn disassemble_arithmetic() {
        let mut chunk = Chunk::new();

        // Add R0, R1, K0 (where K0 = 10)
        let k_idx = chunk.add_constant(Constant::Integer(10)).unwrap();
        let rk_const = rk_constant(u8::try_from(k_idx).unwrap()).unwrap();
        let _idx = chunk.emit(encode_abc(Opcode::Add, 0, 1, rk_const), Span::new(0, 10));

        let output = chunk.disassemble();

        assert!(output.contains("Add"));
        assert!(output.contains("R0"));
        assert!(output.contains("R1"));
        assert!(output.contains("K0"));
    }

    #[test]
    fn disassemble_jump() {
        let mut chunk = Chunk::new();

        // Jump +5
        let _idx = chunk.emit(encode_asbx(Opcode::Jump, 0, 5), Span::new(0, 4));

        let output = chunk.disassemble();

        assert!(output.contains("Jump"));
        assert!(output.contains("5"));
        assert!(output.contains("; -> 6")); // offset 0 + 1 + 5 = 6
    }

    #[test]
    fn disassemble_call() {
        let mut chunk = Chunk::new();

        // Call R0, 2, 1 (call function in R0 with 2 args, expect 1 result)
        let _idx = chunk.emit(encode_abc(Opcode::Call, 0, 2, 1), Span::new(0, 10));

        let output = chunk.disassemble();

        assert!(output.contains("Call"));
        assert!(output.contains("R0"));
        assert!(output.contains("2 args"));
        assert!(output.contains("1 results"));
    }

    #[test]
    fn disassemble_return() {
        let mut chunk = Chunk::new();

        // Return R0, 1 (return 1 value starting at R0)
        let _idx = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(0, 6));

        let output = chunk.disassemble();

        assert!(output.contains("Return"));
        assert!(output.contains("return 1 values"));
    }

    #[test]
    fn disassemble_full_program() {
        // Compile (+ 1 2) conceptually
        let mut chunk = Chunk::with_name(String::from("main"));
        chunk.set_max_registers(3);

        let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

        // LoadK R0, K0  ; load 1
        let _idx = chunk.emit(encode_abx(Opcode::LoadK, 0, k0), Span::new(4, 5));
        // LoadK R1, K1  ; load 2
        let _idx = chunk.emit(encode_abx(Opcode::LoadK, 1, k1), Span::new(6, 7));
        // Add R0, R0, R1  ; R0 = 1 + 2
        let _idx = chunk.emit(encode_abc(Opcode::Add, 0, 0, 1), Span::new(1, 8));
        // Return R0, 1
        let _idx = chunk.emit(encode_abc(Opcode::Return, 0, 1, 0), Span::new(0, 9));

        let output = chunk.disassemble();

        // Verify structure
        assert!(output.contains("== main =="));
        assert!(output.contains("max_registers: 3"));
        assert!(output.contains("LoadK"));
        assert!(output.contains("Add"));
        assert!(output.contains("Return"));
        assert!(output.contains("Constants:"));
        assert!(output.contains("K0: 1"));
        assert!(output.contains("K1: 2"));
    }

    #[test]
    fn disassemble_single_instruction() {
        let chunk = Chunk::new();
        let instr = encode_abc(Opcode::Move, 5, 10, 0);

        let output = chunk.disassemble_instruction(0, instr);

        assert!(output.contains("0000"));
        assert!(output.contains("Move"));
        assert!(output.contains("R5"));
        assert!(output.contains("R10"));
    }
}
