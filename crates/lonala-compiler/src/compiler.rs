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
use crate::opcode::{Opcode, RK_MAX_CONSTANT, encode_abc, encode_abx, rk_constant, rk_register};

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

            // Not yet implemented
            Ast::String(ref _str_val) => Err(Error::NotImplemented {
                feature: "string literals",
                span: expr.span,
            }),
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

    /// Compiles a list as a function call or arithmetic operation.
    fn compile_list(&mut self, elements: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        if elements.is_empty() {
            return Err(Error::EmptyCall { span });
        }

        // Check if the first element is a known operator symbol
        if let Some(spanned_func) = elements.first()
            && let Ast::Symbol(ref name) = spanned_func.node
        {
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

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use crate::opcode::{decode_a, decode_b, decode_bx, decode_c, decode_op};

    /// Helper to compile source and return the chunk.
    fn compile_source(source: &str) -> Chunk {
        let mut interner = symbol::Interner::new();
        compile(source, &mut interner).expect("compilation should succeed")
    }

    /// Helper to compile and return chunk + interner for symbol checks.
    fn compile_with_interner(source: &str) -> (Chunk, symbol::Interner) {
        let mut interner = symbol::Interner::new();
        let chunk = compile(source, &mut interner).expect("compilation should succeed");
        (chunk, interner)
    }

    // =========================================================================
    // Literal Compilation Tests
    // =========================================================================

    #[test]
    fn compile_integer() {
        let chunk = compile_source("42");
        let code = chunk.code();

        // Should have: LoadK R0, K0; Return R0, 1
        assert_eq!(code.len(), 2);

        // LoadK instruction
        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
        assert_eq!(decode_a(instr0), 0);
        let k_idx = decode_bx(instr0);
        assert_eq!(chunk.get_constant(k_idx), Some(&Constant::Integer(42)));

        // Return instruction
        let instr1 = *code.get(1_usize).unwrap();
        assert_eq!(decode_op(instr1), Some(Opcode::Return));
        assert_eq!(decode_a(instr1), 0);
        assert_eq!(decode_b(instr1), 1);
    }

    #[test]
    fn compile_float() {
        let chunk = compile_source("3.14");
        let code = chunk.code();

        assert_eq!(code.len(), 2);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
        let k_idx = decode_bx(instr0);
        assert_eq!(chunk.get_constant(k_idx), Some(&Constant::Float(3.14)));
    }

    #[test]
    fn compile_true() {
        let chunk = compile_source("true");
        let code = chunk.code();

        assert_eq!(code.len(), 2);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::LoadTrue));
        assert_eq!(decode_a(instr0), 0);
    }

    #[test]
    fn compile_false() {
        let chunk = compile_source("false");
        let code = chunk.code();

        assert_eq!(code.len(), 2);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::LoadFalse));
    }

    #[test]
    fn compile_nil() {
        let chunk = compile_source("nil");
        let code = chunk.code();

        assert_eq!(code.len(), 2);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::LoadNil));
    }

    // =========================================================================
    // Symbol Compilation Tests
    // =========================================================================

    #[test]
    fn compile_symbol_global_lookup() {
        let (chunk, interner) = compile_with_interner("foo");
        let code = chunk.code();

        // GetGlobal R0, K0 (where K0 is sym#foo)
        assert_eq!(code.len(), 2);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::GetGlobal));
        assert_eq!(decode_a(instr0), 0);

        let k_idx = decode_bx(instr0);
        if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(k_idx) {
            assert_eq!(interner.resolve(*sym_id), "foo");
        } else {
            panic!("expected Symbol constant");
        }
    }

    // =========================================================================
    // Arithmetic Compilation Tests
    // =========================================================================

    #[test]
    fn compile_addition() {
        let chunk = compile_source("(+ 1 2)");
        let code = chunk.code();

        // Add R0, K0, K1; Return R0, 1
        assert_eq!(code.len(), 2);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Add));
        assert_eq!(decode_a(instr0), 0);

        // Verify constants
        assert_eq!(chunk.get_constant(0), Some(&Constant::Integer(1)));
        assert_eq!(chunk.get_constant(1), Some(&Constant::Integer(2)));
    }

    #[test]
    fn compile_subtraction() {
        let chunk = compile_source("(- 10 3)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Sub));
    }

    #[test]
    fn compile_multiplication() {
        let chunk = compile_source("(* 4 5)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Mul));
    }

    #[test]
    fn compile_division() {
        let chunk = compile_source("(/ 20 4)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Div));
    }

    #[test]
    fn compile_modulo() {
        let chunk = compile_source("(mod 10 3)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Mod));
    }

    #[test]
    fn compile_nested_arithmetic() {
        let chunk = compile_source("(+ (* 2 3) 4)");
        let code = chunk.code();

        // Mul R0, K0, K1 (2 * 3)
        // Add R0, R0, K2 (result + 4)
        // Return R0, 1
        assert_eq!(code.len(), 3);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Mul));

        let instr1 = *code.get(1_usize).unwrap();
        assert_eq!(decode_op(instr1), Some(Opcode::Add));

        // Verify constants
        assert_eq!(chunk.get_constant(0), Some(&Constant::Integer(2)));
        assert_eq!(chunk.get_constant(1), Some(&Constant::Integer(3)));
        assert_eq!(chunk.get_constant(2), Some(&Constant::Integer(4)));
    }

    // =========================================================================
    // Function Call Tests
    // =========================================================================

    #[test]
    fn compile_function_call() {
        let (chunk, interner) = compile_with_interner("(print 42)");
        let code = chunk.code();

        // GetGlobal R0, K0 (print)
        // LoadK R1, K1 (42)
        // Call R0, 1, 1
        // Return R0, 1
        assert_eq!(code.len(), 4);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::GetGlobal));

        let instr1 = *code.get(1_usize).unwrap();
        assert_eq!(decode_op(instr1), Some(Opcode::LoadK));

        let instr2 = *code.get(2_usize).unwrap();
        assert_eq!(decode_op(instr2), Some(Opcode::Call));
        assert_eq!(decode_a(instr2), 0); // base register
        assert_eq!(decode_b(instr2), 1); // 1 argument
        assert_eq!(decode_c(instr2), 1); // 1 result

        // Verify symbol constant
        if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(0) {
            assert_eq!(interner.resolve(*sym_id), "print");
        } else {
            panic!("expected Symbol constant");
        }
    }

    #[test]
    fn compile_print_addition() {
        let (chunk, interner) = compile_with_interner("(print (+ 1 2))");
        let code = chunk.code();

        // GetGlobal R0, K0 (print)
        // Add R1, K1, K2 (1 + 2)
        // Call R0, 1, 1
        // Return R0, 1
        assert_eq!(code.len(), 4);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::GetGlobal));

        let instr1 = *code.get(1_usize).unwrap();
        assert_eq!(decode_op(instr1), Some(Opcode::Add));
        assert_eq!(decode_a(instr1), 1); // result in R1

        let instr2 = *code.get(2_usize).unwrap();
        assert_eq!(decode_op(instr2), Some(Opcode::Call));

        // Verify print symbol
        if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(0) {
            assert_eq!(interner.resolve(*sym_id), "print");
        } else {
            panic!("expected Symbol constant");
        }
    }

    // =========================================================================
    // Error Tests
    // =========================================================================

    #[test]
    fn compile_empty_call_error() {
        let mut interner = symbol::Interner::new();
        let result = compile("()", &mut interner);
        assert!(result.is_err());

        if let Err(CompileError::Compile(Error::EmptyCall { .. })) = result {
            // Expected
        } else {
            panic!("expected EmptyCall error");
        }
    }

    #[test]
    fn compile_string_not_implemented() {
        let mut interner = symbol::Interner::new();
        let result = compile("\"hello\"", &mut interner);
        assert!(result.is_err());

        if let Err(CompileError::Compile(Error::NotImplemented { feature, .. })) = result {
            assert_eq!(feature, "string literals");
        } else {
            panic!("expected NotImplemented error");
        }
    }

    #[test]
    fn compile_keyword_not_implemented() {
        let mut interner = symbol::Interner::new();
        let result = compile(":keyword", &mut interner);
        assert!(result.is_err());

        if let Err(CompileError::Compile(Error::NotImplemented { feature, .. })) = result {
            assert_eq!(feature, "keyword literals");
        } else {
            panic!("expected NotImplemented error");
        }
    }

    #[test]
    fn compile_vector_not_implemented() {
        let mut interner = symbol::Interner::new();
        let result = compile("[1 2 3]", &mut interner);
        assert!(result.is_err());

        if let Err(CompileError::Compile(Error::NotImplemented { feature, .. })) = result {
            assert_eq!(feature, "vector literals");
        } else {
            panic!("expected NotImplemented error");
        }
    }

    // =========================================================================
    // Chunk Metadata Tests
    // =========================================================================

    #[test]
    fn compile_tracks_max_registers() {
        let chunk = compile_source("(print (+ 1 2))");
        // Uses R0 for print, R1 for add result
        assert!(chunk.max_registers() >= 2);
    }

    // =========================================================================
    // Disassembly Tests (for debugging)
    // =========================================================================

    #[test]
    fn disassemble_print_add() {
        let chunk = compile_source("(print (+ 1 2))");
        let disasm = chunk.disassemble();

        // Verify key parts are present in disassembly
        assert!(disasm.contains("GetGlobal"));
        assert!(disasm.contains("Add"));
        assert!(disasm.contains("Call"));
        assert!(disasm.contains("Return"));
    }

    // =========================================================================
    // Multiple Expression Tests
    // =========================================================================

    #[test]
    fn compile_multiple_expressions() {
        let chunk = compile_source("1 2 3");
        let code = chunk.code();

        // Each expression resets registers, so:
        // LoadK R0, K0 (1)
        // LoadK R0, K1 (2)
        // LoadK R0, K2 (3)
        // Return R0, 1
        assert_eq!(code.len(), 4);

        // Last instruction is Return with R0
        let last = *code.get(3_usize).unwrap();
        assert_eq!(decode_op(last), Some(Opcode::Return));
        assert_eq!(decode_a(last), 0);
    }

    #[test]
    fn compile_empty_program() {
        let chunk = compile_source("");
        let code = chunk.code();

        // Empty program: LoadNil R0; Return R0, 1
        assert_eq!(code.len(), 2);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::LoadNil));

        // Verify max_registers is correctly set for frame allocation
        assert_eq!(chunk.max_registers(), 1);
    }

    // =========================================================================
    // Comparison Operators Tests
    // =========================================================================

    #[test]
    fn compile_equality() {
        let chunk = compile_source("(= 1 1)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Eq));
    }

    #[test]
    fn compile_less_than() {
        let chunk = compile_source("(< 1 2)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Lt));
    }

    #[test]
    fn compile_greater_than() {
        let chunk = compile_source("(> 2 1)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Gt));
    }

    #[test]
    fn compile_less_than_or_equal() {
        let chunk = compile_source("(<= 2 2)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Le));
    }

    #[test]
    fn compile_greater_than_or_equal() {
        let chunk = compile_source("(>= 3 2)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Ge));
    }

    // =========================================================================
    // Unary Operators Tests
    // =========================================================================

    #[test]
    fn compile_unary_negation() {
        let chunk = compile_source("(- 5)");
        let code = chunk.code();

        // LoadK R0, K0 (5)
        // Neg R0, R0
        // Return R0, 1
        assert_eq!(code.len(), 3);

        let instr1 = *code.get(1_usize).unwrap();
        assert_eq!(decode_op(instr1), Some(Opcode::Neg));
    }

    #[test]
    fn compile_unary_negation_expression() {
        let chunk = compile_source("(- (+ 1 2))");
        let code = chunk.code();

        // Add R0, K0, K1 (1 + 2)
        // Neg R0, R0
        // Return R0, 1
        assert_eq!(code.len(), 3);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Add));

        let instr1 = *code.get(1_usize).unwrap();
        assert_eq!(decode_op(instr1), Some(Opcode::Neg));
    }

    #[test]
    fn compile_not_operator() {
        let chunk = compile_source("(not true)");
        let code = chunk.code();

        // LoadTrue R0
        // Not R0, R0
        // Return R0, 1
        assert_eq!(code.len(), 3);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::LoadTrue));

        let instr1 = *code.get(1_usize).unwrap();
        assert_eq!(decode_op(instr1), Some(Opcode::Not));
    }

    #[test]
    fn compile_not_false() {
        let chunk = compile_source("(not false)");
        let code = chunk.code();

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::LoadFalse));

        let instr1 = *code.get(1_usize).unwrap();
        assert_eq!(decode_op(instr1), Some(Opcode::Not));
    }

    #[test]
    fn compile_not_with_comparison() {
        let chunk = compile_source("(not (= 1 2))");
        let code = chunk.code();

        // Eq R0, K0, K1 (1 = 2)
        // Not R0, R0
        // Return R0, 1
        assert_eq!(code.len(), 3);

        let instr0 = *code.get(0_usize).unwrap();
        assert_eq!(decode_op(instr0), Some(Opcode::Eq));

        let instr1 = *code.get(1_usize).unwrap();
        assert_eq!(decode_op(instr1), Some(Opcode::Not));
    }

    // =========================================================================
    // CompileError Tests
    // =========================================================================

    #[test]
    fn compile_error_display() {
        let err = CompileError::Compile(Error::EmptyCall {
            span: Span::new(0_usize, 2_usize),
        });
        let msg = alloc::format!("{}", err);
        assert!(msg.contains("empty list"));
    }

    #[test]
    fn compile_error_from_parse() {
        let mut interner = symbol::Interner::new();
        let result = compile("(unclosed", &mut interner);
        assert!(matches!(result, Err(CompileError::Parse(_))));
    }
}
