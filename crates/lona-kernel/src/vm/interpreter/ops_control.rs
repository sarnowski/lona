// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Control flow and function call operations.

use alloc::sync::Arc;
use alloc::vec::Vec;

use lona_core::error_context::ArityExpectation;
use lona_core::list::List;
use lona_core::opcode::{decode_a, decode_b, decode_c, decode_sbx};
use lona_core::symbol;
use lona_core::value::{Function, Value};

use super::{MAX_CALL_DEPTH, Vm};
use crate::vm::error::{Error, Kind as ErrorKind};
use crate::vm::frame::Frame;
use crate::vm::natives::NativeContext;

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
            Value::Symbol(ref symbol) => {
                // Symbol-based function call (native or builtin)
                self.call_symbol_function(symbol.id(), base, argc, frame)
            }
            Value::NativeFunction(symbol) => {
                // First-class native function call
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
            | _ => Err(Error::new(
                ErrorKind::NotCallable {
                    got: func_value.kind(),
                },
                frame.current_location(),
            )),
        }
    }

    /// Calls a user-defined function.
    ///
    /// Handles multi-arity dispatch: finds the matching arity body based on
    /// argument count. For rest parameters, extra arguments beyond `arity`
    /// are collected into a list in the rest parameter position.
    fn call_user_function(
        &mut self,
        func: &Function,
        base: u8,
        argc: u8,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        // Check for stack overflow
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(Error::new(
                ErrorKind::StackOverflow {
                    max_depth: MAX_CALL_DEPTH,
                },
                frame.current_location(),
            ));
        }

        // Find matching arity body
        let body = func.find_body(usize::from(argc)).ok_or_else(|| {
            // Build arity expectation for error message
            let bodies = func.bodies();
            let expectation = if let &[ref first] = bodies {
                // Single body - use its arity
                if first.has_rest() {
                    ArityExpectation::AtLeast(first.arity())
                } else {
                    ArityExpectation::Exact(first.arity())
                }
            } else {
                // Multi-arity - use first body's arity as fallback
                bodies
                    .first()
                    .map_or(ArityExpectation::Exact(0), |first_body| {
                        ArityExpectation::Exact(first_body.arity())
                    })
            };
            Error::new(
                ErrorKind::ArityMismatch {
                    callable: None, // Function name is String, not symbol::Id
                    expected: expectation,
                    got: argc,
                },
                frame.current_location(),
            )
        })?;

        let fixed_arity = body.arity();
        let has_rest = body.has_rest();
        let fn_chunk = body.chunk();

        // Collect arguments from R[base+1] .. R[base+argc]
        let raw_arguments = self.collect_args(base, argc, frame)?;

        // Build the effective arguments list
        // Fixed args: raw_arguments[0..fixed_arity]
        // Rest arg (if has_rest): raw_arguments[fixed_arity..] collected into a list
        let arguments: Vec<Value> = if has_rest {
            let fixed_arity_usize = usize::from(fixed_arity);
            let mut effective = Vec::with_capacity(fixed_arity_usize.saturating_add(1));
            // Add fixed arguments
            for arg in raw_arguments.iter().take(fixed_arity_usize) {
                effective.push(arg.clone());
            }
            // Collect rest arguments into a list
            let rest_elements: Vec<Value> = raw_arguments
                .iter()
                .skip(fixed_arity_usize)
                .cloned()
                .collect();
            effective.push(Value::List(List::from_vec(rest_elements)));
            effective
        } else {
            raw_arguments
        };

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

        // Create new frame and execute with the closure's upvalues
        // Use the same source ID as the current frame for now
        // (TODO: functions could have their own source ID in the future)
        let mut fn_frame = Frame::with_upvalues(
            fn_chunk,
            new_base,
            frame.source(),
            Arc::clone(func.upvalues_arc()),
        );
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
        // Look up native function
        let native_fn = self.natives.get(symbol).ok_or_else(|| {
            Error::new(
                ErrorKind::UndefinedFunction {
                    symbol,
                    suggestion: None, // TODO: implement suggestion lookup
                },
                frame.current_location(),
            )
        })?;

        // Collect arguments from R[base+1] .. R[base+argc]
        let arguments = self.collect_args(base, argc, frame)?;

        // Create native context with interner and macro registry
        let ctx = NativeContext::new(self.interner, self.macro_registry);

        // Call native function
        let result = native_fn(&arguments, &ctx)
            .map_err(|error| Error::new(ErrorKind::Native { error }, frame.current_location()))?;

        // Store result in R[base]
        self.set_register(base, result, frame)?;
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
            let reg = reg_idx.ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidRegister { index: base },
                    frame.current_location(),
                )
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
