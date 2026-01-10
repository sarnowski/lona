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

mod call;
mod collection;
mod fn_compile;

#[cfg(test)]
mod compiler_test;

#[cfg(any(test, feature = "std"))]
mod disassemble;

use crate::bytecode::{BX_MASK, Chunk, MAX_SIGNED_BX, MIN_SIGNED_BX, encode_abc, encode_abx, op};
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;

#[cfg(any(test, feature = "std"))]
pub use disassemble::disassemble;

/// Maximum number of arguments for an intrinsic call.
const MAX_ARGS: u8 = 254; // X1..X254, X0 reserved for result

/// First register available for temporary storage during compilation.
/// Registers 128-255 are used as temps, giving 128 temp slots.
const TEMP_REG_BASE: u8 = 128;

/// Maximum number of parameter bindings in a function.
const MAX_PARAMS: usize = 16;

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

/// Maximum length of a symbol name for binding comparison.
const MAX_SYMBOL_NAME_LEN: usize = 64;

/// Maximum number of captured variables in a closure.
const MAX_CAPTURES: usize = 16;

/// A parameter binding: symbol name → register number.
#[derive(Clone, Copy)]
struct Binding {
    /// Symbol name for comparison (stored inline).
    name: [u8; MAX_SYMBOL_NAME_LEN],
    /// Length of the name.
    name_len: u8,
    /// Register containing the parameter value (X1, X2, ...).
    register: u8,
}

/// A captured variable: maps outer binding to inner register.
#[derive(Clone, Copy)]
struct Capture {
    /// Symbol name of the captured variable.
    name: [u8; MAX_SYMBOL_NAME_LEN],
    /// Length of the name.
    name_len: u8,
    /// Register in outer scope that holds the value.
    outer_register: u8,
    /// Register in inner scope where capture will be loaded.
    inner_register: u8,
}

/// Compiler state for a single expression.
pub struct Compiler<'a, M: MemorySpace> {
    /// The bytecode chunk being built.
    chunk: Chunk,
    /// Reference to the process (for reading/allocating values).
    proc: &'a mut Process,
    /// Reference to the memory space.
    mem: &'a mut M,
    /// Parameter bindings for the current function scope.
    bindings: [Binding; MAX_PARAMS],
    /// Number of active bindings.
    bindings_len: usize,
    /// Bindings from enclosing scope (for closure capture detection).
    outer_bindings: [Binding; MAX_PARAMS],
    /// Number of outer bindings.
    outer_bindings_len: usize,
    /// Captured variables for current closure being compiled.
    captures: [Capture; MAX_CAPTURES],
    /// Number of captures.
    captures_len: usize,
    /// Arity of the inner function (needed for capture register calculation).
    inner_arity: u8,
}

impl<'a, M: MemorySpace> Compiler<'a, M> {
    /// Create a new compiler.
    #[must_use]
    pub const fn new(proc: &'a mut Process, mem: &'a mut M) -> Self {
        Self {
            chunk: Chunk::new(),
            proc,
            mem,
            bindings: [Binding {
                name: [0; MAX_SYMBOL_NAME_LEN],
                name_len: 0,
                register: 0,
            }; MAX_PARAMS],
            bindings_len: 0,
            outer_bindings: [Binding {
                name: [0; MAX_SYMBOL_NAME_LEN],
                name_len: 0,
                register: 0,
            }; MAX_PARAMS],
            outer_bindings_len: 0,
            captures: [Capture {
                name: [0; MAX_SYMBOL_NAME_LEN],
                name_len: 0,
                outer_register: 0,
                inner_register: 0,
            }; MAX_CAPTURES],
            captures_len: 0,
            inner_arity: 0,
        }
    }

    /// Look up a binding for a symbol by name.
    ///
    /// Returns `Some(register)` if the symbol name matches a bound parameter.
    fn lookup_binding_by_name(&self, name: &str) -> Option<u8> {
        let name_bytes = name.as_bytes();
        for i in 0..self.bindings_len {
            let binding = &self.bindings[i];
            let binding_len = binding.name_len as usize;
            if binding_len == name_bytes.len()
                && binding.name[..binding_len] == name_bytes[..binding_len]
            {
                return Some(binding.register);
            }
        }
        None
    }

    /// Look up a binding in the outer scope (for closure capture).
    ///
    /// Returns `Some(register)` if the symbol matches an outer binding.
    fn lookup_outer_binding(&self, name: &str) -> Option<u8> {
        let name_bytes = name.as_bytes();
        for i in 0..self.outer_bindings_len {
            let binding = &self.outer_bindings[i];
            let binding_len = binding.name_len as usize;
            if binding_len == name_bytes.len()
                && binding.name[..binding_len] == name_bytes[..binding_len]
            {
                return Some(binding.register);
            }
        }
        None
    }

