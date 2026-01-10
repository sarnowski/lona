// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Function and intrinsic call compilation.

#[cfg(any(test, feature = "std"))]
use std::vec::Vec;

#[cfg(not(any(test, feature = "std")))]
use alloc::vec::Vec;

use crate::bytecode::{encode_abc, op};
use crate::intrinsics::lookup_intrinsic;
use crate::platform::MemorySpace;
use crate::value::Value;

use super::{CompileError, Compiler, MAX_ARGS};

impl<M: MemorySpace> Compiler<'_, M> {
    /// Compile a list expression (special form, intrinsic call, or function call).
    pub(super) fn compile_list(
        &mut self,
        list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        let pair = self
            .proc
            .read_pair(self.mem, list)
            .ok_or(CompileError::InvalidSyntax)?;

        // Check if head is a symbol (could be special form, intrinsic, or bound parameter)
        if let Value::Symbol(_) = pair.first {
            // Look up the symbol name
            let name = self
                .proc
                .read_string(self.mem, pair.first)
                .ok_or(CompileError::InvalidSyntax)?;

            // Check for special forms first
            if name == "quote" {
                return self.compile_quote(pair.rest, target, temp_base);
            }
            if name == "fn*" {
                return self.compile_fn(pair.rest, target, temp_base);
            }
            if name == "do" {
                return self.compile_do(pair.rest, target, temp_base);
            }
            if name == "var" {
                return self.compile_var(pair.rest, target, temp_base);
            }

            // Check if it's a known intrinsic
            if let Some(intrinsic_id) = lookup_intrinsic(name) {
                return self.compile_intrinsic_call(intrinsic_id, pair.rest, target, temp_base);
            }

            // Check if it's a bound parameter (function value)
            if self.lookup_binding_by_name(name).is_some() {
                // It's a function call with a bound function
                return self.compile_call(pair.first, pair.rest, target, temp_base);
            }

            // Unknown symbol
            return Err(CompileError::UnboundSymbol);
        }

        // Head is not a symbol - compile it and call
        self.compile_call(pair.first, pair.rest, target, temp_base)
    }

    /// Compile a function call.
    ///
    /// The head expression is compiled to get the function, then arguments
    /// are compiled and a CALL instruction is emitted.
    pub(super) fn compile_call(
        &mut self,
        head: Value,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // First, collect arguments
        let mut args: Vec<Value> = Vec::new();
        let mut arg_count: u8 = 0;
        let mut current = arg_list;

        while !current.is_nil() {
            let pair = self
                .proc
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

        // Allocate temps: one for the function, then one per argument
        let fn_temp = temp_base;
        let arg_temps_base = temp_base
            .checked_add(1)
            .ok_or(CompileError::ExpressionTooComplex)?;
        let next_temp = arg_temps_base
            .checked_add(arg_count)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // Compile the head (function) to fn_temp
        let mut current_next_temp = self.compile_expr(head, fn_temp, next_temp)?;

        // Compile each argument to arg temps
        for (i, arg) in args.iter().enumerate() {
            let arg_temp = arg_temps_base
                .checked_add(i as u8)
                .ok_or(CompileError::ExpressionTooComplex)?;
            current_next_temp = self.compile_expr(*arg, arg_temp, current_next_temp)?;
        }

        // Move argument temps to X1..Xn
        for i in 0..arg_count {
            self.chunk.emit(encode_abc(
                op::MOVE,
                i + 1,
                u16::from(arg_temps_base + i),
                0,
            ));
        }

        // Emit CALL: fn_temp holds the function, argc is argument count
        // Result will be in X0
        self.chunk
            .emit(encode_abc(op::CALL, fn_temp, u16::from(arg_count), 0));

        // If target != 0, move X0 to target
        if target != 0 {
            self.chunk.emit(encode_abc(op::MOVE, target, 0, 0));
        }

        Ok(current_next_temp)
    }

    /// Compile an intrinsic call.
    ///
    /// Arguments are first compiled to temp registers, then moved to X1..Xn.
    /// This prevents nested calls from clobbering already-computed arguments.
    /// The INTRINSIC instruction puts the result in X0.
    /// If target != 0, we emit a MOVE to copy X0 to target.
    ///
    /// Returns the next available temp register after compilation.
    pub(super) fn compile_intrinsic_call(
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
                .proc
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

    /// Compile the `quote` special form.
    ///
    /// `(quote expr)` returns `expr` unevaluated.
    pub(super) fn compile_quote(
        &mut self,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Get the single argument
        let pair = self
            .proc
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

    /// Compile the `var` special form.
    ///
    /// `(var sym)` returns the var object for the given symbol.
    /// This is also the expansion of reader syntax `#'sym`.
    ///
    /// For qualified symbols like `user/x`, looks up the namespace and var.
    /// For unqualified symbols, returns `UnboundSymbol` error (current namespace
    /// tracking requires `*ns*` which is implemented in Phase 5).
    pub(super) fn compile_var(
        &mut self,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Get the single argument (must be a symbol)
        let pair = self
            .proc
            .read_pair(self.mem, arg_list)
            .ok_or(CompileError::InvalidSyntax)?;

        // var takes exactly one argument
        if !pair.rest.is_nil() {
            return Err(CompileError::InvalidSyntax);
        }

        // Argument must be a symbol
        if !pair.first.is_symbol() {
            return Err(CompileError::InvalidSyntax);
        }

        // Get the symbol name
        let name = self
            .proc
            .read_string(self.mem, pair.first)
            .ok_or(CompileError::InvalidSyntax)?;

        // Check if qualified (contains '/')
        if let Some(slash_pos) = name.rfind('/') {
            // Qualified symbol: split into namespace and name parts
            let ns_name = &name[..slash_pos];
            let var_name = &name[slash_pos + 1..];

            // Look up the namespace
            let ns_symbol = self
                .proc
                .find_interned_symbol(self.mem, ns_name)
                .ok_or(CompileError::UnboundSymbol)?;

            let ns = self
                .proc
                .find_namespace(self.mem, ns_symbol)
                .ok_or(CompileError::UnboundSymbol)?;

            // Look up the var in the namespace
            let var_symbol = self
                .proc
                .find_interned_symbol(self.mem, var_name)
                .ok_or(CompileError::UnboundSymbol)?;

            let ns_struct = self
                .proc
                .read_namespace(self.mem, ns)
                .ok_or(CompileError::UnboundSymbol)?;

            let var = self
                .proc
                .ns_lookup_var(self.mem, ns_struct.mappings, var_symbol)
                .ok_or(CompileError::UnboundSymbol)?;

            // Load the var as a constant
            self.compile_constant(var, target)?;
            Ok(temp_base)
        } else {
            // Unqualified symbol: would need *ns* for current namespace
            // This is implemented in Phase 5 with the def special form
            Err(CompileError::UnboundSymbol)
        }
    }
}
