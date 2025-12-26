// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Function call and macro call compilation.
//!
//! This module handles compilation of:
//! - General function calls
//! - Macro calls (compile-time expansion)
//!
//! Operator compilation is in `operators.rs` and `nary.rs`.

use alloc::vec::Vec;

use lona_core::opcode::{Opcode, encode_abc};
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
                "var" => return self.compile_var(args, span),
                "case" => return self.compile_case(args, span),
                "ns" => return self.compile_ns(args, span),
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

    /// Compiles a general function call.
    ///
    /// If the call is in tail position (`self.in_tail_position` is true), emits
    /// `TailCall` opcode instead of `Call`. This enables tail call optimization
    /// in the VM, allowing recursive functions to run in constant stack space.
    pub(super) fn compile_call(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let func_expr = elements
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let args = elements.get(1_usize..).unwrap_or(&[]);

        // Remember if we're in tail position before compiling subexpressions
        let is_tail = self.in_tail_position;

        // Allocate contiguous registers: R_base = func, R_base+1..N = args
        let base = self.next_register;

        // Compile function and arguments NOT in tail position
        // (only the call itself can be a tail call, not its subexpressions)
        let func_result =
            self.with_tail_position(false, |compiler| compiler.compile_expr(func_expr))?;

        // Ensure function is at base (should be since we just allocated)
        if func_result.register != base {
            // Move to base if needed (shouldn't happen with current design)
            self.chunk.emit(
                encode_abc(Opcode::Move, base, func_result.register, 0),
                func_expr.span,
            );
        }

        // Compile arguments into consecutive registers (not in tail position)
        for arg in args {
            let _arg_result =
                self.with_tail_position(false, |compiler| compiler.compile_expr(arg))?;
            // Arguments are automatically placed in consecutive registers
        }

        // Emit call instruction
        let arg_count = u8::try_from(args.len())
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;

        // Use TailCall if in tail position, otherwise regular Call
        let opcode = if is_tail {
            Opcode::TailCall
        } else {
            Opcode::Call
        };
        self.chunk
            .emit(encode_abc(opcode, base, arg_count, 1), span);

        // Result is left in base register
        // Free argument registers
        self.free_registers_to(base.saturating_add(1));

        Ok(ExprResult { register: base })
    }
}
