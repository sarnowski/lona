// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Binary and unary operator compilation.
//!
//! This module handles compilation of:
//! - Binary operators (arithmetic and comparison)
//! - Unary operators (not, negation)
//! - RK operand encoding for constants

use lona_core::chunk::Constant;
use lona_core::opcode::{
    Opcode, RK_MAX_CONSTANT, encode_abc, encode_abx, rk_constant, rk_register,
};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind};

impl Compiler<'_, '_, '_> {
    /// Returns the opcode for a binary operator symbol, if any.
    ///
    /// Handles both arithmetic operators (`+`, `-`, `*`, `/`, `mod`) and
    /// comparison operators (`=`, `<`, `>`, `<=`, `>=`).
    #[inline]
    pub(super) fn binary_opcode(name: &str) -> Option<Opcode> {
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

    /// Compiles an arithmetic or comparison operation with n-ary support.
    ///
    /// Handles all arities:
    /// - Zero args: `(+)` → 0, `(*)` → 1, `(-)` and `(/)` → error
    /// - One arg: `(+ x)` → x, `(* x)` → x, `(- x)` → negation, `(/ x)` → error
    /// - Two args: Standard binary operation
    /// - N args: Chains binary operations left-to-right
    pub(super) fn compile_binary_op(
        &mut self,
        opcode: Opcode,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg_count = elements.len().saturating_sub(1_usize);

        match (opcode, arg_count) {
            // Zero args: (+) → 0, (*) → 1
            (Opcode::Add, 0_usize) => self.compile_load_constant(Constant::Integer(0_i64), span),
            (Opcode::Mul, 0_usize) => self.compile_load_constant(Constant::Integer(1_i64), span),

            // Zero args error: (-), (/)
            (Opcode::Sub | Opcode::Div, 0_usize) => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: if opcode == Opcode::Sub { "-" } else { "/" },
                    message: "requires at least one argument",
                },
                self.location(span),
            )),

            // Comparison with 0 or 1 arg → true (vacuously, per Clojure semantics)
            (Opcode::Eq | Opcode::Lt | Opcode::Le | Opcode::Gt | Opcode::Ge, 0_usize) => {
                self.compile_load_true(span)
            }
            (Opcode::Eq | Opcode::Lt | Opcode::Le | Opcode::Gt | Opcode::Ge, 1_usize) => {
                // Evaluate the argument for side effects, then return true
                // Per Clojure: (= x) always returns true
                let arg = elements
                    .get(1_usize)
                    .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
                let checkpoint = self.next_register;
                let _result = self.compile_expr(arg)?;
                self.free_registers_to(checkpoint);
                self.compile_load_true(span)
            }

            // One arg: identity for + and *
            (Opcode::Add | Opcode::Mul, 1_usize) => self.compile_first_arg(elements, span),

            // One arg: (- x) → negation
            (Opcode::Sub, 1_usize) => self.compile_unary_negation(elements, span),

            // One arg: (/ x) → reciprocal via native function call
            (Opcode::Div, 1_usize) => self.compile_unary_reciprocal(elements, span),

            // Two args: standard binary operation
            (_, 2_usize) => self.compile_binary_pair(opcode, elements, span),

            // N args: mod requires exactly 2 arguments
            (Opcode::Mod, _) => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "mod",
                    message: "requires exactly two arguments",
                },
                self.location(span),
            )),

            // N args: comparison operators chain pairwise
            (Opcode::Eq | Opcode::Lt | Opcode::Le | Opcode::Gt | Opcode::Ge, _) => {
                self.compile_nary_comparison(opcode, elements, span)
            }

            // N args: chain arithmetic operators left-to-right
            (Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div, _) => {
                self.compile_nary_arithmetic(opcode, elements, span)
            }

            _ => Err(Error::new(
                ErrorKind::NotImplemented {
                    feature: "unknown operator arity",
                },
                self.location(span),
            )),
        }
    }

    /// Loads a constant value into a register.
    pub(super) fn compile_load_constant(
        &mut self,
        constant: Constant,
        span: Span,
    ) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let idx = self.add_constant(constant, span)?;
        self.chunk.emit(encode_abx(Opcode::LoadK, dest, idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Loads `true` into a register.
    pub(super) fn compile_load_true(&mut self, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::LoadTrue, dest, 0, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles the first argument of an operator expression.
    fn compile_first_arg(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        self.compile_expr(arg)
    }

    /// Compiles unary negation: `(- x)` → `Neg`.
    ///
    /// The operand is never in tail position since its result is used by `Neg`.
    fn compile_unary_negation(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let checkpoint = self.next_register;
        // Operand is NOT in tail position - its result is used by `Neg`
        let result = self.with_tail_position(false, |compiler| compiler.compile_expr(arg))?;

        self.free_registers_to(checkpoint);
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::Neg, dest, result.register, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles unary reciprocal: `(/ x)` → calls native `/` function.
    ///
    /// Unlike negation which has a dedicated opcode, reciprocal is implemented
    /// by calling the native `/` function with a single argument.
    ///
    /// This function call respects tail position: if `(/ x)` is in tail position,
    /// a `TailCall` opcode is emitted instead of `Call`.
    fn compile_unary_reciprocal(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        // Remember if we're in tail position
        let is_tail = self.in_tail_position;
        let base = self.next_register;

        // Load the `/` native function into base register
        let div_sym = self.interner.intern("/");
        let dest = self.alloc_register(span)?;
        let const_idx = self.add_constant(Constant::Symbol(div_sym), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, dest, const_idx), span);

        // Compile the argument into the next register (NOT in tail position)
        let _arg_result = self.with_tail_position(false, |compiler| compiler.compile_expr(arg))?;

        // Emit call instruction (TailCall if in tail position)
        let opcode = if is_tail {
            Opcode::TailCall
        } else {
            Opcode::Call
        };
        self.chunk.emit(encode_abc(opcode, base, 1_u8, 1_u8), span);

        // Result is left in base register
        // Free argument registers
        self.free_registers_to(base.saturating_add(1));

        Ok(ExprResult { register: base })
    }

    /// Compiles a binary operation with exactly two arguments.
    pub(super) fn compile_binary_pair(
        &mut self,
        opcode: Opcode,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg1 = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let arg2 = elements
            .get(2_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        let checkpoint = self.next_register;
        let rk_b = self.try_compile_rk_operand(arg1)?;
        let rk_c = self.try_compile_rk_operand(arg2)?;

        self.free_registers_to(checkpoint);
        let dest = self.alloc_register(span)?;
        self.chunk.emit(encode_abc(opcode, dest, rk_b, rk_c), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles unary `not` operation.
    ///
    /// The operand is never in tail position since its result is used by `not`.
    pub(super) fn compile_not(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        let checkpoint = self.next_register;
        // Operand is NOT in tail position - its result is used by `not`
        let result = self.with_tail_position(false, |compiler| compiler.compile_expr(arg))?;

        self.free_registers_to(checkpoint);
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::Not, dest, result.register, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Tries to compile an operand as an RK value (constant if possible).
    ///
    /// Returns the RK-encoded value (either a register index or constant index).
    ///
    /// Operands of binary operators are never in tail position, so this method
    /// compiles expressions with `in_tail_position = false`.
    pub(super) fn try_compile_rk_operand(&mut self, expr: &Spanned<Ast>) -> Result<u8, Error> {
        // Check if this can be a direct constant
        if let Some(rk) = self.try_constant_rk(expr)? {
            return Ok(rk);
        }

        // Otherwise compile to a register (operands are never in tail position)
        let result = self.with_tail_position(false, |compiler| compiler.compile_expr(expr))?;
        rk_register(result.register)
            .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(expr.span)))
    }

    /// Tries to encode an expression as a constant in an RK field.
    ///
    /// Returns `Some(rk)` if the expression is a simple constant that fits in
    /// the RK constant range (index <= 127), `None` otherwise.
    pub(super) fn try_constant_rk(&mut self, expr: &Spanned<Ast>) -> Result<Option<u8>, Error> {
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
        let idx = self.add_constant(constant, expr.span)?;
        // The index must fit since we checked above
        let idx_u8 = u8::try_from(idx).ok();
        match idx_u8 {
            Some(i) if i <= RK_MAX_CONSTANT => Ok(rk_constant(i)),
            _ => Ok(None), // Should not happen, but handle gracefully
        }
    }
}