    /// Look up an existing capture for a symbol.
    ///
    /// Returns `Some(inner_register)` if this symbol was already captured.
    fn lookup_capture(&self, name: &str) -> Option<u8> {
        let name_bytes = name.as_bytes();
        for i in 0..self.captures_len {
            let capture = &self.captures[i];
            let capture_len = capture.name_len as usize;
            if capture_len == name_bytes.len()
                && capture.name[..capture_len] == name_bytes[..capture_len]
            {
                return Some(capture.inner_register);
            }
        }
        None
    }

    /// Add a capture for a symbol from outer scope.
    ///
    /// Returns the inner register where the captured value will be available.
    fn add_capture(&mut self, name: &str, outer_register: u8) -> Option<u8> {
        if self.captures_len >= MAX_CAPTURES {
            return None;
        }

        // Capture registers start after params: inner_arity + 1 + capture_index
        let inner_register = self.inner_arity + 1 + self.captures_len as u8;

        // Copy name
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len().min(MAX_SYMBOL_NAME_LEN);
        let mut name_buf = [0u8; MAX_SYMBOL_NAME_LEN];
        name_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);

        self.captures[self.captures_len] = Capture {
            name: name_buf,
            name_len: name_len as u8,
            outer_register,
            inner_register,
        };
        self.captures_len += 1;

