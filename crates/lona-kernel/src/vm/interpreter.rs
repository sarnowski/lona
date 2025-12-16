// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Virtual machine interpreter for Lonala bytecode.

use alloc::vec;
use alloc::vec::Vec;

use lona_core::symbol::{self, Interner};
use lona_core::value::Value;
use lonala_compiler::opcode::{
    Opcode, decode_a, decode_b, decode_bx, decode_c, decode_op, decode_opcode_byte, decode_sbx,
    rk_index, rk_is_constant,
};
use lonala_compiler::{Chunk, Constant};

use super::error::Error;
use super::frame::Frame;
use super::globals::Globals;
use super::natives::{NativeFn, Registry as NativeRegistry};
use super::primitives::{PrintCallback, format_print_args};

/// Default register file size.
const DEFAULT_REGISTER_COUNT: usize = 256;

/// The Lonala virtual machine.
///
/// Executes compiled bytecode from `Chunk` objects. Register-based design
/// similar to Lua, with up to 256 registers per frame.
pub struct Vm<'interner> {
    /// Register file for storing values during execution.
    registers: Vec<Value>,
    /// Global variable storage.
    globals: Globals,
    /// Symbol interner for resolving symbol names.
    interner: &'interner Interner,
    /// Registry of native functions.
    natives: NativeRegistry,
    /// Callback for print output.
    print_callback: Option<PrintCallback>,
    /// Symbol ID for "print" - cached for fast lookup.
    print_symbol: Option<symbol::Id>,
}

impl<'interner> Vm<'interner> {
    /// Creates a new virtual machine.
    #[inline]
    #[must_use]
    pub fn new(interner: &'interner Interner) -> Self {
        // Look up "print" symbol if it exists in the interner
        let print_symbol = interner.get("print");

        Self {
            registers: vec![Value::Nil; DEFAULT_REGISTER_COUNT],
            globals: Globals::new(),
            interner,
            natives: NativeRegistry::new(),
            print_callback: None,
            print_symbol,
        }
    }

    /// Registers a native function for a symbol.
    ///
    /// The symbol must already be interned in the interner passed to [`Vm::new`].
    #[inline]
    pub fn register_native(&mut self, symbol: symbol::Id, func: NativeFn) {
        self.natives.register(symbol, func);
    }

    /// Sets the print callback for output.
    ///
    /// When `print` is called, the formatted output is passed to this callback.
    #[inline]
    pub fn set_print_callback(&mut self, callback: PrintCallback) {
        self.print_callback = Some(callback);
    }

    /// Updates the print symbol cache.
    ///
    /// Call this after interning "print" if you want to use the built-in print.
    #[inline]
    pub const fn update_print_symbol(&mut self, symbol: symbol::Id) {
        self.print_symbol = Some(symbol);
    }

    /// Sets a global variable.
    ///
    /// Used to register built-in functions as globals.
    #[inline]
    pub fn set_global(&mut self, symbol: symbol::Id, value: Value) {
        self.globals.set(symbol, value);
    }

