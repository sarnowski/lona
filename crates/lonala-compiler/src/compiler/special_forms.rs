// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Special form compilation.
//!
//! This module handles compilation of structural special forms:
//! - `do` - sequential evaluation
//! - `if` - conditional branching
//! - `def` - global variable definition
//! - `var` - var reference
//!
//! The `let` form is in `let_form.rs` and `quote` is in `quote.rs`.

use alloc::vec::Vec;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abc, encode_abx, encode_asbx};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind};

/// Parsed arguments from a `def` special form.
///
/// Contains: `(name, name_span, explicit_metadata, docstring, value_expr)`
type DefArgs<'args> = (
    alloc::string::String,
    Span,
    Option<&'args Spanned<Ast>>,
    Option<&'args str>,
    &'args Spanned<Ast>,
);

impl Compiler<'_, '_, '_> {
    /// Compiles a `do` special form.
    ///
    /// Syntax: `(do)` or `(do expr1 expr2 ... exprN)`
    ///
    /// Evaluates expressions left to right and returns the value of the last
    /// expression. Empty `(do)` returns nil.
    ///
    /// For tail call optimization: only the last expression inherits tail position.
    /// All preceding expressions are compiled with `in_tail_position = false`.
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

        // Save tail position - only last expression inherits it
        let is_tail = self.in_tail_position;

        // Compile all but last expression, discarding results (NOT in tail position)
        for expr in args
            .get(..args.len().saturating_sub(1_usize))
            .unwrap_or(&[])
        {
            let checkpoint = self.next_register;
            let _result = self.with_tail_position(false, |compiler| compiler.compile_expr(expr))?;
            self.free_registers_to(checkpoint);
        }

        // Compile last expression and return its result (inherits tail position)
        let last_expr = args
            .last()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        self.with_tail_position(is_tail, |compiler| compiler.compile_expr(last_expr))
    }

    /// Compiles an `if` special form.
    ///
    /// Syntax: `(if test then)` or `(if test then else)`
    ///
    /// Evaluates `test`. If truthy (not nil or false), evaluates and returns
    /// `then`. Otherwise evaluates and returns `else` (or nil if no else).
    ///
    /// For tail call optimization: the test expression is never in tail position,
    /// but both `then` and `else` branches inherit the current tail position.
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

        // Save tail position - branches inherit it, but test does not
        let is_tail = self.in_tail_position;

        // Compile test expression NOT in tail position
        let checkpoint = self.next_register;
        let test_result =
            self.with_tail_position(false, |compiler| compiler.compile_expr(test_expr))?;

        // Emit JumpIfNot (will patch offset later)
        let jump_to_else_idx = self.chunk.emit(
            encode_asbx(Opcode::JumpIfNot, test_result.register, 0),
            span,
        );

        // Free test register
        self.free_registers_to(checkpoint);

        // Allocate destination register for result
        let dest = self.alloc_register(span)?;

        // Compile then branch into dest (inherits tail position)
        let then_result =
            self.with_tail_position(is_tail, |compiler| compiler.compile_expr(then_expr))?;
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

        // Compile else branch (or nil) into dest (inherits tail position)
        if let Some(else_branch) = else_expr {
            let else_result =
                self.with_tail_position(is_tail, |compiler| compiler.compile_expr(else_branch))?;
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
    /// Syntax variations:
    /// - `(def name value)` - basic definition
    /// - `(def "docstring" name value)` - with documentation
    /// - `(def ^{:key val} name value)` - with explicit metadata
    /// - `(def ^:private name value)` - with keyword metadata shorthand
    ///
    /// All definitions automatically include source location metadata
    /// (`:line`, `:column`).
    ///
    /// Returns the symbol `name`.
    pub(super) fn compile_def(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Parse the def form arguments
        let (name, name_span, explicit_meta, docstring, value_expr) =
            self.parse_def_args(args, span)?;

        // Compile value expression
        let checkpoint = self.next_register;
        let value_result = self.compile_expr(value_expr)?;

        // Intern the symbol and add to constants
        let symbol_id = self.interner.intern(&name);
        let symbol_const = self.add_constant(Constant::Symbol(symbol_id), span)?;

        // Emit SetGlobal
        self.chunk.emit(
            encode_abx(Opcode::SetGlobal, value_result.register, symbol_const),
            span,
        );

        // Build and emit metadata
        let meta_register =
            self.build_def_metadata(name_span, explicit_meta, docstring, checkpoint)?;

        // Emit SetGlobalMeta if we have metadata
        if let Some(meta_reg) = meta_register {
            self.chunk.emit(
                encode_abx(Opcode::SetGlobalMeta, meta_reg, symbol_const),
                span,
            );
        }

        // Free temporary registers
        self.free_registers_to(checkpoint);

        // Return the symbol (load it into destination)
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, symbol_const), span);

        Ok(ExprResult { register: dest })
    }

    /// Parses `def` form arguments.
    ///
    /// Returns: `(name, name_span, explicit_meta, docstring, value_expr)`
    fn parse_def_args<'args>(
        &self,
        args: &'args [Spanned<Ast>],
        span: Span,
    ) -> Result<DefArgs<'args>, Error> {
        if args.len() < 2_usize || args.len() > 3_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "def",
                    message: "expected (def name value) or (def \"doc\" name value)",
                },
                self.location(span),
            ));
        }

        // Check for docstring: (def "doc" name value)
        if args.len() == 3_usize {
            let first = args
                .first()
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

            if let Ast::String(ref doc) = first.node {
                // Docstring form: (def "doc" name value)
                let name_expr = args
                    .get(1_usize)
                    .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
                let value_expr = args
                    .get(2_usize)
                    .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

                let (name, name_span, explicit_meta) = self.extract_def_name(name_expr)?;
                return Ok((
                    name,
                    name_span,
                    explicit_meta,
                    Some(doc.as_str()),
                    value_expr,
                ));
            }
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "def",
                    message: "expected (def name value) or (def \"doc\" name value)",
                },
                self.location(span),
            ));
        }

        // Standard form: (def name value) or (def ^meta name value)
        let name_expr = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let value_expr = args
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        let (name, name_span, explicit_meta) = self.extract_def_name(name_expr)?;
        Ok((name, name_span, explicit_meta, None, value_expr))
    }

    /// Extracts the name and optional metadata from a def name expression.
    ///
    /// Handles `name` and `^{...} name` forms.
    fn extract_def_name<'expr>(
        &self,
        expr: &'expr Spanned<Ast>,
    ) -> Result<(alloc::string::String, Span, Option<&'expr Spanned<Ast>>), Error> {
        match expr.node {
            Ast::Symbol(ref name) => Ok((name.clone(), expr.span, None)),
            Ast::WithMeta {
                ref meta,
                ref value,
            } => {
                // Extract the actual symbol from inside WithMeta
                if let Ast::Symbol(ref name) = value.node {
                    Ok((name.clone(), value.span, Some(meta.as_ref())))
                } else {
                    Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "def",
                            message: "name must be a symbol",
                        },
                        self.location(value.span),
                    ))
                }
            }
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Vector(_)
            | Ast::Map(_)
            | Ast::Set(_)
            | _ => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "def",
                    message: "name must be a symbol",
                },
                self.location(expr.span),
            )),
        }
    }

    /// Builds the metadata map for a def and loads it into a register.
    ///
    /// Combines source location, docstring, and explicit metadata.
    fn build_def_metadata(
        &mut self,
        name_span: Span,
        explicit_meta: Option<&Spanned<Ast>>,
        docstring: Option<&str>,
        checkpoint: u8,
    ) -> Result<Option<u8>, Error> {
        // Build metadata pairs
        let mut meta_pairs: Vec<(Constant, Constant)> = Vec::new();

        // Add source location: :line, :column
        // TODO: compute proper line/column from source registry when available.
        // For now, use placeholder values. Using byte offset as column would be
        // semantically incorrect and produce confusing debug info.
        let line = 1_i64;
        let column = 1_i64;

        let line_kw = self.interner.intern(":line");
        let col_kw = self.interner.intern(":column");
        meta_pairs.push((Constant::Symbol(line_kw), Constant::Integer(line)));
        meta_pairs.push((Constant::Symbol(col_kw), Constant::Integer(column)));

        // Add docstring if present
        if let Some(doc) = docstring {
            let doc_kw = self.interner.intern(":doc");
            meta_pairs.push((
                Constant::Symbol(doc_kw),
                Constant::String(alloc::string::String::from(doc)),
            ));
        }

        // Add explicit metadata from ^{...}
        if let Some(meta_ast) = explicit_meta {
            self.merge_ast_metadata_into(&mut meta_pairs, meta_ast)?;
        }

        // If no metadata, return None
        if meta_pairs.is_empty() {
            return Ok(None);
        }

        // Load the metadata map into a register
        let meta_const = Constant::Map(meta_pairs);
        let meta_const_idx = self.add_constant(meta_const, name_span)?;

        self.free_registers_to(checkpoint);
        let meta_reg = self.alloc_register(name_span)?;
        self.chunk.emit(
            encode_abx(Opcode::LoadK, meta_reg, meta_const_idx),
            name_span,
        );

        Ok(Some(meta_reg))
    }

    /// Merges AST metadata (from `^{...}`) into the metadata pairs.
    fn merge_ast_metadata_into(
        &mut self,
        pairs: &mut Vec<(Constant, Constant)>,
        meta_ast: &Spanned<Ast>,
    ) -> Result<(), Error> {
        // Meta should be a Map
        let Ast::Map(ref elements) = meta_ast.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "def",
                    message: "metadata must be a map",
                },
                self.location(meta_ast.span),
            ));
        };

        // Convert each pair
        if elements.len() % 2_usize != 0 {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "def",
                    message: "metadata map must have even number of elements",
                },
                self.location(meta_ast.span),
            ));
        }

        for chunk in elements.chunks_exact(2) {
            let key =
                self.ast_to_constant(chunk.first().ok_or_else(|| {
                    Error::new(ErrorKind::EmptyCall, self.location(meta_ast.span))
                })?)?;
            let val =
                self.ast_to_constant(chunk.get(1_usize).ok_or_else(|| {
                    Error::new(ErrorKind::EmptyCall, self.location(meta_ast.span))
                })?)?;
            pairs.push((key, val));
        }

        Ok(())
    }

    /// Compiles a `var` special form.
    ///
    /// Syntax: `(var name)` or reader macro `#'name`
    ///
    /// Returns the Var itself (not its value), enabling metadata access.
    pub(super) fn compile_var(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        if args.len() != 1_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "var",
                    message: "expected (var name)",
                },
                self.location(span),
            ));
        }

        let name_expr = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let Ast::Symbol(ref name) = name_expr.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "var",
                    message: "argument must be a symbol",
                },
                self.location(name_expr.span),
            ));
        };

        let symbol_id = self.interner.intern(name);
        let symbol_const = self.add_constant(Constant::Symbol(symbol_id), span)?;

        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobalVar, dest, symbol_const), span);

        Ok(ExprResult { register: dest })
    }
}
