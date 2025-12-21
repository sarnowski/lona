// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Bytecode chunk and constant pool structures.
//!
//! A `Chunk` represents a compiled function body or top-level expression.
//! It contains the bytecode instructions, a constant pool, and metadata
//! for debugging and execution.
//!
//! See `docs/architecture/register-based-vm.md` (from the repository root) for design rationale.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Display, Write as _};

use crate::opcode::{
    Opcode, decode_a, decode_b, decode_bx, decode_c, decode_op, decode_sbx, rk_index,
    rk_is_constant,
};
use crate::span::Span;
use crate::symbol;

#[cfg(test)]
mod tests;

/// Error when the constant pool exceeds its maximum size.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConstantPoolFullError {
    /// Source location where the error occurred.
    pub span: Span,
}

impl Display for ConstantPoolFullError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "too many constants in chunk (maximum 65535) at {}",
            self.span
        )
    }
}

/// Compile-time representation of a single function arity body.
///
/// Each body represents one arity variant of a (potentially multi-arity)
/// function. Contains the compiled bytecode, fixed parameter count, and
/// whether this arity accepts rest arguments.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FunctionBodyData {
    /// Compiled bytecode for this arity.
    pub chunk: Box<Chunk>,
    /// Number of fixed parameters.
    pub arity: u8,
    /// Whether this arity accepts rest arguments.
    pub has_rest: bool,
}

impl FunctionBodyData {
    /// Creates a new function body data.
    #[inline]
    #[must_use]
    pub const fn new(chunk: Box<Chunk>, arity: u8, has_rest: bool) -> Self {
        Self {
            chunk,
            arity,
            has_rest,
        }
    }
}

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
    /// An interned keyword identifier.
    Keyword(symbol::Id),
    /// A list of constants (for quoted lists).
    List(Vec<Self>),
    /// A vector of constants (for quoted vectors).
    Vector(Vec<Self>),
    /// A map of key-value pairs (for quoted maps and metadata).
    ///
    /// Each pair is (key, value). Keys and values can be any constant type.
    Map(Vec<(Self, Self)>),
    /// A compiled function.
    ///
    /// Contains all arity bodies for this function. Single-arity functions
    /// have exactly one body. Multi-arity functions have multiple bodies,
    /// each with different parameter counts.
    Function {
        /// All arity bodies for this function.
        bodies: Vec<FunctionBodyData>,
        /// Optional function name for debugging and error messages.
        name: Option<String>,
    },
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
            Self::Keyword(id) => write!(f, "kw#{}", id.as_u32()),
            Self::List(ref elements) => {
                write!(f, "(")?;
                for (idx, elem) in elements.iter().enumerate() {
                    if idx > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, ")")
            }
            Self::Vector(ref elements) => {
                write!(f, "[")?;
                for (idx, elem) in elements.iter().enumerate() {
                    if idx > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "]")
            }
            Self::Map(ref pairs) => {
                write!(f, "{{")?;
                for (idx, &(ref key, ref val)) in pairs.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{key} {val}")?;
                }
                write!(f, "}}")
            }
            Self::Function {
                ref bodies,
                ref name,
            } => {
                // Format: #<fn/1,2+> or #<fn name/1,2+>
                // The + indicates the last body (if variadic) has rest params
                if bodies.is_empty() {
                    match *name {
                        Some(ref func_name) => write!(f, "#<fn {func_name}/?>"),
                        None => write!(f, "#<fn/?>"),
                    }
                } else if let &[ref body] = bodies.as_slice() {
                    // Single arity - simpler format
                    let rest_indicator = if body.has_rest { "+" } else { "" };
                    match *name {
                        Some(ref func_name) => {
                            write!(f, "#<fn {func_name}/{}{rest_indicator}>", body.arity)
                        }
                        None => write!(f, "#<fn/{}{rest_indicator}>", body.arity),
                    }
                } else {
                    // Multi-arity - list all arities
                    match *name {
                        Some(ref func_name) => write!(f, "#<fn {func_name}/")?,
                        None => write!(f, "#<fn/")?,
                    }
                    for (idx, body) in bodies.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ",")?;
                        }
                        write!(f, "{}", body.arity)?;
                        if body.has_rest {
                            write!(f, "+")?;
                        }
                    }
                    write!(f, ">")
                }
            }
        }
    }
}

/// A compiled bytecode chunk.
///
/// Represents a function body or top-level expression. Contains all the
/// information needed for the VM to execute the code.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Chunk {
    /// Bytecode instructions.
    code: Vec<u32>,
    /// Constant pool.
    constants: Vec<Constant>,
    /// Maximum registers used by this chunk.
    max_registers: u8,
    /// Number of fixed parameters (0 for top-level code).
    arity: u8,
    /// Whether this chunk uses rest parameters.
    has_rest: bool,
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
            has_rest: false,
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
            has_rest: false,
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

    /// Sets the number of fixed parameters.
    #[inline]
    pub const fn set_arity(&mut self, arity: u8) {
        self.arity = arity;
    }

    /// Returns whether this chunk uses rest parameters.
    #[inline]
    #[must_use]
    pub const fn has_rest(&self) -> bool {
        self.has_rest
    }

    /// Sets whether this chunk uses rest parameters.
    #[inline]
    pub const fn set_has_rest(&mut self, has_rest: bool) {
        self.has_rest = has_rest;
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
    /// error reporting, use `add_constant_at` instead.
    ///
    /// # Errors
    ///
    /// Returns `ConstantPoolFullError` if the constant pool is full (> 65535).
    #[inline]
    pub fn add_constant(&mut self, constant: Constant) -> Result<u16, ConstantPoolFullError> {
        self.add_constant_at(constant, Span::new(0_usize, 0_usize))
    }

    /// Adds a constant with source span for error reporting.
    ///
    /// # Errors
    ///
    /// Returns `ConstantPoolFullError` with the span if the pool is full.
    #[inline]
    pub fn add_constant_at(
        &mut self,
        constant: Constant,
        span: Span,
    ) -> Result<u16, ConstantPoolFullError> {
        let index = self.constants.len();
        let index_u16 = u16::try_from(index).map_err(|_err| ConstantPoolFullError { span })?;
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

    // =========================================================================
    // Disassembler Methods
    // =========================================================================

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
        let rest_info = if self.has_rest { " (variadic)" } else { "" };
        let _result = writeln!(
            output,
            "arity: {}{rest_info}, max_registers: {}",
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
            for (idx, constant) in self.constants.iter().enumerate() {
                let _result = writeln!(output, "  K{idx}: {constant}");
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

            // iABx format - regular global access
            Opcode::LoadK | Opcode::GetGlobal | Opcode::SetGlobal => {
                let _result = write!(output, "R{reg_a}, K{bx}");
                if let Some(constant) = self.get_constant(bx) {
                    let _result = write!(output, "        ; {constant}");
                }
            }

            // iABx format - var and metadata operations
            Opcode::SetGlobalMeta => {
                let _result = write!(output, "R{reg_a}, K{bx}");
                if let Some(constant) = self.get_constant(bx) {
                    let _result = write!(output, "        ; meta for {constant}");
                }
            }

            Opcode::GetGlobalVar => {
                let _result = write!(output, "R{reg_a}, K{bx}");
                if let Some(constant) = self.get_constant(bx) {
                    let _result = write!(output, "        ; var {constant}");
                }
            }

            // iAsBx format (jumps)
            Opcode::Jump => {
                let _result = write!(output, "{sbx}");
                // Show target address
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_possible_wrap,
                    reason = "[approved] instruction offset is small; used for display only"
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
                    reason = "[approved] instruction offset is small; used for display only"
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
