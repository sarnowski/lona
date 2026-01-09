// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Function compilation for `fn*` special form and closures.

use crate::bytecode::{Chunk, encode_abc, encode_abx, op};
use crate::platform::MemorySpace;
use crate::value::Value;

use super::{
    Binding, CompileError, Compiler, MAX_CAPTURES, MAX_PARAMS, MAX_SYMBOL_NAME_LEN, TEMP_REG_BASE,
};

impl<M: MemorySpace> Compiler<'_, M> {
    /// Compile the `fn*` special form.
    ///
    /// Supports two forms:
    /// - `(fn* [params...] body...)` - anonymous function
    /// - `(fn* name [params...] body...)` - named function (for debugging)
    ///
    /// The function is compiled to bytecode, allocated on the heap as a
    /// `HeapCompiledFn`. If the function captures variables from the enclosing
    /// scope, a `Closure` is created instead.
    ///
    /// Variadic functions use `&` before the rest parameter:
    /// - `(fn* [a b & rest] ...)` - 2 required params, rest collected in tuple
    pub(super) fn compile_fn(
        &mut self,
        args: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Parse: (fn* [params] body...) or (fn* name [params] body...)
        let pair = self
            .proc
            .read_pair(self.mem, args)
            .ok_or(CompileError::InvalidSyntax)?;

        // First argument can be a symbol (name) or tuple (params)
        let (params, body) = match pair.first {
            Value::Symbol(_) => {
                // Named function: (fn* name [params] body...)
                // Name is parsed for syntax but not stored (prints as #<fn/N>)
                // Read the next element which should be params
                let next_pair = self
                    .proc
                    .read_pair(self.mem, pair.rest)
                    .ok_or(CompileError::InvalidSyntax)?;
                let params = next_pair.first;
                if !matches!(params, Value::Tuple(_)) {
                    return Err(CompileError::InvalidSyntax);
                }
                (params, next_pair.rest)
            }
            Value::Tuple(_) => {
                // Anonymous function: (fn* [params] body...)
                (pair.first, pair.rest)
            }
            _ => return Err(CompileError::InvalidSyntax),
        };

        // Parse parameter list, detecting variadic `&`
        let (arity, variadic) = self.parse_fn_params(params)?;

        // Save current state FIRST (before modifying anything)
        // We save entire arrays because setup_outer_bindings_for_nested_fn modifies them.
        let saved_bindings = self.bindings;
        let saved_bindings_len = self.bindings_len;
        let saved_outer_bindings = self.outer_bindings;
        let saved_outer_bindings_len = self.outer_bindings_len;
        let saved_captures_len = self.captures_len;
        let saved_inner_arity = self.inner_arity;

        // Set up outer_bindings for nested function capture detection.
        // This builds the scope visible to nested functions from current bindings + captures.
        self.setup_outer_bindings_for_nested_fn();

        // Clear bindings and captures for the new function scope
        self.clear_bindings();
        self.inner_arity = arity;

        // Bind each required parameter to its register (X1, X2, ...)
        for i in 0..arity as usize {
            let param = self
                .proc
                .read_tuple_element(self.mem, params, i)
                .ok_or(CompileError::InvalidSyntax)?;

            // Parameter must be a symbol
            let Value::Symbol(_) = param else {
                // Restore state on error
                self.bindings = saved_bindings;
                self.bindings_len = saved_bindings_len;
                self.outer_bindings = saved_outer_bindings;
                self.outer_bindings_len = saved_outer_bindings_len;
                self.captures_len = saved_captures_len;
                self.inner_arity = saved_inner_arity;
                return Err(CompileError::InvalidSyntax);
            };

            self.bind_param(param, (i + 1) as u8)?;
        }

        // If variadic, also bind the rest parameter (after the `&`)
        // For [a b & rest], rest is at index arity+1 and binds to X(arity+1)
        if variadic {
            let rest_idx = arity as usize + 1; // Skip the `&` at index `arity`
            let rest_param = self
                .proc
                .read_tuple_element(self.mem, params, rest_idx)
                .ok_or(CompileError::InvalidSyntax)?;
            self.bind_param(rest_param, arity + 1)?;
        }

        // Force capture of grandparent variables AFTER binding params.
        // At this point, bindings contains only THIS function's params (not parent's).
        // This ensures nested functions can access grandparent vars via current scope's
        // registers (not grandparent registers that won't exist at runtime).
        self.capture_all_outer_bindings();

        // Compile the function body to a separate chunk
        // This will detect captures via lookup_outer_binding
        let fn_chunk = self.compile_fn_body(body)?;

        // Collect captures info before restoring state
        let captures_count = self.captures_len;
        let mut capture_outer_regs = [0u8; MAX_CAPTURES];
        for (i, capture) in self.captures.iter().enumerate().take(captures_count) {
            capture_outer_regs[i] = capture.outer_register;
        }

        // Restore state (arrays and lengths)
        self.bindings = saved_bindings;
        self.bindings_len = saved_bindings_len;
        self.outer_bindings = saved_outer_bindings;
        self.outer_bindings_len = saved_outer_bindings_len;
        self.captures_len = saved_captures_len;
        self.inner_arity = saved_inner_arity;

        // Allocate the CompiledFn on the heap
        let fn_val = self
            .proc
            .alloc_compiled_fn(
                self.mem,
                arity,
                variadic,
                0, // no extra locals
                &fn_chunk.code,
                &fn_chunk.constants,
            )
            .ok_or(CompileError::ConstantPoolFull)?;

        if captures_count == 0 {
            // No captures - just load the function as a constant
            self.compile_constant(fn_val, target)?;
        } else {
            // Has captures - emit bytecode to create closure at runtime
            self.emit_closure_creation(
                fn_val,
                &capture_outer_regs,
                captures_count,
                target,
                temp_base,
            )?;
        }

        Ok(temp_base)
    }

    /// Emit bytecode to create a closure at runtime.
    ///
    /// The closure combines a compiled function with captured values from the enclosing scope.
    fn emit_closure_creation(
        &mut self,
        fn_val: Value,
        capture_outer_regs: &[u8; MAX_CAPTURES],
        captures_count: usize,
        target: u8,
        temp_base: u8,
    ) -> Result<(), CompileError> {
        // Temp registers for building closure
        let fn_temp = temp_base;
        let captures_temp = temp_base + 1;
        let closure_temp = temp_base + 2;
        let captures_base = closure_temp + 1;

        // Load function constant
        self.compile_constant(fn_val, fn_temp)?;

        // Move capture values from outer registers to consecutive temp registers
        for (i, &src_reg) in capture_outer_regs.iter().enumerate().take(captures_count) {
            let dst_reg = captures_base + i as u8;
            self.chunk
                .emit(encode_abc(op::MOVE, dst_reg, u16::from(src_reg), 0));
        }

        // BUILD_TUPLE to create captures tuple
        self.chunk.emit(encode_abc(
            op::BUILD_TUPLE,
            captures_temp,
            u16::from(captures_base),
            captures_count as u16,
        ));

        // BUILD_CLOSURE target, fn_reg, captures_reg
        self.chunk.emit(encode_abc(
            op::BUILD_CLOSURE,
            target,
            u16::from(fn_temp),
            u16::from(captures_temp),
        ));

        Ok(())
    }

    /// Compile a function body to a standalone chunk.
    ///
    /// The body is a list of expressions that are implicitly wrapped in `do`.
    /// Returns the compiled chunk with a RETURN instruction at the end.
    fn compile_fn_body(&mut self, body: Value) -> Result<Chunk, CompileError> {
        // Create a new chunk for the function body
        let mut fn_chunk = Chunk::new();

        // Swap chunks so we compile to the new one
        core::mem::swap(&mut self.chunk, &mut fn_chunk);

        // Compile the body (implicit do)
        if body.is_nil() {
            // Empty body returns nil
            self.chunk.emit(encode_abx(op::LOADNIL, 0, 0));
        } else {
            self.compile_do(body, 0, TEMP_REG_BASE)?;
        }

        // Emit RETURN to return X0
        self.chunk.emit(encode_abx(op::RETURN, 0, 0));

        // Swap back and return the function chunk
        core::mem::swap(&mut self.chunk, &mut fn_chunk);
        Ok(fn_chunk)
    }

    /// Bind a parameter symbol to a register.
    fn bind_param(&mut self, param: Value, register: u8) -> Result<(), CompileError> {
        let param_name = self
            .proc
            .read_string(self.mem, param)
            .ok_or(CompileError::InvalidSyntax)?;

        // Copy name to local buffer
        let mut name_buf = [0u8; MAX_SYMBOL_NAME_LEN];
        let name_bytes = param_name.as_bytes();
        let name_len = name_bytes.len().min(MAX_SYMBOL_NAME_LEN);
        name_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);

        // Add binding
        if self.bindings_len < MAX_PARAMS {
            self.bindings[self.bindings_len] = Binding {
                name: name_buf,
                name_len: name_len as u8,
                register,
            };
            self.bindings_len += 1;
        }

        Ok(())
    }

    /// Parse function parameters, detecting variadic `&`.
    ///
    /// Returns (arity, variadic) where:
    /// - `arity` is the number of required parameters
    /// - `variadic` is true if the function accepts rest args via `&`
    ///
    /// For `[a b & rest]`: arity=2, variadic=true
    /// For `[a b c]`: arity=3, variadic=false
    fn parse_fn_params(&self, params: Value) -> Result<(u8, bool), CompileError> {
        let len = self
            .proc
            .read_tuple_len(self.mem, params)
            .ok_or(CompileError::InvalidSyntax)?;

        // Check for `&` in parameter list
        for i in 0..len {
            let param = self
                .proc
                .read_tuple_element(self.mem, params, i)
                .ok_or(CompileError::InvalidSyntax)?;

            if let Value::Symbol(_) = param {
                let name = self
                    .proc
                    .read_string(self.mem, param)
                    .ok_or(CompileError::InvalidSyntax)?;

                if name == "&" {
                    // Found variadic marker
                    // Must have exactly one param after `&`
                    if i + 2 != len {
                        return Err(CompileError::InvalidSyntax);
                    }
                    // Verify the rest param is a symbol
                    let rest_param = self
                        .proc
                        .read_tuple_element(self.mem, params, i + 1)
                        .ok_or(CompileError::InvalidSyntax)?;
                    if !matches!(rest_param, Value::Symbol(_)) {
                        return Err(CompileError::InvalidSyntax);
                    }
                    // arity is the number of params before `&`
                    return Ok((i as u8, true));
                }
            } else {
                // Parameter must be a symbol
                return Err(CompileError::InvalidSyntax);
            }
        }

        // No `&` found, all params are required
        Ok((len as u8, false))
    }
}
