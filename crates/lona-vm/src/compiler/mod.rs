// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Compiler from Lonala S-expressions to bytecode.
//!
//! The compiler transforms parsed `Value` trees into executable `Chunk`s.
//! It uses a simple single-pass algorithm suitable for expression evaluation.
//!
//! ## Calling Convention
//!
//! - Arguments are compiled into X1, X2, X3, ...
//! - The result of an expression is always in X0
//! - For intrinsics: `INTRINSIC id, argc` reads X1..X(argc), writes X0

#[cfg(test)]
mod compiler_test;

#[cfg(any(test, feature = "std"))]
use std::vec::Vec;

#[cfg(not(any(test, feature = "std")))]
use alloc::vec::Vec;

use crate::bytecode::{BX_MASK, Chunk, MAX_SIGNED_BX, MIN_SIGNED_BX, encode_abc, encode_abx, op};
use crate::heap::Heap;
use crate::intrinsics::lookup_intrinsic;
use crate::platform::MemorySpace;
use crate::value::Value;

/// Maximum number of arguments for an intrinsic call.
const MAX_ARGS: u8 = 254; // X1..X254, X0 reserved for result

/// First register available for temporary storage during compilation.
/// Registers 128-255 are used as temps, giving 128 temp slots.
const TEMP_REG_BASE: u8 = 128;

/// Compilation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileError {
    /// Unbound symbol (not a known intrinsic).
    UnboundSymbol,
    /// Invalid syntax in expression.
    InvalidSyntax,
    /// Too many arguments in a call.
    TooManyArguments,
    /// Integer too large for inline encoding.
    IntegerTooLarge,
    /// Constant pool overflow.
    ConstantPoolFull,
    /// Expression too complex (register overflow).
    ExpressionTooComplex,
}

/// Compiler state for a single expression.
pub struct Compiler<'a, M: MemorySpace> {
    /// The bytecode chunk being built.
    chunk: Chunk,
    /// Reference to the heap (for reading strings/symbols).
    heap: &'a Heap,
    /// Reference to the memory space.
    mem: &'a M,
}

impl<'a, M: MemorySpace> Compiler<'a, M> {
    /// Create a new compiler.
    #[must_use]
    pub const fn new(heap: &'a Heap, mem: &'a M) -> Self {
        Self {
            chunk: Chunk::new(),
            heap,
            mem,
        }
    }

    /// Compile an expression and emit HALT.
    ///
    /// The result will be in X0.
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails.
    pub fn compile(mut self, expr: Value) -> Result<Chunk, CompileError> {
        // Compile the expression, result in X0
        // Start temp registers at TEMP_REG_BASE (128)
        self.compile_expr(expr, 0, TEMP_REG_BASE)?;

        // Emit HALT to stop execution
        self.chunk.emit(encode_abx(op::HALT, 0, 0));

        Ok(self.chunk)
    }

    /// Compile an expression, placing the result in the target register.
    ///
    /// `temp_base` is the first available temp register. For nested calls,
    /// this is bumped up to avoid register conflicts.
    ///
    /// Returns the next available temp register after compilation.
    fn compile_expr(&mut self, expr: Value, target: u8, temp_base: u8) -> Result<u8, CompileError> {
        match expr {
            Value::Nil => {
                self.chunk.emit(encode_abx(op::LOADNIL, target, 0));
                Ok(temp_base)
            }
            Value::Bool(b) => {
                self.chunk
                    .emit(encode_abx(op::LOADBOOL, target, u32::from(b)));
                Ok(temp_base)
            }
            Value::Int(n) => {
                self.compile_int(n, target)?;
                Ok(temp_base)
            }
            Value::String(_) => {
                self.compile_constant(expr, target)?;
                Ok(temp_base)
            }
            Value::Symbol(_) => {
                // Bare symbols are not supported yet (no variables)
                Err(CompileError::UnboundSymbol)
            }
            Value::Pair(_) => self.compile_list(expr, target, temp_base),
        }
    }

