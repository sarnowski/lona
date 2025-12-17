// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Compiler from Lonala AST to bytecode.
//!
//! Transforms parsed [`Ast`] expressions into executable [`Chunk`] bytecode.
//! This module implements the core compilation logic for the Lonala language.

use lona_core::symbol;
use lonala_parser::{Ast, Span, Spanned};

use crate::chunk::{Chunk, Constant};
use crate::error::Error;
use crate::opcode::{
    Opcode, RK_MAX_CONSTANT, encode_abc, encode_abx, encode_asbx, rk_constant, rk_register,
};

#[cfg(test)]
mod tests;

/// Result of compiling an expression.
///
/// Contains the register where the expression's value is stored after
/// the compiled instructions execute.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ExprResult {
    /// Register containing the expression's result.
    pub register: u8,
}

/// Compiler state for transforming AST to bytecode.
///
/// Manages register allocation, constant pool, and instruction emission
/// during compilation of a single chunk (function or top-level code).
pub struct Compiler<'interner> {
    /// The bytecode chunk being built.
    chunk: Chunk,
    /// Symbol interner for looking up and creating symbol IDs.
    interner: &'interner mut symbol::Interner,
    /// Next available register.
    next_register: u8,
    /// Maximum register used so far (for tracking chunk metadata).
    max_register: u8,
}

impl<'interner> Compiler<'interner> {
    /// Creates a new compiler with the given symbol interner.
    #[inline]
    #[must_use]
    pub const fn new(interner: &'interner mut symbol::Interner) -> Self {
        Self {
            chunk: Chunk::new(),
            interner,
            next_register: 0,
            max_register: 0,
        }
    }

