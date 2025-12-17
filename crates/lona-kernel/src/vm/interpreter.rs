// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Virtual machine interpreter for Lonala bytecode.

use alloc::vec;
use alloc::vec::Vec;

use lona_core::integer::Integer;
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
use super::helpers::{constant_type_name, value_type_name, values_equal};
use super::natives::{NativeFn, Registry as NativeRegistry};
use super::numeric;
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
        let result = numeric::add(&left, &right, frame)?;
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
        let result = numeric::sub(&left, &right, frame)?;
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
        let result = numeric::mul(&left, &right, frame)?;
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
        let result = numeric::div(&left, &right, frame)?;
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
        let result = numeric::modulo(&left, &right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Neg`: `R[A] = -R[B]`
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer/Ratio negation is safe with arbitrary precision"
    )]
    fn op_neg(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let src = decode_b(instruction);

        let operand = self.get_register(src, frame)?;
        let result = match operand {
            Value::Integer(ref int_val) => Value::Integer(-int_val),
            Value::Float(float_val) => Value::Float(-float_val),
            Value::Ratio(ref ratio_val) => Value::Ratio(-ratio_val),
            other @ (Value::Nil | Value::Bool(_) | Value::Symbol(_) | Value::String(_) | _) => {
                return Err(Error::TypeError {
                    expected: "number",
                    got: value_type_name(&other),
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
        let result = Value::Bool(values_equal(&left, &right));
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
        let result = numeric::compare(&left, &right, frame, |lv, rv| lv < rv)?;
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
        let result = numeric::compare(&left, &right, frame, |lv, rv| lv <= rv)?;
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
        let result = numeric::compare(&left, &right, frame, |lv, rv| lv > rv)?;
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
        let result = numeric::compare(&left, &right, frame, |lv, rv| lv >= rv)?;
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
            .cloned()
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
            Constant::Integer(num) => Value::Integer(Integer::from_i64(num)),
            Constant::Float(num) => Value::Float(num),
            Constant::Symbol(id) => Value::Symbol(id),
            Constant::String(ref text) => {
                Value::String(lona_core::string::HeapStr::from(text.as_str()))
            }
            // Handle Nil and future Constant variants (Constant is #[non_exhaustive])
            Constant::Nil | _ => Value::Nil,
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
}