    /// Compile an integer literal.
    #[expect(
        clippy::cast_sign_loss,
        reason = "intentional two's complement encoding for signed immediate"
    )]
    fn compile_int(&mut self, n: i64, target: u8) -> Result<(), CompileError> {
        // Check if it fits in 18-bit signed immediate
        if n >= i64::from(MIN_SIGNED_BX) && n <= i64::from(MAX_SIGNED_BX) {
            // Encode as two's complement in 18 bits
            // Cast through i32 first to get proper sign extension, then mask
            let bx = (n as i32 as u32) & BX_MASK;
            self.chunk.emit(encode_abx(op::LOADINT, target, bx));
            Ok(())
        } else {
            // Too large for inline, use constant pool
            self.compile_constant(Value::int(n), target)
        }
    }

    /// Compile a constant (load from constant pool).
    fn compile_constant(&mut self, value: Value, target: u8) -> Result<(), CompileError> {
        let idx = self
            .chunk
            .add_constant(value)
            .ok_or(CompileError::ConstantPoolFull)?;
        self.chunk.emit(encode_abx(op::LOADK, target, idx));
        Ok(())
    }

    /// Compile a list expression (special form or intrinsic call).
    fn compile_list(&mut self, list: Value, target: u8, temp_base: u8) -> Result<u8, CompileError> {
        let pair = self
            .heap
            .read_pair(self.mem, list)
            .ok_or(CompileError::InvalidSyntax)?;

        // First element should be a symbol (the operator)
        let Value::Symbol(_) = pair.first else {
            return Err(CompileError::InvalidSyntax);
        };

        // Look up the symbol name
        let name = self
            .heap
            .read_string(self.mem, pair.first)
            .ok_or(CompileError::InvalidSyntax)?;

        // Check for special forms first
        if name == "quote" {
            return self.compile_quote(pair.rest, target, temp_base);
        }

        // Check if it's a known intrinsic
        let intrinsic_id = lookup_intrinsic(name).ok_or(CompileError::UnboundSymbol)?;

        // Compile the intrinsic call
        self.compile_intrinsic_call(intrinsic_id, pair.rest, target, temp_base)
    }

    /// Compile the `quote` special form.
    ///
    /// `(quote expr)` returns `expr` unevaluated.
    fn compile_quote(
        &mut self,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Get the single argument
        let pair = self
            .heap
            .read_pair(self.mem, arg_list)
            .ok_or(CompileError::InvalidSyntax)?;

        // quote takes exactly one argument
        if !pair.rest.is_nil() {
            return Err(CompileError::InvalidSyntax);
        }

        // Load the quoted expression as a constant (unevaluated)
        self.compile_constant(pair.first, target)?;
        Ok(temp_base)
    }

    /// Compile an intrinsic call.
    ///
    /// Arguments are first compiled to temp registers, then moved to X1..Xn.
    /// This prevents nested calls from clobbering already-computed arguments.
    /// The INTRINSIC instruction puts the result in X0.
    /// If target != 0, we emit a MOVE to copy X0 to target.
    ///
    /// Returns the next available temp register after compilation.
    fn compile_intrinsic_call(
        &mut self,
        intrinsic_id: u8,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // First, collect all arguments while counting
        let mut args: Vec<Value> = Vec::new();
        let mut arg_count: u8 = 0;
        let mut current = arg_list;

        while !current.is_nil() {
            let pair = self
                .heap
                .read_pair(self.mem, current)
                .ok_or(CompileError::InvalidSyntax)?;

            arg_count = arg_count
                .checked_add(1)
                .ok_or(CompileError::TooManyArguments)?;
            if arg_count > MAX_ARGS {
                return Err(CompileError::TooManyArguments);
            }

            args.push(pair.first);
            current = pair.rest;
        }

        // Handle zero-arg case
        if arg_count == 0 {
            self.chunk
                .emit(encode_abc(op::INTRINSIC, intrinsic_id, 0, 0));
            if target != 0 {
                self.chunk.emit(encode_abc(op::MOVE, target, 0, 0));
            }
            return Ok(temp_base);
        }

        // Allocate temp registers for our args: temp_base..temp_base+argc-1
        // Nested calls will use temps starting at temp_base+argc
        let next_temp = temp_base
            .checked_add(arg_count)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // Compile each argument to its temp register
        let mut current_next_temp = next_temp;
        for (i, arg) in args.iter().enumerate() {
            let temp_reg = temp_base
                .checked_add(i as u8)
                .ok_or(CompileError::ExpressionTooComplex)?;
            current_next_temp = self.compile_expr(*arg, temp_reg, current_next_temp)?;
        }

        // Move temps to argument positions X1..Xn
        for i in 0..arg_count {
            self.chunk
                .emit(encode_abc(op::MOVE, i + 1, u16::from(temp_base + i), 0));
        }

        // Emit INTRINSIC instruction
        // Format: INTRINSIC id, arg_count (id in A field, arg_count in B field)
        self.chunk.emit(encode_abc(
            op::INTRINSIC,
            intrinsic_id,
            u16::from(arg_count),
            0,
        ));

        // If target != 0, move X0 to target
        if target != 0 {
            self.chunk.emit(encode_abc(op::MOVE, target, 0, 0));
        }

        Ok(current_next_temp)
    }
}