    /// Returns a reference to the symbol interner.
    #[inline]
    #[must_use]
    pub const fn interner(&self) -> &'interner Interner {
        self.interner
    }

    /// Returns a mutable reference to the global variables.
    ///
    /// Use this to register native functions or set initial global values.
    #[inline]
    #[must_use]
    pub const fn globals_mut(&mut self) -> &mut Globals {
        &mut self.globals
    }

    /// Returns a reference to the global variables.
    #[inline]
    #[must_use]
    pub const fn globals(&self) -> &Globals {
        &self.globals
    }

    /// Executes a chunk of bytecode and returns the result.
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails due to:
    /// - Invalid opcodes
    /// - Type errors in operations
    /// - Undefined global variables
    /// - Division by zero
    /// - Stack overflow
    #[inline]
    pub fn execute(&mut self, chunk: &Chunk) -> Result<Value, Error> {
        // Reset registers to nil
        for reg in &mut self.registers {
            *reg = Value::Nil;
        }

        let mut frame = Frame::new(chunk, 0);
        self.run(&mut frame)
    }

    /// Main execution loop.
    fn run(&mut self, frame: &mut Frame<'_>) -> Result<Value, Error> {
        loop {
            let Some(instruction) = frame.fetch() else {
                // End of bytecode - return nil by default
                return Ok(Value::Nil);
            };

            let Some(opcode) = decode_op(instruction) else {
                return Err(Error::InvalidOpcode {
                    byte: decode_opcode_byte(instruction),
                    pc: frame.pc().saturating_sub(1),
                    span: frame.current_span(),
                });
            };

            match opcode {
                // Data Movement
                Opcode::Move => self.op_move(instruction, frame)?,
                Opcode::LoadK => self.op_load_k(instruction, frame)?,
                Opcode::LoadNil => self.op_load_nil(instruction, frame),
                Opcode::LoadTrue => self.op_load_true(instruction, frame)?,
                Opcode::LoadFalse => self.op_load_false(instruction, frame)?,

                // Globals
                Opcode::GetGlobal => self.op_get_global(instruction, frame)?,
                Opcode::SetGlobal => self.op_set_global(instruction, frame)?,

                // Arithmetic
                Opcode::Add => self.op_add(instruction, frame)?,
                Opcode::Sub => self.op_sub(instruction, frame)?,
                Opcode::Mul => self.op_mul(instruction, frame)?,
                Opcode::Div => self.op_div(instruction, frame)?,
                Opcode::Mod => self.op_mod(instruction, frame)?,
                Opcode::Neg => self.op_neg(instruction, frame)?,

                // Comparison
                Opcode::Eq => self.op_eq(instruction, frame)?,
                Opcode::Lt => self.op_lt(instruction, frame)?,
                Opcode::Le => self.op_le(instruction, frame)?,
                Opcode::Gt => self.op_gt(instruction, frame)?,
                Opcode::Ge => self.op_ge(instruction, frame)?,
                Opcode::Not => self.op_not(instruction, frame)?,

                // Control Flow
                Opcode::Jump => Self::op_jump(instruction, frame),
                Opcode::JumpIf => self.op_jump_if(instruction, frame)?,
                Opcode::JumpIfNot => self.op_jump_if_not(instruction, frame)?,

                // Function Calls
                Opcode::Call => {
                    self.op_call(instruction, frame)?;
                }
                Opcode::TailCall => {
                    // TailCall will be fully implemented in Phase 4
                    // For now, treat as regular call
                    self.op_call(instruction, frame)?;
                }
                Opcode::Return => {
                    let dest = decode_a(instruction);
                    let count = decode_b(instruction);
                    return self.op_return(dest, count, frame);
                }

                // Handle future Opcode variants (Opcode is #[non_exhaustive])
                _ => {
                    return Err(Error::InvalidOpcode {
                        byte: decode_opcode_byte(instruction),
                        pc: frame.pc().saturating_sub(1),
                        span: frame.current_span(),
                    });
                }
            }
        }
    }

    // =========================================================================
    // Data Movement Operations
    // =========================================================================

    /// `Move`: `R[A] = R[B]`
    fn op_move(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let src = decode_b(instruction);
        let value = self.get_register(src, frame)?;
        self.set_register(dest, value, frame)?;
        Ok(())
    }

    /// `LoadK`: `R[A] = K[Bx]`
    fn op_load_k(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let const_idx = decode_bx(instruction);
        let value = Self::constant_to_value(frame.chunk(), const_idx, frame)?;
        self.set_register(dest, value, frame)?;
        Ok(())
    }

    /// `LoadNil`: `R[A]..R[A+B] = nil`
    fn op_load_nil(&mut self, instruction: u32, frame: &Frame<'_>) {
        let start = decode_a(instruction);
        let count = decode_b(instruction);
        let base = frame.base();

        for offset in 0_u16..=u16::from(count) {
            let reg_idx = base
                .checked_add(usize::from(start))
                .and_then(|x| x.checked_add(usize::from(offset)));
            if let Some(idx) = reg_idx
                && let Some(reg) = self.registers.get_mut(idx)
            {
                *reg = Value::Nil;
            }
        }
    }

    /// `LoadTrue`: `R[A] = true`
    fn op_load_true(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        self.set_register(dest, Value::Bool(true), frame)?;
        Ok(())
    }

    /// `LoadFalse`: `R[A] = false`
    fn op_load_false(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        self.set_register(dest, Value::Bool(false), frame)?;
        Ok(())
    }

    // =========================================================================
    // Global Variable Operations
    // =========================================================================

    /// `GetGlobal`: `R[A] = globals[K[Bx]]`
    fn op_get_global(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;
        let value = self
            .globals
            .get(symbol)
            .ok_or_else(|| Error::UndefinedGlobal {
                symbol,
                span: frame.current_span(),
            })?;
        self.set_register(dest, value, frame)?;
        Ok(())
    }

    /// `SetGlobal`: `globals[K[Bx]] = R[A]`
    fn op_set_global(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let src = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;
        let value = self.get_register(src, frame)?;
        self.globals.set(symbol, value);
        Ok(())
    }

    // =========================================================================
    // Arithmetic Operations
    // =========================================================================

    /// `Add`: `R[A] = RK[B] + RK[C]`
    fn op_add(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = Self::numeric_add(left, right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Sub`: `R[A] = RK[B] - RK[C]`
    fn op_sub(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = Self::numeric_sub(left, right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Mul`: `R[A] = RK[B] * RK[C]`
    fn op_mul(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = Self::numeric_mul(left, right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Div`: `R[A] = RK[B] / RK[C]`
    fn op_div(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = Self::numeric_div(left, right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Mod`: `R[A] = RK[B] % RK[C]`
    fn op_mod(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = Self::numeric_mod(left, right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Neg`: `R[A] = -R[B]`
    fn op_neg(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let src = decode_b(instruction);

        let operand = self.get_register(src, frame)?;
        let result = match operand {
            Value::Integer(int_val) => Value::Integer(int_val.saturating_neg()),
            Value::Float(float_val) => Value::Float(-float_val),
            Value::Nil | Value::Bool(_) | Value::Symbol(_) | _ => {
                return Err(Error::TypeError {
                    expected: "number",
                    got: value_type_name(operand),
                    span: frame.current_span(),
                });
            }
        };
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    // =========================================================================
    // Comparison Operations
    // =========================================================================

    /// `Eq`: `R[A] = RK[B] == RK[C]`
    fn op_eq(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = Value::Bool(values_equal(left, right));
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Lt`: `R[A] = RK[B] < RK[C]`
    fn op_lt(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = Self::numeric_compare(left, right, frame, |lv, rv| lv < rv, |lv, rv| lv < rv)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Le`: `R[A] = RK[B] <= RK[C]`
    fn op_le(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result =
            Self::numeric_compare(left, right, frame, |lv, rv| lv <= rv, |lv, rv| lv <= rv)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Gt`: `R[A] = RK[B] > RK[C]`
    fn op_gt(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = Self::numeric_compare(left, right, frame, |lv, rv| lv > rv, |lv, rv| lv > rv)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Ge`: `R[A] = RK[B] >= RK[C]`
    fn op_ge(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result =
            Self::numeric_compare(left, right, frame, |lv, rv| lv >= rv, |lv, rv| lv >= rv)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Not`: `R[A] = not R[B]`
    fn op_not(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let src = decode_b(instruction);

        let operand = self.get_register(src, frame)?;
        let result = Value::Bool(!operand.is_truthy());
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    // =========================================================================
    // Control Flow Operations
    // =========================================================================

    /// `Jump`: `PC += sBx`
    const fn op_jump(instruction: u32, frame: &mut Frame<'_>) {
        let sbx = decode_sbx(instruction);
        frame.jump(sbx);
    }

    /// `JumpIf`: `if R[A] then PC += sBx`
    fn op_jump_if(&self, instruction: u32, frame: &mut Frame<'_>) -> Result<(), Error> {
        let cond_reg = decode_a(instruction);
        let offset = decode_sbx(instruction);

        let condition = self.get_register(cond_reg, frame)?;
        if condition.is_truthy() {
            frame.jump(offset);
        }
        Ok(())
    }

    /// `JumpIfNot`: `if not R[A] then PC += sBx`
    fn op_jump_if_not(&self, instruction: u32, frame: &mut Frame<'_>) -> Result<(), Error> {
        let cond_reg = decode_a(instruction);
        let offset = decode_sbx(instruction);

        let condition = self.get_register(cond_reg, frame)?;
        if !condition.is_truthy() {
            frame.jump(offset);
        }
        Ok(())
    }

    // =========================================================================
    // Function Call Operations
    // =========================================================================

    /// `Call`: `R[A] = R[A](R[A+1], ..., R[A+B])`
    ///
    /// Calls the function in R[A] with B arguments from R[A+1]..R[A+B].
    /// The result is stored back in R[A].
    fn op_call(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let base = decode_a(instruction);
        let argc = decode_b(instruction);
        let _result_count = decode_c(instruction);

        // Get function value from R[base]
        let func_value = self.get_register(base, frame)?;

        // Check if it's a symbol (global function reference)
        let Value::Symbol(symbol) = func_value else {
            return Err(Error::NotCallable {
                span: frame.current_span(),
            });
        };

        // Check if this is the built-in print function
        if self.print_symbol == Some(symbol) {
            return self.handle_print(base, argc, frame);
        }

        // Look up native function
        let native_fn = self
            .natives
            .get(symbol)
            .ok_or_else(|| Error::UndefinedFunction {
                symbol,
                span: frame.current_span(),
            })?;

        // Collect arguments from R[base+1] .. R[base+argc]
        let arguments = self.collect_args(base, argc, frame)?;

        // Call native function
        let result = native_fn(&arguments, self.interner).map_err(|error| Error::Native {
            error,
            span: frame.current_span(),
        })?;

        // Store result in R[base]
        self.set_register(base, result, frame)?;
        Ok(())
    }

    /// Handles the built-in print function.
    fn handle_print(&mut self, base: u8, arg_count: u8, frame: &Frame<'_>) -> Result<(), Error> {
        // Collect arguments
        let arguments = self.collect_args(base, arg_count, frame)?;

        // Format the output
        let output = format_print_args(&arguments, self.interner);

        // Call the print callback if set
        if let Some(callback) = self.print_callback {
            callback(&output);
        }

        // Store nil as result in R[base]
        self.set_register(base, Value::Nil, frame)?;
        Ok(())
    }

    /// Collects function arguments from registers.
    fn collect_args(
        &self,
        base: u8,
        arg_count: u8,
        frame: &Frame<'_>,
    ) -> Result<Vec<Value>, Error> {
        let mut arguments = Vec::with_capacity(usize::from(arg_count));

        for offset in 0_u8..arg_count {
            let reg_idx = base
                .checked_add(1)
                .and_then(|base_plus_one| base_plus_one.checked_add(offset));
            let reg = reg_idx.ok_or_else(|| Error::InvalidRegister {
                index: base,
                span: frame.current_span(),
            })?;
            arguments.push(self.get_register(reg, frame)?);
        }

        Ok(arguments)
    }

    // =========================================================================
    // Return Operation
    // =========================================================================

    /// `Return`: `return R[A]..R[A+B-1]`
    fn op_return(&self, start_reg: u8, count: u8, frame: &Frame<'_>) -> Result<Value, Error> {
        // For now, just return the first value (single return)
        // Full multi-value returns will come in Phase 4
        if count == 0 {
            // Return all values - for now just return nil
            Ok(Value::Nil)
        } else {
            // Return count values starting at R[start_reg]
            self.get_register(start_reg, frame)
        }
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Gets a value from a register.
    fn get_register(&self, index: u8, frame: &Frame<'_>) -> Result<Value, Error> {
        let absolute_index = frame.base().saturating_add(usize::from(index));
        self.registers
            .get(absolute_index)
            .copied()
            .ok_or_else(|| Error::InvalidRegister {
                index,
                span: frame.current_span(),
            })
    }

    /// Sets a value in a register.
    fn set_register(&mut self, index: u8, value: Value, frame: &Frame<'_>) -> Result<(), Error> {
        let absolute_index = frame.base().saturating_add(usize::from(index));
        let reg = self
            .registers
            .get_mut(absolute_index)
            .ok_or_else(|| Error::InvalidRegister {
                index,
                span: frame.current_span(),
            })?;
        *reg = value;
        Ok(())
    }

    /// Gets a value from an RK field (register or constant).
    fn get_rk(&self, rk: u8, frame: &Frame<'_>) -> Result<Value, Error> {
        if rk_is_constant(rk) {
            let const_index = u16::from(rk_index(rk));
            Self::constant_to_value(frame.chunk(), const_index, frame)
        } else {
            self.get_register(rk, frame)
        }
    }

    /// Converts a constant pool entry to a value.
    fn constant_to_value(chunk: &Chunk, index: u16, frame: &Frame<'_>) -> Result<Value, Error> {
        let constant = chunk
            .get_constant(index)
            .ok_or_else(|| Error::InvalidConstant {
                index,
                span: frame.current_span(),
            })?;

        Ok(match *constant {
            Constant::Bool(val) => Value::Bool(val),
            Constant::Integer(num) => Value::Integer(num),
            Constant::Float(num) => Value::Float(num),
            Constant::Symbol(id) => Value::Symbol(id),
            // Constant::Nil, Constant::String, and future variants all become nil for now.
            // String values will be fully supported in Phase 3.2.
            Constant::Nil | Constant::String(_) | _ => Value::Nil,
        })
    }

    /// Gets a symbol ID from a constant pool entry.
    fn get_symbol_from_constant(
        chunk: &Chunk,
        index: u16,
        frame: &Frame<'_>,
    ) -> Result<symbol::Id, Error> {
        let constant = chunk
            .get_constant(index)
            .ok_or_else(|| Error::InvalidConstant {
                index,
                span: frame.current_span(),
            })?;

        match *constant {
            Constant::Symbol(id) => Ok(id),
            Constant::Nil
            | Constant::Bool(_)
            | Constant::Integer(_)
            | Constant::Float(_)
            | Constant::String(_)
            | _ => Err(Error::TypeError {
                expected: "symbol",
                got: constant_type_name(constant),
                span: frame.current_span(),
            }),
        }
    }

    // =========================================================================
    // Numeric Operations
    // =========================================================================

    /// Performs addition with type promotion.
    fn numeric_add(left: Value, right: Value, frame: &Frame<'_>) -> Result<Value, Error> {
        match (left, right) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                Ok(Value::Integer(lhs.saturating_add(rhs)))
            }
            (Value::Float(lhs), Value::Float(rhs)) => Ok(Value::Float(lhs + rhs)),
            (Value::Integer(lhs), Value::Float(rhs)) => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "i64 to f64 may lose precision for very large integers, but this is acceptable for numeric operations"
                )]
                let lhs_float = lhs as f64;
                Ok(Value::Float(lhs_float + rhs))
            }
            (Value::Float(lhs), Value::Integer(rhs)) => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "i64 to f64 may lose precision for very large integers, but this is acceptable for numeric operations"
                )]
                let rhs_float = rhs as f64;
                Ok(Value::Float(lhs + rhs_float))
            }
            _ => Err(Error::TypeError {
                expected: "number",
                got: binary_type_description(left, right),
                span: frame.current_span(),
            }),
        }
    }

    /// Performs subtraction with type promotion.
    fn numeric_sub(left: Value, right: Value, frame: &Frame<'_>) -> Result<Value, Error> {
        match (left, right) {
            (Value::Integer(lhs), Value::Integer(rhs)) => {
                Ok(Value::Integer(lhs.saturating_sub(rhs)))
            }
            (Value::Float(lhs), Value::Float(rhs)) => Ok(Value::Float(lhs - rhs)),
            (Value::Integer(lhs), Value::Float(rhs)) => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "i64 to f64 may lose precision for very large integers"
                )]
                let lhs_float = lhs as f64;
                Ok(Value::Float(lhs_float - rhs))
            }
            (Value::Float(lhs), Value::Integer(rhs)) => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "i64 to f64 may lose precision for very large integers"
                )]
                let rhs_float = rhs as f64;
                Ok(Value::Float(lhs - rhs_float))
            }
            _ => Err(Error::TypeError {
                expected: "number",
                got: binary_type_description(left, right),
                span: frame.current_span(),
            }),
        }
    }

    /// Performs multiplication with type promotion.
    fn numeric_mul(left: Value, right: Value, frame: &Frame<'_>) -> Result<Value, Error> {
        match (left, right) {
            (Value::Integer(left_int), Value::Integer(right_int)) => {
                Ok(Value::Integer(left_int.saturating_mul(right_int)))
            }
            (Value::Float(left_float), Value::Float(right_float)) => {
                Ok(Value::Float(left_float * right_float))
            }
            (Value::Integer(left_int), Value::Float(right_float)) => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "i64 to f64 may lose precision for very large integers"
                )]
                let left_as_float = left_int as f64;
                Ok(Value::Float(left_as_float * right_float))
            }
            (Value::Float(left_float), Value::Integer(right_int)) => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "i64 to f64 may lose precision for very large integers"
                )]
                let right_as_float = right_int as f64;
                Ok(Value::Float(left_float * right_as_float))
            }
            _ => Err(Error::TypeError {
                expected: "number",
                got: binary_type_description(left, right),
                span: frame.current_span(),
            }),
        }
    }

    /// Performs division with type promotion.
    fn numeric_div(left: Value, right: Value, frame: &Frame<'_>) -> Result<Value, Error> {
        match (left, right) {
            (Value::Integer(left_int), Value::Integer(right_int)) => {
                if right_int == 0 {
                    Err(Error::DivisionByZero {
                        span: frame.current_span(),
                    })
                } else {
                    Ok(Value::Integer(left_int.checked_div(right_int).unwrap_or(0)))
                }
            }
            (Value::Float(left_float), Value::Float(right_float)) => {
                if right_float == 0.0 {
                    Err(Error::DivisionByZero {
                        span: frame.current_span(),
                    })
                } else {
                    Ok(Value::Float(left_float / right_float))
                }
            }
            (Value::Integer(left_int), Value::Float(right_float)) => {
                if right_float == 0.0 {
                    Err(Error::DivisionByZero {
                        span: frame.current_span(),
                    })
                } else {
                    #[expect(
                        clippy::as_conversions,
                        clippy::cast_precision_loss,
                        reason = "i64 to f64 may lose precision for very large integers"
                    )]
                    let left_as_float = left_int as f64;
                    Ok(Value::Float(left_as_float / right_float))
                }
            }
            (Value::Float(left_float), Value::Integer(right_int)) => {
                if right_int == 0 {
                    Err(Error::DivisionByZero {
                        span: frame.current_span(),
                    })
                } else {
                    #[expect(
                        clippy::as_conversions,
                        clippy::cast_precision_loss,
                        reason = "i64 to f64 may lose precision for very large integers"
                    )]
                    let right_as_float = right_int as f64;
                    Ok(Value::Float(left_float / right_as_float))
                }
            }
            _ => Err(Error::TypeError {
                expected: "number",
                got: binary_type_description(left, right),
                span: frame.current_span(),
            }),
        }
    }

    /// Performs modulo with type promotion.
    #[expect(
        clippy::modulo_arithmetic,
        reason = "[approved] Standard IEEE 754 float modulo for language runtime"
    )]
    fn numeric_mod(left: Value, right: Value, frame: &Frame<'_>) -> Result<Value, Error> {
        match (left, right) {
            (Value::Integer(left_int), Value::Integer(right_int)) => {
                if right_int == 0 {
                    Err(Error::DivisionByZero {
                        span: frame.current_span(),
                    })
                } else {
                    Ok(Value::Integer(left_int.checked_rem(right_int).unwrap_or(0)))
                }
            }
            (Value::Float(left_float), Value::Float(right_float)) => {
                if right_float == 0.0 {
                    Err(Error::DivisionByZero {
                        span: frame.current_span(),
                    })
                } else {
                    Ok(Value::Float(left_float % right_float))
                }
            }
            (Value::Integer(left_int), Value::Float(right_float)) => {
                if right_float == 0.0 {
                    Err(Error::DivisionByZero {
                        span: frame.current_span(),
                    })
                } else {
                    #[expect(
                        clippy::as_conversions,
                        clippy::cast_precision_loss,
                        reason = "i64 to f64 may lose precision for very large integers"
                    )]
                    let left_as_float = left_int as f64;
                    Ok(Value::Float(left_as_float % right_float))
                }
            }
            (Value::Float(left_float), Value::Integer(right_int)) => {
                if right_int == 0 {
                    Err(Error::DivisionByZero {
                        span: frame.current_span(),
                    })
                } else {
                    #[expect(
                        clippy::as_conversions,
                        clippy::cast_precision_loss,
                        reason = "i64 to f64 may lose precision for very large integers"
                    )]
                    let right_as_float = right_int as f64;
                    Ok(Value::Float(left_float % right_as_float))
                }
            }
            _ => Err(Error::TypeError {
                expected: "number",
                got: binary_type_description(left, right),
                span: frame.current_span(),
            }),
        }
    }

    /// Performs a numeric comparison operation.
    fn numeric_compare<FI, FF>(
        left: Value,
        right: Value,
        frame: &Frame<'_>,
        int_cmp: FI,
        float_cmp: FF,
    ) -> Result<Value, Error>
    where
        FI: Fn(i64, i64) -> bool,
        FF: Fn(f64, f64) -> bool,
    {
        match (left, right) {
            (Value::Integer(left_int), Value::Integer(right_int)) => {
                Ok(Value::Bool(int_cmp(left_int, right_int)))
            }
            (Value::Float(left_float), Value::Float(right_float)) => {
                Ok(Value::Bool(float_cmp(left_float, right_float)))
            }
            (Value::Integer(left_int), Value::Float(right_float)) => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "i64 to f64 may lose precision for very large integers"
                )]
                let left_as_float = left_int as f64;
                Ok(Value::Bool(float_cmp(left_as_float, right_float)))
            }
            (Value::Float(left_float), Value::Integer(right_int)) => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "i64 to f64 may lose precision for very large integers"
                )]
                let right_as_float = right_int as f64;
                Ok(Value::Bool(float_cmp(left_float, right_as_float)))
            }
            _ => Err(Error::TypeError {
                expected: "number",
                got: binary_type_description(left, right),
                span: frame.current_span(),
            }),
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Returns the type name of a value.
const fn value_type_name(value: Value) -> &'static str {
    match value {
        Value::Nil => "nil",
        Value::Bool(_) => "boolean",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::Symbol(_) => "symbol",
        // Value is non-exhaustive, handle future variants
        _ => "unknown",
    }
}

