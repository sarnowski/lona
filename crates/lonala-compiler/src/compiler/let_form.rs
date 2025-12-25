// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Let form compilation.
//!
//! This module handles compilation of the `let` special form, including:
//! - Local variable bindings
//! - Destructuring patterns (sequential and map)
//! - Body expression compilation

use lona_core::opcode::{Opcode, encode_abc};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind};

impl Compiler<'_, '_, '_> {
    /// Compiles a `let` special form.
    ///
    /// Syntax: `(let [name1 val1 name2 val2 ...] body...)`
    ///
    /// Bindings are evaluated left to right, and each binding can reference
    /// previous bindings. Body expressions are evaluated with bindings in scope,
    /// and the value of the last body expression is returned.
    ///
    /// For tail call optimization: binding values are never in tail position,
    /// but the body (specifically its last expression) inherits tail position.
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

        // Save tail position - body inherits it, but bindings do not
        let is_tail = self.in_tail_position;

        let body = args.get(1_usize..).unwrap_or(&[]);
        let checkpoint = self.next_register;
        self.locals.push_scope();

        // Process all binding pairs (NOT in tail position)
        self.with_tail_position(false, |compiler| {
            compiler.compile_let_bindings(bindings, span)
        })?;

        // Compile body and return result (inherits tail position)
        let result =
            self.with_tail_position(is_tail, |compiler| compiler.compile_body(body, span))?;

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

    /// Processes binding pairs in a let form.
    pub(super) fn compile_let_bindings(
        &mut self,
        bindings: &[Spanned<Ast>],
        span: Span,
    ) -> Result<(), Error> {
        let mut binding_idx: usize = 0;
        while binding_idx < bindings.len() {
            let name_ast = bindings
                .get(binding_idx)
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
            let value_ast = bindings
                .get(binding_idx.saturating_add(1))
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

            let value_result = self.compile_expr(value_ast)?;
            self.compile_binding_target(name_ast, value_result.register, value_ast.span)?;

            binding_idx = binding_idx.saturating_add(2);
        }
        Ok(())
    }

    /// Compiles a single binding target (symbol or destructuring pattern).
    pub(super) fn compile_binding_target(
        &mut self,
        name_ast: &Spanned<Ast>,
        value_reg: u8,
        value_span: Span,
    ) -> Result<(), Error> {
        match name_ast.node {
            Ast::Symbol(ref name) => {
                let reg = self.alloc_register(value_span)?;
                if value_reg != reg {
                    self.chunk
                        .emit(encode_abc(Opcode::Move, reg, value_reg, 0), value_span);
                }
                self.free_registers_to(reg.saturating_add(1));
                let symbol_id = self.interner.intern(name);
                self.locals.define(symbol_id, reg);
            }
            Ast::Vector(_) => {
                let pattern = super::destructure::parse_sequential_pattern(
                    self.interner,
                    name_ast,
                    self.source_id,
                    0, // Initial depth
                )?;
                self.compile_sequential_binding(&pattern, value_reg, name_ast.span)?;
            }
            Ast::Map(_) => {
                let pattern = super::destructure::parse_map_pattern(
                    self.interner,
                    name_ast,
                    self.source_id,
                    0, // Initial depth
                )?;
                self.compile_map_binding(&pattern, value_reg, name_ast.span)?;
            }
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Set(_)
            | Ast::WithMeta { .. }
            | _ => {
                return Err(Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "let",
                        message: "binding target must be a symbol, vector, or map pattern",
                    },
                    self.location(name_ast.span),
                ));
            }
        }
        Ok(())
    }

    /// Compiles a body (sequence of expressions), returning the last result or nil.
    ///
    /// For tail call optimization: all but the last expression are compiled with
    /// `in_tail_position = false`. The last expression inherits the current
    /// tail position setting.
    pub(super) fn compile_body(
        &mut self,
        body: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        if body.is_empty() {
            let dest = self.alloc_register(span)?;
            self.chunk
                .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
            return Ok(ExprResult { register: dest });
        }

        // Save tail position - only last expression inherits it
        let is_tail = self.in_tail_position;

        // Compile all but last expression NOT in tail position
        for expr in body.get(..body.len().saturating_sub(1)).unwrap_or(&[]) {
            let temp_checkpoint = self.next_register;
            let _temp_result =
                self.with_tail_position(false, |compiler| compiler.compile_expr(expr))?;
            self.free_registers_to(temp_checkpoint);
        }

        // Compile last expression (inherits tail position)
        let last = body
            .last()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        self.with_tail_position(is_tail, |compiler| compiler.compile_expr(last))
    }
}