    /// Compiles a sequence of expressions into a chunk.
    ///
    /// Compiles each expression in order and emits a `Return` for the
    /// last expression's value (or nil if the sequence is empty).
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails (too many constants,
    /// registers, or unsupported features).
    #[inline]
    pub fn compile_program(mut self, exprs: &[Spanned<Ast>]) -> Result<Chunk, Error> {
        let last_result = if exprs.is_empty() {
            // Empty program returns nil
            let span = Span::new(0_usize, 0_usize);
            let reg = self.alloc_register(span)?;
            self.chunk
                .emit(encode_abc(Opcode::LoadNil, reg, 0, 0), span);
            ExprResult { register: reg }
        } else {
            // Compile all expressions, keeping the last result
            let mut result = ExprResult { register: 0 };
            for expr in exprs {
                // Reset registers before each top-level expression
                self.next_register = 0;
                result = self.compile_expr(expr)?;
            }
            result
        };

        // Emit final return
        let span = exprs
            .last()
            .map_or(Span::new(0_usize, 0_usize), |last| last.span);
        self.chunk
            .emit(encode_abc(Opcode::Return, last_result.register, 1, 0), span);

        // Update chunk metadata
        self.chunk
            .set_max_registers(self.max_register.saturating_add(1));

        Ok(self.chunk)
    }

    /// Compiles a single expression.
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails.
    #[inline]
    pub fn compile_expr(&mut self, expr: &Spanned<Ast>) -> Result<ExprResult, Error> {
        match expr.node {
            // Literals
            Ast::Integer(num) => self.compile_integer(num, expr.span),
            Ast::Float(num) => self.compile_float(num, expr.span),
            Ast::Bool(val) => self.compile_bool(val, expr.span),
            Ast::Nil => self.compile_nil(expr.span),

            // Symbols (global variable lookup)
            Ast::Symbol(ref name) => self.compile_symbol(name, expr.span),

            // Lists (function calls or special forms)
            Ast::List(ref elements) => self.compile_list(elements, expr.span),

            // String literals
            Ast::String(ref string) => self.compile_string(string, expr.span),
            Ast::Keyword(ref _kw) => Err(Error::NotImplemented {
                feature: "keyword literals",
                span: expr.span,
            }),
            Ast::Vector(ref _elements) => Err(Error::NotImplemented {
                feature: "vector literals",
                span: expr.span,
            }),
            Ast::Map(ref _elements) => Err(Error::NotImplemented {
                feature: "map literals",
                span: expr.span,
            }),

            // Handle future Ast variants (Ast is #[non_exhaustive])
            _ => Err(Error::NotImplemented {
                feature: "unknown AST node",
                span: expr.span,
            }),
        }
    }

    // =========================================================================
    // Literal Compilation
    // =========================================================================

    /// Compiles an integer literal.
    fn compile_integer(&mut self, value: i64, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let const_idx = self.chunk.add_constant_at(Constant::Integer(value), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a float literal.
    fn compile_float(&mut self, value: f64, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let const_idx = self.chunk.add_constant_at(Constant::Float(value), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a boolean literal.
    fn compile_bool(&mut self, value: bool, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let opcode = if value {
            Opcode::LoadTrue
        } else {
            Opcode::LoadFalse
        };
        self.chunk.emit(encode_abc(opcode, dest, 0, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a nil literal.
    fn compile_nil(&mut self, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a string literal.
    fn compile_string(&mut self, value: &str, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let const_idx = self
            .chunk
            .add_constant_at(Constant::String(alloc::string::String::from(value)), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    // =========================================================================
    // Symbol Compilation
    // =========================================================================

    /// Compiles a symbol as a global variable lookup.
    ///
    /// Note: This method always interns from string. The interner correctly
    /// deduplicates, so repeated symbols within the same compilation share
    /// the same `symbol::Id`. In future phases, if the parser pre-interns
    /// symbols, this method could be refactored to accept `symbol::Id`
    /// directly to avoid the lookup overhead.
    fn compile_symbol(&mut self, name: &str, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let sym_id = self.interner.intern(name);
        let const_idx = self.chunk.add_constant_at(Constant::Symbol(sym_id), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    // =========================================================================
    // List (Call) Compilation
    // =========================================================================

    /// Compiles a list as a function call, special form, or arithmetic operation.
    fn compile_list(&mut self, elements: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        if elements.is_empty() {
            return Err(Error::EmptyCall { span });
        }

        // Check if the first element is a symbol (could be special form or operator)
        if let Some(spanned_func) = elements.first()
            && let Ast::Symbol(ref name) = spanned_func.node
        {
            // Check for special forms first
            let args = elements.get(1_usize..).unwrap_or(&[]);
            match name.as_str() {
                "do" => return self.compile_do(args, span),
                "if" => return self.compile_if(args, span),
                "def" => return self.compile_def(args, span),
                _ => {}
            }

            // Check for unary 'not'
            if name == "not" && elements.len() == 2_usize {
                return self.compile_not(elements, span);
            }

            // Check for binary operators (arithmetic and comparison)
            if let Some(opcode) = Self::binary_opcode(name) {
                return self.compile_binary_op(opcode, elements, span);
            }
        }

        // General function call
        self.compile_call(elements, span)
    }

    /// Returns the opcode for a binary operator symbol, if any.
    ///
    /// Handles both arithmetic operators (`+`, `-`, `*`, `/`, `mod`) and
    /// comparison operators (`=`, `<`, `>`, `<=`, `>=`).
    fn binary_opcode(name: &str) -> Option<Opcode> {
        match name {
            // Arithmetic operators
            "+" => Some(Opcode::Add),
            "-" => Some(Opcode::Sub),
            "*" => Some(Opcode::Mul),
            "/" => Some(Opcode::Div),
            "mod" => Some(Opcode::Mod),
            // Comparison operators
            "=" => Some(Opcode::Eq),
            "<" => Some(Opcode::Lt),
            ">" => Some(Opcode::Gt),
            "<=" => Some(Opcode::Le),
            ">=" => Some(Opcode::Ge),
            _ => None,
        }
    }

    /// Compiles a binary operation (arithmetic or comparison).
    ///
    /// Also handles unary negation `(- x)` as a special case.
    fn compile_binary_op(
        &mut self,
        opcode: Opcode,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        match elements.len() {
            // Unary: (- x) → Neg
            2_usize if opcode == Opcode::Sub => {
                let arg = elements.get(1_usize).ok_or(Error::EmptyCall { span })?;
                let checkpoint = self.next_register;
                let result = self.compile_expr(arg)?;

                self.free_registers_to(checkpoint);
                let dest = self.alloc_register(span)?;
                self.chunk
                    .emit(encode_abc(Opcode::Neg, dest, result.register, 0), span);
                Ok(ExprResult { register: dest })
            }
            // Binary: (op x y)
            3_usize => {
                let arg1 = elements.get(1_usize).ok_or(Error::EmptyCall { span })?;
                let arg2 = elements.get(2_usize).ok_or(Error::EmptyCall { span })?;

                // Save register checkpoint
                let checkpoint = self.next_register;

                // Try to use RK encoding for constant operands
                let rk_b = self.try_compile_rk_operand(arg1)?;
                let rk_c = self.try_compile_rk_operand(arg2)?;

                // Allocate destination register (reuse checkpoint if possible)
                self.free_registers_to(checkpoint);
                let dest = self.alloc_register(span)?;

                self.chunk.emit(encode_abc(opcode, dest, rk_b, rk_c), span);
                Ok(ExprResult { register: dest })
            }
            _ => Err(Error::NotImplemented {
                feature: "n-ary arithmetic",
                span,
            }),
        }
    }

    /// Compiles unary `not` operation.
    fn compile_not(&mut self, elements: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        let arg = elements.get(1_usize).ok_or(Error::EmptyCall { span })?;

        let checkpoint = self.next_register;
        let result = self.compile_expr(arg)?;

        self.free_registers_to(checkpoint);
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::Not, dest, result.register, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Tries to compile an operand as an RK value (constant if possible).
    ///
    /// Returns the RK-encoded value (either a register index or constant index).
    fn try_compile_rk_operand(&mut self, expr: &Spanned<Ast>) -> Result<u8, Error> {
        // Check if this can be a direct constant
        if let Some(rk) = self.try_constant_rk(expr)? {
            return Ok(rk);
        }

        // Otherwise compile to a register
        let result = self.compile_expr(expr)?;
        rk_register(result.register).ok_or(Error::TooManyRegisters { span: expr.span })
    }

    /// Tries to encode an expression as a constant in an RK field.
    ///
    /// Returns `Some(rk)` if the expression is a simple constant that fits in
    /// the RK constant range (index <= 127), `None` otherwise.
    fn try_constant_rk(&mut self, expr: &Spanned<Ast>) -> Result<Option<u8>, Error> {
        let constant = match expr.node {
            Ast::Integer(num) => Constant::Integer(num),
            Ast::Float(num) => Constant::Float(num),
            // Other AST types are not simple constants for RK encoding
            Ast::Nil
            | Ast::Bool(_)
            | Ast::String(_)
            | Ast::Symbol(_)
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Vector(_)
            | Ast::Map(_)
            // Handle future Ast variants (Ast is #[non_exhaustive])
            | _ => return Ok(None),
        };

        // Check if the next constant index would fit in RK range BEFORE adding.
        // This avoids adding the constant twice if it doesn't fit (once here,
        // once when falling back to register compilation).
        let next_idx = self.chunk.constants().len();
        if next_idx > usize::from(RK_MAX_CONSTANT) {
            return Ok(None);
        }

        // Add constant - index is guaranteed to fit in RK range
        let idx = self.chunk.add_constant_at(constant, expr.span)?;
        // The index must fit since we checked above
        let idx_u8 = u8::try_from(idx).ok();
        match idx_u8 {
            Some(i) if i <= RK_MAX_CONSTANT => Ok(rk_constant(i)),
            _ => Ok(None), // Should not happen, but handle gracefully
        }
    }

    /// Compiles a general function call.
    fn compile_call(&mut self, elements: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        let func_expr = elements.first().ok_or(Error::EmptyCall { span })?;
        let args = elements.get(1_usize..).unwrap_or(&[]);

        // Allocate contiguous registers: R_base = func, R_base+1..N = args
        let base = self.next_register;

        // Compile function into base register
        let func_result = self.compile_expr(func_expr)?;
        // Ensure function is at base (should be since we just allocated)
        if func_result.register != base {
            // Move to base if needed (shouldn't happen with current design)
            self.chunk.emit(
                encode_abc(Opcode::Move, base, func_result.register, 0),
                func_expr.span,
            );
        }

        // Compile arguments into consecutive registers
        for arg in args {
            let _arg_result = self.compile_expr(arg)?;
            // Arguments are automatically placed in consecutive registers
        }

        // Emit call instruction
        let arg_count =
            u8::try_from(args.len()).map_err(|_err| Error::TooManyRegisters { span })?;

        self.chunk
            .emit(encode_abc(Opcode::Call, base, arg_count, 1), span);

        // Result is left in base register
        // Free argument registers
        self.free_registers_to(base.saturating_add(1));

        Ok(ExprResult { register: base })
    }

    // =========================================================================
    // Special Forms
    // =========================================================================

    /// Compiles a `do` special form.
    ///
    /// Syntax: `(do)` or `(do expr1 expr2 ... exprN)`
    ///
    /// Evaluates expressions left to right and returns the value of the last
    /// expression. Empty `(do)` returns nil.
    fn compile_do(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        if args.is_empty() {
            // Empty do returns nil
            let dest = self.alloc_register(span)?;
            self.chunk
                .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
            return Ok(ExprResult { register: dest });
        }

        // Compile all but last expression, discarding results
        for expr in args
            .get(..args.len().saturating_sub(1_usize))
            .unwrap_or(&[])
        {
            let checkpoint = self.next_register;
            let _result = self.compile_expr(expr)?;
            self.free_registers_to(checkpoint);
        }

        // Compile last expression and return its result
        let last_expr = args.last().ok_or(Error::EmptyCall { span })?;
        self.compile_expr(last_expr)
    }

    /// Compiles an `if` special form.
    ///
    /// Syntax: `(if test then)` or `(if test then else)`
    ///
    /// Evaluates `test`. If truthy (not nil or false), evaluates and returns
    /// `then`. Otherwise evaluates and returns `else` (or nil if no else).
    fn compile_if(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        // Validate: need 2 or 3 args (test, then, [else])
        if args.len() < 2_usize || args.len() > 3_usize {
            return Err(Error::InvalidSpecialForm {
                form: "if",
                message: "expected (if test then) or (if test then else)",
                span,
            });
        }

        let test_expr = args.first().ok_or(Error::EmptyCall { span })?;
        let then_expr = args.get(1_usize).ok_or(Error::EmptyCall { span })?;
        let else_expr = args.get(2_usize);

        // Compile test expression
        let checkpoint = self.next_register;
        let test_result = self.compile_expr(test_expr)?;

        // Emit JumpIfNot (will patch offset later)
        let jump_to_else_idx = self.chunk.emit(
            encode_asbx(Opcode::JumpIfNot, test_result.register, 0),
            span,
        );

        // Free test register
        self.free_registers_to(checkpoint);

        // Allocate destination register for result
        let dest = self.alloc_register(span)?;

        // Compile then branch into dest
        let then_result = self.compile_expr(then_expr)?;
        if then_result.register != dest {
            self.chunk.emit(
                encode_abc(Opcode::Move, dest, then_result.register, 0),
                then_expr.span,
            );
        }
        // Free any temps from then branch but keep dest
        self.free_registers_to(dest.saturating_add(1));

        // Emit Jump over else branch (will patch offset later)
        let jump_to_end_idx = self.chunk.emit(encode_asbx(Opcode::Jump, 0, 0), span);

        // Patch jump_to_else to point here (current instruction index)
        let else_offset = self
            .chunk
            .len()
            .saturating_sub(jump_to_else_idx)
            .saturating_sub(1);
        let else_offset_i16 =
            i16::try_from(else_offset).map_err(|_err| Error::JumpTooLarge { span })?;
        self.chunk.patch(
            jump_to_else_idx,
            encode_asbx(Opcode::JumpIfNot, test_result.register, else_offset_i16),
        );

        // Compile else branch (or nil) into dest
        if let Some(else_branch) = else_expr {
            let else_result = self.compile_expr(else_branch)?;
            if else_result.register != dest {
                self.chunk.emit(
                    encode_abc(Opcode::Move, dest, else_result.register, 0),
                    else_branch.span,
                );
            }
        } else {
            self.chunk
                .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
        }
        // Free any temps from else branch but keep dest
        self.free_registers_to(dest.saturating_add(1));

        // Patch jump_to_end to point here
        let end_offset = self
            .chunk
            .len()
            .saturating_sub(jump_to_end_idx)
            .saturating_sub(1);
        let end_offset_i16 =
            i16::try_from(end_offset).map_err(|_err| Error::JumpTooLarge { span })?;
        self.chunk.patch(
            jump_to_end_idx,
            encode_asbx(Opcode::Jump, 0, end_offset_i16),
        );

        Ok(ExprResult { register: dest })
    }

    /// Compiles a `def` special form.
    ///
    /// Syntax: `(def name value)`
    ///
    /// Evaluates `value` and binds it to the global variable `name`.
    /// Returns the symbol `name`.
    fn compile_def(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        // Validate: need exactly 2 args (name, value)
        if args.len() != 2_usize {
            return Err(Error::InvalidSpecialForm {
                form: "def",
                message: "expected (def name value)",
                span,
            });
        }

        // First arg must be a symbol
        let name_expr = args.first().ok_or(Error::EmptyCall { span })?;
        let Ast::Symbol(ref name) = name_expr.node else {
            return Err(Error::InvalidSpecialForm {
                form: "def",
                message: "first argument must be a symbol",
                span: name_expr.span,
            });
        };

        let value_expr = args.get(1_usize).ok_or(Error::EmptyCall { span })?;

        // Compile value expression
        let checkpoint = self.next_register;
        let value_result = self.compile_expr(value_expr)?;

        // Intern the symbol and add to constants
        let symbol_id = self.interner.intern(name);
        let symbol_const = self
            .chunk
            .add_constant_at(Constant::Symbol(symbol_id), span)?;

        // Emit SetGlobal
        self.chunk.emit(
            encode_abx(Opcode::SetGlobal, value_result.register, symbol_const),
            span,
        );

        // Free value register
        self.free_registers_to(checkpoint);

        // Return the symbol (load it into destination)
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, symbol_const), span);

        Ok(ExprResult { register: dest })
    }

    // =========================================================================
    // Register Management
    // =========================================================================

    /// Allocates the next available register.
    ///
    /// # Errors
    ///
    /// Returns `Error::TooManyRegisters` if all 256 registers are in use.
    const fn alloc_register(&mut self, span: Span) -> Result<u8, Error> {
        let reg = self.next_register;
        if reg == u8::MAX {
            return Err(Error::TooManyRegisters { span });
        }
        self.next_register = reg.saturating_add(1);
        if reg > self.max_register {
            self.max_register = reg;
        }
        Ok(reg)
    }

    /// Releases registers back to a checkpoint.
    ///
    /// Used to reclaim temporary registers after a subexpression.
    const fn free_registers_to(&mut self, checkpoint: u8) {
        self.next_register = checkpoint;
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Error type for high-level compilation that includes parse errors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompileError {
    /// Error during parsing.
    Parse(lonala_parser::Error),
    /// Error during compilation.
    Compile(Error),
}

impl core::fmt::Display for CompileError {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Parse(ref err) => write!(f, "parse error: {err}"),
            Self::Compile(ref err) => write!(f, "compile error: {err}"),
        }
    }
}

impl From<lonala_parser::Error> for CompileError {
    #[inline]
    fn from(err: lonala_parser::Error) -> Self {
        Self::Parse(err)
    }
}

impl From<Error> for CompileError {
    #[inline]
    fn from(err: Error) -> Self {
        Self::Compile(err)
    }
}

/// Parses and compiles Lonala source code to bytecode.
///
/// Convenience function that combines parsing and compilation.
///
/// # Errors
///
/// Returns `CompileError::Parse` if parsing fails, or
/// `CompileError::Compile` if compilation fails.
///
/// # Example
///
/// ```
/// use lona_core::symbol::Interner;
/// use lonala_compiler::compile;
///
/// let mut interner = Interner::new();
/// let chunk = compile("(+ 1 2)", &mut interner).unwrap();
/// ```
#[inline]
pub fn compile(source: &str, interner: &mut symbol::Interner) -> Result<Chunk, CompileError> {
    let exprs = lonala_parser::parse(source)?;
    let compiler = Compiler::new(interner);
    let chunk = compiler.compile_program(&exprs)?;
    Ok(chunk)
}