/// Returns the type name of a constant.
const fn constant_type_name(constant: &Constant) -> &'static str {
    match *constant {
        Constant::Nil => "nil",
        Constant::Bool(_) => "boolean",
        Constant::Integer(_) => "integer",
        Constant::Float(_) => "float",
        Constant::String(_) => "string",
        Constant::Symbol(_) => "symbol",
        // Constant is non-exhaustive, handle future variants
        _ => "unknown",
    }
}

/// Returns a description of the types in a binary operation.
const fn binary_type_description(left: Value, right: Value) -> &'static str {
    match (left, right) {
        (Value::Nil, _) | (_, Value::Nil) => "nil",
        (Value::Bool(_), _) | (_, Value::Bool(_)) => "boolean",
        (Value::Symbol(_), _) | (_, Value::Symbol(_)) => "symbol",
        _ => "non-number",
    }
}

/// Tests if two values are equal.
#[expect(
    clippy::float_cmp,
    reason = "[approved] VM equality semantics require exact float comparison"
)]
fn values_equal(left: Value, right: Value) -> bool {
    match (left, right) {
        (Value::Nil, Value::Nil) => true,
        (Value::Bool(left_bool), Value::Bool(right_bool)) => left_bool == right_bool,
        (Value::Integer(left_int), Value::Integer(right_int)) => left_int == right_int,
        (Value::Float(left_float), Value::Float(right_float)) => left_float == right_float,
        (Value::Symbol(left_sym), Value::Symbol(right_sym)) => left_sym == right_sym,
        // Cross-type numeric comparison
        (Value::Integer(left_int), Value::Float(right_float)) => {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "i64 to f64 for comparison"
            )]
            let left_as_float = left_int as f64;
            left_as_float == right_float
        }
        (Value::Float(left_float), Value::Integer(right_int)) => {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "i64 to f64 for comparison"
            )]
            let right_as_float = right_int as f64;
            left_float == right_as_float
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lonala_compiler::opcode::{encode_abc, encode_abx, encode_asbx, rk_constant};
    use lonala_parser::Span;

    /// Creates a VM with a fresh interner for testing.
    fn make_vm(interner: &Interner) -> Vm<'_> {
        Vm::new(interner)
    }

    /// Creates a test chunk.
    fn make_chunk() -> Chunk {
        Chunk::new()
    }

    // =========================================================================
    // Literal Execution Tests
    // =========================================================================

    #[test]
    fn execute_load_true_and_return() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let _idx = chunk.emit(
            encode_abc(Opcode::LoadTrue, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn execute_load_false_and_return() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let _idx = chunk.emit(
            encode_abc(Opcode::LoadFalse, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn execute_load_nil_and_return() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let _idx = chunk.emit(
            encode_abc(Opcode::LoadNil, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn execute_load_integer() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k_idx = chunk.add_constant(Constant::Integer(42)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k_idx),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn execute_load_float() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k_idx = chunk.add_constant(Constant::Float(3.14)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k_idx),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Float(3.14));
    }

    // =========================================================================
    // Arithmetic Tests
    // =========================================================================

    #[test]
    fn execute_add_integers() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

        // LoadK R0, K0  ; R0 = 1
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k0),
            Span::new(0_usize, 1_usize),
        );
        // LoadK R1, K1  ; R1 = 2
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 1, k1),
            Span::new(1_usize, 2_usize),
        );
        // Add R0, R0, R1  ; R0 = R0 + R1 = 3
        let _idx = chunk.emit(
            encode_abc(Opcode::Add, 0, 0, 1),
            Span::new(2_usize, 3_usize),
        );
        // Return R0, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(3_usize, 4_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(3));
    }

    #[test]
    fn execute_add_with_constants() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(20)).unwrap();

        // Get RK encodings for constants
        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        // Add R0, K0, K1  ; R0 = 10 + 20 = 30
        let _idx = chunk.emit(
            encode_abc(Opcode::Add, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        // Return R0, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(30));
    }

    #[test]
    fn execute_add_floats() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Float(1.5)).unwrap();
        let k1 = chunk.add_constant(Constant::Float(2.5)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Add, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Float(4.0));
    }

    #[test]
    fn execute_add_mixed_promotes_to_float() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let k1 = chunk.add_constant(Constant::Float(2.5)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Add, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Float(3.5));
    }

    #[test]
    fn execute_sub() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(3)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Sub, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(7));
    }

    #[test]
    fn execute_mul() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(6)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(7)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Mul, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn execute_div() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(3)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Div, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(3));
    }

    #[test]
    fn execute_mod() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(3)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Mod, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(1));
    }

    #[test]
    fn execute_neg_integer() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(42)).unwrap();

        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k0),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Neg, 0, 0, 0),
            Span::new(1_usize, 2_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(2_usize, 3_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(-42));
    }

    #[test]
    fn execute_neg_float() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Float(3.14)).unwrap();

        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k0),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Neg, 0, 0, 0),
            Span::new(1_usize, 2_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(2_usize, 3_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Float(-3.14));
    }

    // =========================================================================
    // Division by Zero Tests
    // =========================================================================

    #[test]
    fn execute_div_by_zero_integer() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(0)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Div, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );

        let result = vm.execute(&chunk);
        assert!(matches!(result, Err(Error::DivisionByZero { .. })));
    }

    #[test]
    fn execute_mod_by_zero() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(10)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(0)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Mod, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );

        let result = vm.execute(&chunk);
        assert!(matches!(result, Err(Error::DivisionByZero { .. })));
    }

    // =========================================================================
    // Comparison Tests
    // =========================================================================

    #[test]
    fn execute_eq_true() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(42)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(42)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Eq, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn execute_eq_false() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Eq, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn execute_lt() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Lt, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn execute_not_truthy() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let _idx = chunk.emit(
            encode_abc(Opcode::LoadTrue, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Not, 0, 0, 0),
            Span::new(1_usize, 2_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(2_usize, 3_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn execute_not_falsy() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let _idx = chunk.emit(
            encode_abc(Opcode::LoadNil, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Not, 0, 0, 0),
            Span::new(1_usize, 2_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(2_usize, 3_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    // =========================================================================
    // Global Variable Tests
    // =========================================================================

    #[test]
    fn execute_set_and_get_global() {
        let mut interner = Interner::new();
        let x_sym = interner.intern("x");

        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k_val = chunk.add_constant(Constant::Integer(42)).unwrap();
        let k_sym = chunk.add_constant(Constant::Symbol(x_sym)).unwrap();

        // LoadK R0, K0  ; R0 = 42
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k_val),
            Span::new(0_usize, 1_usize),
        );
        // SetGlobal R0, K1  ; globals[x] = 42
        let _idx = chunk.emit(
            encode_abx(Opcode::SetGlobal, 0, k_sym),
            Span::new(1_usize, 2_usize),
        );
        // LoadNil R0  ; R0 = nil (clear it)
        let _idx = chunk.emit(
            encode_abc(Opcode::LoadNil, 0, 0, 0),
            Span::new(2_usize, 3_usize),
        );
        // GetGlobal R0, K1  ; R0 = globals[x]
        let _idx = chunk.emit(
            encode_abx(Opcode::GetGlobal, 0, k_sym),
            Span::new(3_usize, 4_usize),
        );
        // Return R0, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(4_usize, 5_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn execute_undefined_global_error() {
        let mut interner = Interner::new();
        let x_sym = interner.intern("undefined");

        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k_sym = chunk.add_constant(Constant::Symbol(x_sym)).unwrap();

        // GetGlobal R0, K0  ; should fail
        let _idx = chunk.emit(
            encode_abx(Opcode::GetGlobal, 0, k_sym),
            Span::new(0_usize, 1_usize),
        );

        let result = vm.execute(&chunk);
        assert!(matches!(result, Err(Error::UndefinedGlobal { .. })));
    }

    // =========================================================================
    // Control Flow Tests
    // =========================================================================

    #[test]
    fn execute_unconditional_jump() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

        // 0: LoadK R0, K0  ; R0 = 1
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k0),
            Span::new(0_usize, 1_usize),
        );
        // 1: Jump +1  ; skip next instruction
        let _idx = chunk.emit(encode_asbx(Opcode::Jump, 0, 1), Span::new(1_usize, 2_usize));
        // 2: LoadK R0, K1  ; R0 = 2 (should be skipped)
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k1),
            Span::new(2_usize, 3_usize),
        );
        // 3: Return R0, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(3_usize, 4_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(1));
    }

    #[test]
    fn execute_jump_if_true() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

        // 0: LoadTrue R0
        let _idx = chunk.emit(
            encode_abc(Opcode::LoadTrue, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        );
        // 1: LoadK R1, K0  ; R1 = 1
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 1, k0),
            Span::new(1_usize, 2_usize),
        );
        // 2: JumpIf R0, +1  ; if true, skip next
        let _idx = chunk.emit(
            encode_asbx(Opcode::JumpIf, 0, 1),
            Span::new(2_usize, 3_usize),
        );
        // 3: LoadK R1, K1  ; R1 = 2 (should be skipped)
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 1, k1),
            Span::new(3_usize, 4_usize),
        );
        // 4: Return R1, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 1, 1, 0),
            Span::new(4_usize, 5_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(1));
    }

    #[test]
    fn execute_jump_if_not_false() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let k1 = chunk.add_constant(Constant::Integer(2)).unwrap();

        // 0: LoadFalse R0
        let _idx = chunk.emit(
            encode_abc(Opcode::LoadFalse, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        );
        // 1: LoadK R1, K0  ; R1 = 1
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 1, k0),
            Span::new(1_usize, 2_usize),
        );
        // 2: JumpIfNot R0, +1  ; if false, skip next
        let _idx = chunk.emit(
            encode_asbx(Opcode::JumpIfNot, 0, 1),
            Span::new(2_usize, 3_usize),
        );
        // 3: LoadK R1, K1  ; R1 = 2 (should be skipped)
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 1, k1),
            Span::new(3_usize, 4_usize),
        );
        // 4: Return R1, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 1, 1, 0),
            Span::new(4_usize, 5_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(1));
    }

    // =========================================================================
    // Type Error Tests
    // =========================================================================

    #[test]
    fn execute_add_type_error() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let k1 = chunk.add_constant(Constant::Bool(true)).unwrap();

        let rk0 = rk_constant(u8::try_from(k0).unwrap()).unwrap();
        let rk1 = rk_constant(u8::try_from(k1).unwrap()).unwrap();

        let _idx = chunk.emit(
            encode_abc(Opcode::Add, 0, rk0, rk1),
            Span::new(0_usize, 1_usize),
        );

        let result = vm.execute(&chunk);
        assert!(matches!(result, Err(Error::TypeError { .. })));
    }

    #[test]
    fn execute_neg_type_error() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let _idx = chunk.emit(
            encode_abc(Opcode::LoadTrue, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Neg, 0, 0, 0),
            Span::new(1_usize, 2_usize),
        );

        let result = vm.execute(&chunk);
        assert!(matches!(result, Err(Error::TypeError { .. })));
    }

    // =========================================================================
    // Move Operation Test
    // =========================================================================

    #[test]
    fn execute_move() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        let k0 = chunk.add_constant(Constant::Integer(42)).unwrap();

        // LoadK R0, K0  ; R0 = 42
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k0),
            Span::new(0_usize, 1_usize),
        );
        // Move R1, R0  ; R1 = R0
        let _idx = chunk.emit(
            encode_abc(Opcode::Move, 1, 0, 0),
            Span::new(1_usize, 2_usize),
        );
        // Return R1, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 1, 1, 0),
            Span::new(2_usize, 3_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    // =========================================================================
    // Empty Chunk Test
    // =========================================================================

    #[test]
    fn execute_empty_chunk_returns_nil() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);
        let chunk = make_chunk();

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Nil);
    }

    // =========================================================================
    // Native Function Call Tests
    // =========================================================================

    #[test]
    fn execute_print_returns_nil() {
        // Test that print call completes without error and returns nil
        let mut interner = Interner::new();
        let print_sym = interner.intern("print");

        let mut vm = Vm::new(&interner);
        vm.update_print_symbol(print_sym);
        // Register print as a global (the value is the symbol itself)
        vm.set_global(print_sym, Value::Symbol(print_sym));
        // Note: No print callback set - output is discarded

        let mut chunk = make_chunk();
        // GetGlobal R0, K0 (print symbol)
        let k_print = chunk.add_constant(Constant::Symbol(print_sym)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::GetGlobal, 0, k_print),
            Span::new(0_usize, 5_usize),
        );
        // LoadK R1, K1 (42)
        let k_42 = chunk.add_constant(Constant::Integer(42)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 1, k_42),
            Span::new(6_usize, 8_usize),
        );
        // Call R0, 1, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Call, 0, 1, 1),
            Span::new(0_usize, 10_usize),
        );
        // Return R0, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(0_usize, 10_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        // Print returns nil
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn execute_print_with_multiple_args() {
        let mut interner = Interner::new();
        let print_sym = interner.intern("print");

        let mut vm = Vm::new(&interner);
        vm.update_print_symbol(print_sym);
        vm.set_global(print_sym, Value::Symbol(print_sym));

        let mut chunk = make_chunk();
        // GetGlobal R0, K0 (print symbol)
        let k_print = chunk.add_constant(Constant::Symbol(print_sym)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::GetGlobal, 0, k_print),
            Span::new(0_usize, 5_usize),
        );
        // LoadK R1, K1 (1)
        let k_1 = chunk.add_constant(Constant::Integer(1)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 1, k_1),
            Span::new(0_usize, 1_usize),
        );
        // LoadK R2, K2 (2)
        let k_2 = chunk.add_constant(Constant::Integer(2)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 2, k_2),
            Span::new(0_usize, 1_usize),
        );
        // LoadK R3, K3 (3)
        let k_3 = chunk.add_constant(Constant::Integer(3)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 3, k_3),
            Span::new(0_usize, 1_usize),
        );
        // Call R0, 3, 1 (3 arguments)
        let _idx = chunk.emit(
            encode_abc(Opcode::Call, 0, 3, 1),
            Span::new(0_usize, 10_usize),
        );
        // Return R0, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(0_usize, 10_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn execute_native_function() {
        use crate::vm::NativeError;

        fn native_double(args: &[Value], _interner: &Interner) -> Result<Value, NativeError> {
            if args.len() != 1_usize {
                return Err(NativeError::ArityMismatch {
                    expected: 1,
                    got: args.len(),
                });
            }
            let n = args
                .first()
                .and_then(Value::as_integer)
                .ok_or(NativeError::TypeError {
                    expected: "integer",
                    got: "non-integer",
                    arg_index: 0,
                })?;
            Ok(Value::Integer(n.saturating_mul(2)))
        }

        let mut interner = Interner::new();
        let double_sym = interner.intern("double");

        let mut vm = Vm::new(&interner);
        vm.register_native(double_sym, native_double);
        // Register the function as a global
        vm.set_global(double_sym, Value::Symbol(double_sym));

        let mut chunk = make_chunk();
        // GetGlobal R0, K0 (double symbol)
        let k_double = chunk.add_constant(Constant::Symbol(double_sym)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::GetGlobal, 0, k_double),
            Span::new(0_usize, 6_usize),
        );
        // LoadK R1, K1 (21)
        let k_21 = chunk.add_constant(Constant::Integer(21)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 1, k_21),
            Span::new(0_usize, 2_usize),
        );
        // Call R0, 1, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Call, 0, 1, 1),
            Span::new(0_usize, 10_usize),
        );
        // Return R0, 1
        let _idx = chunk.emit(
            encode_abc(Opcode::Return, 0, 1, 0),
            Span::new(0_usize, 10_usize),
        );

        let result = vm.execute(&chunk).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn execute_undefined_function_error() {
        let mut interner = Interner::new();
        let unknown_sym = interner.intern("unknown");

        let mut vm = Vm::new(&interner);
        // Register the symbol as a global (so GetGlobal works)
        // but don't register it as a native function
        vm.set_global(unknown_sym, Value::Symbol(unknown_sym));

        let mut chunk = make_chunk();
        let k_unknown = chunk.add_constant(Constant::Symbol(unknown_sym)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::GetGlobal, 0, k_unknown),
            Span::new(0_usize, 7_usize),
        );
        let _idx = chunk.emit(
            encode_abc(Opcode::Call, 0, 0, 1),
            Span::new(0_usize, 10_usize),
        );

        let result = vm.execute(&chunk);
        assert!(matches!(result, Err(Error::UndefinedFunction { .. })));
    }

    #[test]
    fn execute_call_non_symbol_error() {
        let interner = Interner::new();
        let mut vm = make_vm(&interner);

        let mut chunk = make_chunk();
        // Load an integer (not a symbol) into R0
        let k_42 = chunk.add_constant(Constant::Integer(42)).unwrap();
        let _idx = chunk.emit(
            encode_abx(Opcode::LoadK, 0, k_42),
            Span::new(0_usize, 2_usize),
        );
        // Try to call it
        let _idx = chunk.emit(
            encode_abc(Opcode::Call, 0, 0, 1),
            Span::new(0_usize, 10_usize),
        );

        let result = vm.execute(&chunk);
        assert!(matches!(result, Err(Error::NotCallable { .. })));
    }
}
