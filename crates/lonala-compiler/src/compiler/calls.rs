// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Function call and operator compilation.
//!
//! This module handles compilation of:
//! - General function calls
//! - Macro calls (compile-time expansion)
//! - Binary operators (arithmetic and comparison)
//! - Unary operators (not, negation)

use alloc::vec::Vec;

use lona_core::chunk::Constant;
use lona_core::opcode::{
    Opcode, RK_MAX_CONSTANT, encode_abc, encode_abx, encode_asbx, rk_constant, rk_register,
};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::conversion;
use super::{Compiler, ExprResult, MAX_MACRO_EXPANSION_DEPTH};
use crate::error::{Error, Kind as ErrorKind};

impl Compiler<'_, '_, '_> {
    /// Compiles a list as a function call, special form, or arithmetic operation.
    pub(super) fn compile_list(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        if elements.is_empty() {
            return Err(Error::new(ErrorKind::EmptyCall, self.location(span)));
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
                "let" => return self.compile_let(args, span),
                "quote" => return self.compile_quote(args, span),
                "syntax-quote" => return self.compile_syntax_quote(args, span),
                "unquote" => {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "unquote",
                            message: "unquote (~) not inside syntax-quote (`)",
                        },
                        self.location(span),
                    ));
                }
                "unquote-splicing" => {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "unquote-splicing",
                            message: "unquote-splicing (~@) not inside syntax-quote (`)",
                        },
                        self.location(span),
                    ));
                }
                "fn" => return self.compile_fn(args, span),
                "defmacro" => return self.compile_defmacro(args, span),
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

            // Check for macro call - only if we have an expander to actually expand it
            // Without an expander, macro calls are treated as regular (undefined) function calls
            if self.expander.is_some() {
                let sym_id = self.interner.intern(name);
                if self.registry.contains(sym_id) {
                    return self.compile_macro_call(sym_id, args, span);
                }
            }
        }

        // General function call
        self.compile_call(elements, span)
    }

    /// Compiles a macro call by expanding and then compiling the result.
    ///
    /// This method is only called when an expander is available (checked by
    /// `compile_list` before calling this method). The macro transformer is
    /// executed at compile time to produce the expanded form.
    ///
    /// # Expansion Depth
    ///
    /// The compiler tracks macro expansion depth to prevent infinite recursion.
    /// If a macro expands to code that calls itself (directly or indirectly),
    /// the depth will eventually exceed `MAX_MACRO_EXPANSION_DEPTH` and
    /// compilation will fail with `Error::MacroExpansionDepthExceeded`.
    pub(super) fn compile_macro_call(
        &mut self,
        macro_name: lona_core::symbol::Id,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Check expansion depth before proceeding
        if self.macro_expansion_depth >= MAX_MACRO_EXPANSION_DEPTH {
            return Err(Error::new(
                ErrorKind::MacroExpansionDepthExceeded {
                    depth: self.macro_expansion_depth,
                },
                self.location(span),
            ));
        }

        // Get the macro definition
        let macro_def = self
            .registry
            .get(macro_name)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "macro",
                        message: "macro not found in registry",
                    },
                    self.location(span),
                )
            })?
            .clone();

        // Get the expander (we know it exists because compile_list checked)
        let Some(ref mut expander) = self.expander else {
            // This should not happen - compile_list only calls us with an expander
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "macro",
                    message: "internal error: macro expansion without expander",
                },
                self.location(span),
            ));
        };

        // Convert AST arguments to Values
        let value_args: Vec<lona_core::value::Value> = args
            .iter()
            .map(|arg| conversion::ast_to_value(arg, self.interner))
            .collect();

        // Run the macro transformer
        let expanded_value = expander
            .expand(&macro_def, value_args, self.interner)
            .map_err(|err| {
                Error::new(
                    ErrorKind::MacroExpansionFailed {
                        message: err.message,
                    },
                    self.location(span),
                )
            })?;

        // Convert result back to AST
        let expanded_ast =
            conversion::value_to_ast(&expanded_value, self.interner, self.source_id, span)?;

        // Increment depth before recursive compilation
        self.macro_expansion_depth = self.macro_expansion_depth.saturating_add(1);

        // Recursively compile the expanded AST
        let result = self.compile_expr(&expanded_ast);

        // Decrement depth after compilation (even on error, for consistency)
        self.macro_expansion_depth = self.macro_expansion_depth.saturating_sub(1);

        result
    }

    /// Returns the opcode for a binary operator symbol, if any.
    ///
    /// Handles both arithmetic operators (`+`, `-`, `*`, `/`, `mod`) and
    /// comparison operators (`=`, `<`, `>`, `<=`, `>=`).
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
    fn compile_load_constant(
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
    fn compile_load_true(&mut self, span: Span) -> Result<ExprResult, Error> {
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
    fn compile_unary_negation(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let checkpoint = self.next_register;
        let result = self.compile_expr(arg)?;

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
    fn compile_unary_reciprocal(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        let base = self.next_register;

        // Load the `/` native function into base register
        let div_sym = self.interner.intern("/");
        let dest = self.alloc_register(span)?;
        let const_idx = self.add_constant(Constant::Symbol(div_sym), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, dest, const_idx), span);

        // Compile the argument into the next register
        let _arg_result = self.compile_expr(arg)?;

        // Emit call instruction with 1 argument
        self.chunk
            .emit(encode_abc(Opcode::Call, base, 1_u8, 1_u8), span);

        // Result is left in base register
        // Free argument registers
        self.free_registers_to(base.saturating_add(1));

        Ok(ExprResult { register: base })
    }

    /// Compiles a binary operation with exactly two arguments.
    fn compile_binary_pair(
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

    /// Compiles n-ary arithmetic by chaining binary operations left-to-right.
    fn compile_nary_arithmetic(
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
        let acc = self.alloc_register(span)?;
        self.chunk.emit(encode_abc(opcode, acc, rk_b, rk_c), span);

        // Chain remaining arguments: acc = acc op arg
        for arg in elements.get(3_usize..).unwrap_or(&[]) {
            let arg_checkpoint = self.next_register;
            let rk_arg = self.try_compile_rk_operand(arg)?;
            self.free_registers_to(arg_checkpoint);

            let rk_acc = rk_register(acc)
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            self.chunk
                .emit(encode_abc(opcode, acc, rk_acc, rk_arg), span);
        }

        Ok(ExprResult { register: acc })
    }

    /// Compiles n-ary comparison by chaining binary comparisons with short-circuit AND.
    ///
    /// `(= a b c)` compiles to: `(and (= a b) (= b c))`
    ///
    /// Strategy:
    /// 1. Evaluate first pair (a, b), check comparison
    /// 2. If false, jump to end with false result
    /// 3. For each subsequent pair (b, c), (c, d), etc:
    ///    - Reuse previous right operand as left operand
    ///    - Check comparison
    ///    - If false, jump to end
    /// 4. If all pass, result is true
    fn compile_nary_comparison(
        &mut self,
        opcode: Opcode,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let args = elements.get(1_usize..).unwrap_or(&[]);
        if args.len() < 2_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "comparison",
                    message: "requires at least two arguments",
                },
                self.location(span),
            ));
        }

        // We'll compile each argument into its own register and keep them around
        // for pairwise comparison. This uses more registers but simplifies the logic.
        let checkpoint = self.next_register;

        // Compile all arguments into consecutive registers
        let mut arg_registers = Vec::new();
        for arg in args {
            let result = self.compile_expr(arg)?;
            arg_registers.push(result.register);
        }

        // Destination register for the final boolean result
        let dest = self.alloc_register(span)?;

        // Collect jump offsets that need to be patched to the false case
        let mut false_jumps = Vec::new();

        // Compare adjacent pairs: (a, b), (b, c), (c, d), ...
        for pair in arg_registers.windows(2_usize) {
            let left_reg = *pair
                .first()
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            let right_reg = *pair
                .get(1_usize)
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;

            let rk_left = rk_register(left_reg)
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            let rk_right = rk_register(right_reg)
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;

            // Emit comparison into dest register
            self.chunk
                .emit(encode_abc(opcode, dest, rk_left, rk_right), span);

            // If comparison is false, jump to false case
            // JumpIfNot: if not R[A] then PC += sBx
            let jump_offset = self.chunk.code().len();
            // Use placeholder offset 0, will be patched later
            self.chunk
                .emit(encode_asbx(Opcode::JumpIfNot, dest, 0), span);
            false_jumps.push(jump_offset);
        }

        // All comparisons passed: load true into dest
        self.chunk
            .emit(encode_abc(Opcode::LoadTrue, dest, 0, 0), span);

        // Jump over the false case
        let skip_false_offset = self.chunk.code().len();
        self.chunk.emit(encode_asbx(Opcode::Jump, 0, 0), span);

        // Record current offset for patching false case
        let false_case_offset = self.chunk.code().len();

        // Load false into dest (jumped to from false_jumps)
        self.chunk
            .emit(encode_abc(Opcode::LoadFalse, dest, 0, 0), span);

        // End label - where skip_false should jump to
        let end_offset = self.chunk.code().len();

        // Patch all false jumps to jump to false_case_offset
        for jump_offset in false_jumps {
            let false_offset_i32 = i32::try_from(false_case_offset)
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            let jump_plus_one = i32::try_from(jump_offset.saturating_add(1))
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            let relative_jump = i16::try_from(false_offset_i32.saturating_sub(jump_plus_one))
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            self.chunk.patch(
                jump_offset,
                encode_asbx(Opcode::JumpIfNot, dest, relative_jump),
            );
        }

        // Patch skip_false to jump to end
        let end_offset_i32 = i32::try_from(end_offset)
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
        let skip_plus_one = i32::try_from(skip_false_offset.saturating_add(1))
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
        let skip_relative = i16::try_from(end_offset_i32.saturating_sub(skip_plus_one))
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
        self.chunk.patch(
            skip_false_offset,
            encode_asbx(Opcode::Jump, 0, skip_relative),
        );

        // Free temporary argument registers
        self.free_registers_to(checkpoint);

        // Move result to final register if needed
        let final_dest = self.alloc_register(span)?;
        if final_dest != dest {
            self.chunk
                .emit(encode_abc(Opcode::Move, final_dest, dest, 0), span);
        }

        Ok(ExprResult {
            register: final_dest,
        })
    }

    /// Compiles unary `not` operation.
    pub(super) fn compile_not(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

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
    pub(super) fn try_compile_rk_operand(&mut self, expr: &Spanned<Ast>) -> Result<u8, Error> {
        // Check if this can be a direct constant
        if let Some(rk) = self.try_constant_rk(expr)? {
            return Ok(rk);
        }

        // Otherwise compile to a register
        let result = self.compile_expr(expr)?;
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

    /// Compiles a general function call.
    pub(super) fn compile_call(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let func_expr = elements
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
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
        let arg_count = u8::try_from(args.len())
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;

        self.chunk
            .emit(encode_abc(Opcode::Call, base, arg_count, 1), span);

        // Result is left in base register
        // Free argument registers
        self.free_registers_to(base.saturating_add(1));

        Ok(ExprResult { register: base })
    }
}