        Some(inner_register)
    }

    /// Clear all bindings and captures (for starting a new function).
    const fn clear_bindings(&mut self) {
        self.bindings_len = 0;
        self.captures_len = 0;
    }

    /// Force capture of all grandparent variables into the current scope.
    ///
    /// This must be called BEFORE saving state when compiling a nested function.
    /// It ensures that any variable from grandparent scopes is captured in the
    /// current function, so nested functions can access them via the current
    /// function's registers (not grandparent registers that won't exist at runtime).
    fn capture_all_outer_bindings(&mut self) {
        for i in 0..self.outer_bindings_len {
            let binding = self.outer_bindings[i];
            let name_slice = &binding.name[..binding.name_len as usize];

            // Check if already in current bindings
            let in_bindings = self.bindings[..self.bindings_len]
                .iter()
                .any(|b| b.name[..b.name_len as usize] == *name_slice);

            // Check if already captured
            let in_captures = self.captures[..self.captures_len]
                .iter()
                .any(|c| c.name[..c.name_len as usize] == *name_slice);

            // If not already accessible, capture it
            if !in_bindings && !in_captures && self.captures_len < MAX_CAPTURES {
                let inner_reg = self.inner_arity + 1 + self.captures_len as u8;
                self.captures[self.captures_len] = Capture {
                    name: binding.name,
                    name_len: binding.name_len,
                    outer_register: binding.register,
                    inner_register: inner_reg,
                };
                self.captures_len += 1;
            }
        }
    }

    /// Set up `outer_bindings` for a nested function.
    ///
    /// Builds the scope chain visible to the nested function from:
    /// 1. Current bindings (params of this function)
    /// 2. Current captures (variables captured from parent, including grandparent
    ///    vars that were force-captured by `capture_all_outer_bindings`)
    ///
    /// Note: `capture_all_outer_bindings` must be called first to ensure
    /// grandparent variables are in current captures.
    fn setup_outer_bindings_for_nested_fn(&mut self) {
        let mut new_outer_bindings = [Binding {
            name: [0; MAX_SYMBOL_NAME_LEN],
            name_len: 0,
            register: 0,
        }; MAX_PARAMS];
        let mut new_outer_len = 0;

        // First, copy current parameters (these are directly accessible)
        for binding in self.bindings.iter().take(self.bindings_len) {
            if new_outer_len < MAX_PARAMS {
                new_outer_bindings[new_outer_len] = *binding;
                new_outer_len += 1;
            }
        }

        // Then, copy current captures (including grandparent vars)
        // Use inner_register since that's where the value lives in current scope
        for capture in self.captures.iter().take(self.captures_len) {
            if new_outer_len < MAX_PARAMS {
                new_outer_bindings[new_outer_len] = Binding {
                    name: capture.name,
                    name_len: capture.name_len,
                    register: capture.inner_register,
                };
                new_outer_len += 1;
            }
        }

        self.outer_bindings = new_outer_bindings;
        self.outer_bindings_len = new_outer_len;
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
            Value::Symbol(_) => self.compile_symbol(expr, target, temp_base),
            Value::Keyword(_) => {
                // Keywords are self-evaluating constants
                self.compile_constant(expr, target)?;
                Ok(temp_base)
            }
            Value::Pair(_) => self.compile_list(expr, target, temp_base),
            Value::Tuple(_) => self.compile_tuple(expr, target, temp_base),
            Value::Map(_) => self.compile_map(expr, target, temp_base),
            Value::Namespace(_) => {
                // Namespaces are self-evaluating constants
                self.compile_constant(expr, target)?;
                Ok(temp_base)
            }
            Value::CompiledFn(_) | Value::Closure(_) | Value::NativeFn(_) => {
                // Functions are self-evaluating constants
                self.compile_constant(expr, target)?;
                Ok(temp_base)
            }
            Value::Var(_) => {
                // Vars are self-evaluating (return the var object itself)
                self.compile_constant(expr, target)?;
                Ok(temp_base)
            }
            Value::Unbound => {
                // Unbound is a special sentinel - shouldn't appear in source
                Err(CompileError::InvalidSyntax)
            }
        }
    }

    /// Compile a symbol reference.
    fn compile_symbol(
        &mut self,
        expr: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Read the symbol name into a local buffer to avoid borrow conflicts
        let name_str = self
            .proc
            .read_string(self.mem, expr)
            .ok_or(CompileError::InvalidSyntax)?;

        // Copy to local buffer to release the borrow
        let mut name_buf = [0u8; MAX_SYMBOL_NAME_LEN];
        let name_bytes = name_str.as_bytes();
        let name_len = name_bytes.len().min(MAX_SYMBOL_NAME_LEN);
        name_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);

        // Work with the copied name
        let name =
            core::str::from_utf8(&name_buf[..name_len]).map_err(|_| CompileError::InvalidSyntax)?;

        // Check if symbol is bound to a parameter in current scope
        if let Some(reg) = self.lookup_binding_by_name(name) {
            // Emit MOVE to copy parameter to target
            self.chunk
                .emit(encode_abc(op::MOVE, target, u16::from(reg), 0));
            return Ok(temp_base);
        }

        // Check if we already captured this variable
        if let Some(capture_reg) = self.lookup_capture(name) {
            self.chunk
                .emit(encode_abc(op::MOVE, target, u16::from(capture_reg), 0));
            return Ok(temp_base);
        }

        // Check if symbol is in outer scope (capture candidate)
        if let Some(outer_reg) = self.lookup_outer_binding(name) {
            // Add capture and use the inner register
            let inner_reg = self
                .add_capture(name, outer_reg)
                .ok_or(CompileError::ExpressionTooComplex)?;
            self.chunk
                .emit(encode_abc(op::MOVE, target, u16::from(inner_reg), 0));
            return Ok(temp_base);
        }

        // Unbound symbol
        Err(CompileError::UnboundSymbol)
    }

    /// Compile an integer literal.
    fn compile_int(&mut self, n: i64, target: u8) -> Result<(), CompileError> {
        // Check if it fits in 18-bit signed immediate
        if n >= i64::from(MIN_SIGNED_BX) && n <= i64::from(MAX_SIGNED_BX) {
            // Encode as two's complement in 18 bits using to_ne_bytes/from_ne_bytes
            // for explicit bit-level reinterpretation without sign loss issues.
            let bytes = (n as i32).to_ne_bytes();
            let bx = u32::from_ne_bytes(bytes) & BX_MASK;
            self.chunk.emit(encode_abx(op::LOADINT, target, bx));
            Ok(())
        } else {
            // Too large for inline, use constant pool
            self.compile_constant(Value::int(n), target)
        }
    }

    /// Compile a constant (load from constant pool).
    pub(crate) fn compile_constant(
        &mut self,
        value: Value,
        target: u8,
    ) -> Result<(), CompileError> {
        let idx = self
            .chunk
            .add_constant(value)
            .ok_or(CompileError::ConstantPoolFull)?;
        self.chunk.emit(encode_abx(op::LOADK, target, idx));
        Ok(())
    }

    /// Compile the `do` special form.
    ///
    /// `(do expr1 expr2 ... exprN)` evaluates each expression in sequence
    /// and returns the value of the last one.
    pub(crate) fn compile_do(
        &mut self,
        body: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        let mut current = body;
        let mut result = temp_base;

        // Empty do returns nil
        if current.is_nil() {
            self.chunk.emit(encode_abx(op::LOADNIL, target, 0));
            return Ok(temp_base);
        }

        // Evaluate each expression
        while !current.is_nil() {
            let pair = self
                .proc
                .read_pair(self.mem, current)
                .ok_or(CompileError::InvalidSyntax)?;

            // Compile each expression to target (last one's value is kept)
            result = self.compile_expr(pair.first, target, temp_base)?;
            current = pair.rest;
        }

        Ok(result)
    }
}

/// Convenience function to compile an expression.
///
/// # Errors
///
/// Returns an error if compilation fails.
pub fn compile<M: MemorySpace>(
    expr: Value,
    proc: &mut Process,
    mem: &mut M,
) -> Result<Chunk, CompileError> {
    Compiler::new(proc, mem).compile(expr)
}
