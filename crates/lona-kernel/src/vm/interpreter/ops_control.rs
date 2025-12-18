// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Control flow and function call operations.

use alloc::vec::Vec;

use lona_core::opcode::{decode_a, decode_b, decode_c, decode_sbx};
use lona_core::symbol;
use lona_core::value::{Function, Value};

use super::{MAX_CALL_DEPTH, Vm};
use crate::vm::error::Error;
use crate::vm::frame::Frame;
use crate::vm::natives::NativeContext;
use crate::vm::primitives::format_print_args;

impl Vm<'_> {
    // =========================================================================
    // Control Flow Operations
    // =========================================================================

    /// `Jump`: `PC += sBx`
    pub(super) const fn op_jump(instruction: u32, frame: &mut Frame<'_>) {
        let sbx = decode_sbx(instruction);
        frame.jump(sbx);
    }

    /// `JumpIf`: `if R[A] then PC += sBx`
    pub(super) fn op_jump_if(&self, instruction: u32, frame: &mut Frame<'_>) -> Result<(), Error> {
        let cond_reg = decode_a(instruction);
        let offset = decode_sbx(instruction);

        let condition = self.get_register(cond_reg, frame)?;
        if condition.is_truthy() {
            frame.jump(offset);
        }
        Ok(())
    }

    /// `JumpIfNot`: `if not R[A] then PC += sBx`
    pub(super) fn op_jump_if_not(
        &self,
        instruction: u32,
        frame: &mut Frame<'_>,
    ) -> Result<(), Error> {
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
    pub(super) fn op_call(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let base = decode_a(instruction);
        let argc = decode_b(instruction);
        let _result_count = decode_c(instruction);

        // Get function value from R[base]
        let func_value = self.get_register(base, frame)?;

        match func_value {
            Value::Function(ref func) => {
                // User-defined function call
                self.call_user_function(func, base, argc, frame)
            }
            Value::Symbol(symbol) => {
                // Symbol-based function call (native or builtin)
                self.call_symbol_function(symbol, base, argc, frame)
            }
            // All other types are not callable
            Value::Nil
            | Value::Bool(_)
            | Value::Integer(_)
            | Value::Float(_)
            | Value::Ratio(_)
            | Value::String(_)
            | Value::List(_)
            | Value::Vector(_)
            | Value::Map(_)
            | _ => Err(Error::NotCallable {
                span: frame.current_span(),
            }),
        }
    }

    /// Calls a user-defined function.
    fn call_user_function(
        &mut self,
        func: &Function,
        base: u8,
        argc: u8,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        // Check for stack overflow
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(Error::StackOverflow {
                max_depth: MAX_CALL_DEPTH,
                span: frame.current_span(),
            });
        }

        // Verify arity
        if argc != func.arity() {
            return Err(Error::ArityMismatch {
                expected: func.arity(),
                got: argc,
                span: frame.current_span(),
            });
        }

        // Get the function's chunk directly from the Function value
        let fn_chunk = func.chunk();

        // Collect arguments from R[base+1] .. R[base+argc]
        let arguments = self.collect_args(base, argc, frame)?;

        // Calculate the new frame's base register
        // We'll use registers starting after the caller's current registers
        let new_base = frame
            .base()
            .saturating_add(usize::from(base))
            .saturating_add(1);

        // Ensure we have enough registers
        let needed_registers = new_base.saturating_add(usize::from(fn_chunk.max_registers()));
        if needed_registers > self.registers.len() {
            self.registers.resize(needed_registers, Value::Nil);
        }

        // Set up argument registers for the function (R[0], R[1], ... relative to new_base)
        for (idx, arg) in arguments.into_iter().enumerate() {
            let reg_idx = new_base.saturating_add(idx);
            if let Some(reg) = self.registers.get_mut(reg_idx) {
                *reg = arg;
            }
        }

        // Create new frame and execute
        let mut fn_frame = Frame::new(fn_chunk, new_base);
        self.call_depth = self.call_depth.saturating_add(1);
        let result = self.run(&mut fn_frame);
        self.call_depth = self.call_depth.saturating_sub(1);

        // Store result in R[base]
        let result_value = result?;
        self.set_register(base, result_value, frame)?;
        Ok(())
    }

    /// Calls a symbol-based function (native or builtin).
    fn call_symbol_function(
        &mut self,
        symbol: symbol::Id,
        base: u8,
        argc: u8,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        // Check if this is the built-in print function
        // (print is handled specially because it needs the print callback)
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

        // Create native context with interner and macro registry
        let ctx = NativeContext::new(self.interner, self.macro_registry);

        // Call native function
        let result = native_fn(&arguments, &ctx).map_err(|error| Error::Native {
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
    pub(super) fn op_return(
        &self,
        start_reg: u8,
        count: u8,
        frame: &Frame<'_>,
    ) -> Result<Value, Error> {
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
}
