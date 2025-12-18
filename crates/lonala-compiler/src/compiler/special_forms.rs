// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Special form compilation.
//!
//! This module handles compilation of structural special forms:
//! - `do` - sequential evaluation
//! - `if` - conditional branching
//! - `def` - global variable definition
//! - `let` - local variable binding
//! - `quote` - datum quoting

use alloc::vec::Vec;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abc, encode_abx, encode_asbx};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind};

impl Compiler<'_, '_, '_> {
    /// Compiles a `do` special form.
    ///
    /// Syntax: `(do)` or `(do expr1 expr2 ... exprN)`
    ///
    /// Evaluates expressions left to right and returns the value of the last
    /// expression. Empty `(do)` returns nil.
    pub(super) fn compile_do(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
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
        let last_expr = args
            .last()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        self.compile_expr(last_expr)
    }

    /// Compiles an `if` special form.
    ///
    /// Syntax: `(if test then)` or `(if test then else)`
    ///
    /// Evaluates `test`. If truthy (not nil or false), evaluates and returns
    /// `then`. Otherwise evaluates and returns `else` (or nil if no else).
    pub(super) fn compile_if(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Validate: need 2 or 3 args (test, then, [else])
        if args.len() < 2_usize || args.len() > 3_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "if",
                    message: "expected (if test then) or (if test then else)",
                },
                self.location(span),
            ));
        }

        let test_expr = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let then_expr = args
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
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
        let else_offset_i16 = i16::try_from(else_offset)
            .map_err(|_err| Error::new(ErrorKind::JumpTooLarge, self.location(span)))?;
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
        let end_offset_i16 = i16::try_from(end_offset)
            .map_err(|_err| Error::new(ErrorKind::JumpTooLarge, self.location(span)))?;
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
    pub(super) fn compile_def(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Validate: need exactly 2 args (name, value)
        if args.len() != 2_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "def",
                    message: "expected (def name value)",
                },
                self.location(span),
            ));
        }

        // First arg must be a symbol
        let name_expr = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let Ast::Symbol(ref name) = name_expr.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "def",
                    message: "first argument must be a symbol",
                },
                self.location(name_expr.span),
            ));
        };

        let value_expr = args
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        // Compile value expression
        let checkpoint = self.next_register;
        let value_result = self.compile_expr(value_expr)?;

        // Intern the symbol and add to constants
        let symbol_id = self.interner.intern(name);
        let symbol_const = self.add_constant(Constant::Symbol(symbol_id), span)?;

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

    /// Compiles a `let` special form.
    ///
    /// Syntax: `(let [name1 val1 name2 val2 ...] body...)`
    ///
    /// Bindings are evaluated left to right, and each binding can reference
    /// previous bindings. Body expressions are evaluated with bindings in scope,
    /// and the value of the last body expression is returned.
    pub(super) fn compile_let(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Need at least bindings vector
        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "let",
                    message: "expected (let [bindings...] body...)",
                },
                self.location(span),
            ));
        }

        // First arg must be a vector of bindings
        let bindings_ast = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let Ast::Vector(ref bindings) = bindings_ast.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "let",
                    message: "first argument must be a vector of bindings",
                },
                self.location(bindings_ast.span),
            ));
        };

        // Bindings must come in pairs
        if bindings.len() % 2_usize != 0_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "let",
                    message: "bindings must be pairs of [name value ...]",
                },
                self.location(bindings_ast.span),
            ));
        }

        let body = args.get(1_usize..).unwrap_or(&[]);

        // Save register state for cleanup
        let checkpoint = self.next_register;

        // Push new scope
        self.locals.push_scope();

        // Process bindings in pairs
        let mut binding_idx: usize = 0;
        while binding_idx < bindings.len() {
            let name_ast = bindings
                .get(binding_idx)
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
            let value_ast = bindings
                .get(binding_idx.saturating_add(1))
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

            // Name must be a symbol
            let Ast::Symbol(ref name) = name_ast.node else {
                return Err(Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "let",
                        message: "binding name must be a symbol",
                    },
                    self.location(name_ast.span),
                ));
            };

            // Allocate register for this binding
            let reg = self.alloc_register(value_ast.span)?;

            // Compile value into the register
            let value_result = self.compile_expr(value_ast)?;
            if value_result.register != reg {
                self.chunk.emit(
                    encode_abc(Opcode::Move, reg, value_result.register, 0),
                    value_ast.span,
                );
            }

            // Free any temps but keep the binding register
            self.free_registers_to(reg.saturating_add(1));

            // Register the binding
            let symbol_id = self.interner.intern(name);
            self.locals.define(symbol_id, reg);

            binding_idx = binding_idx.saturating_add(2);
        }

        // Compile body (like do)
        let result = if body.is_empty() {
            // Empty body returns nil
            let dest = self.alloc_register(span)?;
            self.chunk
                .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
            ExprResult { register: dest }
        } else {
            // Compile all but last expression, discarding results
            for expr in body.get(..body.len().saturating_sub(1)).unwrap_or(&[]) {
                let temp_checkpoint = self.next_register;
                let _temp_result = self.compile_expr(expr)?;
                self.free_registers_to(temp_checkpoint);
            }
            // Compile last expression to return
            let last = body
                .last()
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
            self.compile_expr(last)?
        };

        // Pop scope and restore registers
        self.locals.pop_scope();
        self.free_registers_to(checkpoint);

        // Move result to checkpoint register if needed
        let dest = self.alloc_register(span)?;
        if result.register != dest {
            self.chunk
                .emit(encode_abc(Opcode::Move, dest, result.register, 0), span);
        }

        Ok(ExprResult { register: dest })
    }

    /// Compiles a `quote` special form.
    ///
    /// Syntax: `(quote datum)`
    ///
    /// Returns the datum as a value without evaluating it.
    pub(super) fn compile_quote(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        if args.len() != 1_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "quote",
                    message: "expected (quote datum)",
                },
                self.location(span),
            ));
        }

        let datum = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let constant = self.ast_to_constant(datum)?;
        let const_idx = self.add_constant(constant, datum.span)?;
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Converts an AST node to a compile-time constant.
    ///
    /// Used by `quote` to convert the quoted datum to a constant value.
    pub(super) fn ast_to_constant(&mut self, ast: &Spanned<Ast>) -> Result<Constant, Error> {
        match ast.node {
            Ast::Nil => Ok(Constant::Nil),
            Ast::Bool(bool_val) => Ok(Constant::Bool(bool_val)),
            Ast::Integer(num) => Ok(Constant::Integer(num)),
            Ast::Float(num) => Ok(Constant::Float(num)),
            Ast::String(ref text) => {
                Ok(Constant::String(alloc::string::String::from(text.as_str())))
            }
            Ast::Symbol(ref name) => {
                let id = self.interner.intern(name);
                Ok(Constant::Symbol(id))
            }
            Ast::Keyword(ref name) => {
                // Keywords are stored as symbols with a : prefix
                let keyword_name = alloc::format!(":{name}");
                let id = self.interner.intern(&keyword_name);
                Ok(Constant::Symbol(id))
            }
            Ast::List(ref elements) => {
                let constants: Result<Vec<Constant>, Error> = elements
                    .iter()
                    .map(|elem| self.ast_to_constant(elem))
                    .collect();
                Ok(Constant::List(constants?))
            }
            Ast::Vector(ref elements) => {
                let constants: Result<Vec<Constant>, Error> = elements
                    .iter()
                    .map(|elem| self.ast_to_constant(elem))
                    .collect();
                Ok(Constant::Vector(constants?))
            }
            Ast::Map(_) => Err(Error::new(
                ErrorKind::NotImplemented {
                    feature: "quoted maps",
                },
                self.location(ast.span),
            )),
            // Ast is non-exhaustive, handle future variants
            _ => Err(Error::new(
                ErrorKind::NotImplemented {
                    feature: "unknown AST node in quote",
                },
                self.location(ast.span),
            )),
        }
    }
}