/// Convenience function to compile an expression.
///
/// # Errors
///
/// Returns an error if compilation fails.
pub fn compile<M: MemorySpace>(expr: Value, heap: &Heap, mem: &M) -> Result<Chunk, CompileError> {
    Compiler::new(heap, mem).compile(expr)
}

/// Debug helper: disassemble a chunk to a string.
#[cfg(any(test, feature = "std"))]
#[must_use]
pub fn disassemble(chunk: &Chunk) -> std::string::String {
    use crate::bytecode::{decode_a, decode_b, decode_bx, decode_opcode, decode_sbx};
    use std::fmt::Write;

    let mut out = std::string::String::new();

    for (i, &instr) in chunk.code.iter().enumerate() {
        let opcode = decode_opcode(instr);
        let a = decode_a(instr);
        let bx = decode_bx(instr);

        let _ = write!(out, "{i:04}: ");

        match opcode {
            op::LOADNIL => {
                let _ = writeln!(out, "LOADNIL   X{a}");
            }
            op::LOADBOOL => {
                let b = if bx != 0 { "true" } else { "false" };
                let _ = writeln!(out, "LOADBOOL  X{a}, {b}");
            }
            op::LOADINT => {
                let sbx = decode_sbx(instr);
                let _ = writeln!(out, "LOADINT   X{a}, {sbx}");
            }
            op::LOADK => {
                let _ = writeln!(out, "LOADK     X{a}, K{bx}");
            }
            op::MOVE => {
                let b = decode_b(instr);
                let _ = writeln!(out, "MOVE      X{a}, X{b}");
            }
            op::INTRINSIC => {
                let b = decode_b(instr);
                let name = crate::intrinsics::intrinsic_name(a).unwrap_or("?");
                let _ = writeln!(out, "INTRINSIC {name}({a}), {b} args");
            }
            op::RETURN => {
                let _ = writeln!(out, "RETURN");
            }
            op::HALT => {
                let _ = writeln!(out, "HALT");
            }
            _ => {
                let _ = writeln!(out, "??? opcode={opcode}");
            }
        }
    }

    if !chunk.constants.is_empty() {
        let _ = writeln!(out, "\nConstants:");
        for (i, c) in chunk.constants.iter().enumerate() {
            let _ = writeln!(out, "  K{i}: {c:?}");
        }
    }

    out
}
