// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Macro definition compilation.
//!
//! This module handles compilation of `defmacro` special forms,
//! including single-arity and multi-arity macro definitions.

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lona_core::chunk::{Chunk, Constant, FunctionBodyData};
use lona_core::opcode::{Opcode, encode_abc, encode_abx};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use crate::compiler::macros::{MacroBody, MacroDefinition};
use crate::compiler::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind, SourceLocation};

impl Compiler<'_, '_, '_> {
    /// Compiles a `defmacro` special form.
    ///
    /// Supports both single-arity and multi-arity syntax:
    /// - Single: `(defmacro name [params] body...)`
    /// - Multi: `(defmacro name ([p1] b1) ([p2] b2)...)`
    ///
    /// Unlike `fn`, macros are always named and are registered globally
    /// for compile-time expansion.
    #[inline]
    pub fn compile_defmacro(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let location = self.location(span);

        // defmacro requires at least name and one body form
        if args.len() < 2 {
            return Err(Self::defmacro_error(
                "expected (defmacro name [params] body...) or (defmacro name ([p] b)...)",
                location,
            ));
        }

        // First arg must be symbol (macro name)
        let name_ast = args
            .first()
            .ok_or_else(|| Self::defmacro_error("expected macro name", location))?;
        let Ast::Symbol(ref name_ref) = name_ast.node else {
            return Err(Self::defmacro_error(
                "macro name must be a symbol",
                self.location(name_ast.span),
            ));
        };
        let name = name_ref.clone();
        let name_id = self.interner.intern(&name);
        // Check if this is single-arity or multi-arity syntax
        let second = args.get(1_usize).ok_or_else(|| {
            Self::defmacro_error(
                "expected parameter vector or arity body after macro name",
                location,
            )
        })?;

        let macro_bodies = match second.node {
            Ast::Vector(_) => {
                let body = args.get(2_usize..).unwrap_or(&[]);
                if body.is_empty() {
                    return Err(Self::defmacro_error("macro body cannot be empty", location));
                }
                alloc::vec![self.compile_macro_body(&name, second, body, span)?]
            }
            Ast::List(_) if Self::is_arity_body(&second.node) => {
                self.compile_multi_arity_macro(&name, args.get(1_usize..).unwrap_or(&[]), location)?
            }
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Symbol(_)
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Map(_)
            | _ => {
                return Err(Self::defmacro_error(
                    "expected [params] or ([params] body...) after macro name",
                    self.location(second.span),
                ));
            }
        };

        self.registry.register(
            name_id,
            MacroDefinition::new(macro_bodies.clone(), name.clone()),
        );

        // Also create a Var so metadata can be accessed via (meta (var macro-name))
        // Convert MacroBody to FunctionBodyData for storage
        let fn_bodies: Vec<FunctionBodyData> = macro_bodies
            .iter()
            .map(|mb| {
                FunctionBodyData::new(
                    Box::new((*mb.chunk).clone()),
                    mb.arity,
                    mb.has_rest,
                    Vec::new(),
                )
            })
            .collect();

        // Create function constant and load it
        let fn_const = Constant::Function {
            bodies: fn_bodies,
            name: Some(name.clone()),
        };
        let fn_const_idx = self.add_constant(fn_const, span)?;
        let fn_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, fn_reg, fn_const_idx), span);

        // Emit SetGlobal to store the macro function
        let symbol_const = self.add_constant(Constant::Symbol(name_id), span)?;
        self.chunk
            .emit(encode_abx(Opcode::SetGlobal, fn_reg, symbol_const), span);

        // Build metadata with :macro true and source location
        let mut meta_pairs: Vec<(Constant, Constant)> = Vec::new();

        // :macro true
        let macro_kw = self.interner.intern(":macro");
        meta_pairs.push((Constant::Symbol(macro_kw), Constant::Bool(true)));

        // Source location: :line, :column
        // TODO: compute proper line/column from source registry when available.
        // For now, use placeholder values. Using byte offset as column would be
        // semantically incorrect and produce confusing debug info.
        let line = 1_i64;
        let column = 1_i64;
        let line_kw = self.interner.intern(":line");
        let col_kw = self.interner.intern(":column");
        meta_pairs.push((Constant::Symbol(line_kw), Constant::Integer(line)));
        meta_pairs.push((Constant::Symbol(col_kw), Constant::Integer(column)));

        // Load metadata and emit SetGlobalMeta
        let meta_const = Constant::Map(meta_pairs);
        let meta_const_idx = self.add_constant(meta_const, span)?;
        let meta_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, meta_reg, meta_const_idx), span);
        self.chunk.emit(
            encode_abx(Opcode::SetGlobalMeta, meta_reg, symbol_const),
            span,
        );

        // Return the symbol
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, symbol_const), span);
        Ok(ExprResult { register: dest })
    }

    /// Helper for defmacro error messages.
    const fn defmacro_error(message: &'static str, location: SourceLocation) -> Error {
        Error::new(
            ErrorKind::InvalidSpecialForm {
                form: "defmacro",
                message,
            },
            location,
        )
    }

    /// Compiles multi-arity macro bodies.
    fn compile_multi_arity_macro(
        &mut self,
        name: &str,
        bodies_ast: &[Spanned<Ast>],
        location: SourceLocation,
    ) -> Result<Vec<MacroBody>, Error> {
        let mut macro_bodies = Vec::with_capacity(bodies_ast.len());
        for body_ast in bodies_ast {
            let (params, body) = Self::parse_arity_body(body_ast, self.location(body_ast.span))?;
            if body.is_empty() {
                return Err(Self::defmacro_error(
                    "macro body cannot be empty",
                    self.location(body_ast.span),
                ));
            }
            macro_bodies.push(self.compile_macro_body(name, params, body, body_ast.span)?);
        }
        let body_datas: Vec<FunctionBodyData> = macro_bodies
            .iter()
            .map(|mb| {
                FunctionBodyData::new(
                    Box::new((*mb.chunk).clone()),
                    mb.arity,
                    mb.has_rest,
                    Vec::new(),
                )
            })
            .collect();
        Self::validate_arities(&body_datas, location)?;
        Ok(macro_bodies)
    }

    /// Compiles a single macro body into a `MacroBody`.
    fn compile_macro_body(
        &mut self,
        name: &str,
        params_ast: &Spanned<Ast>,
        body: &[Spanned<Ast>],
        span: Span,
    ) -> Result<MacroBody, Error> {
        let location = self.location(span);

        // Extract parameter names
        let parsed_params = self.extract_params(params_ast).map_err(|_err| {
            Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "defmacro",
                    message: "parameters must be a vector of symbols",
                },
                self.location(params_ast.span),
            )
        })?;
        let arity = u8::try_from(parsed_params.fixed.len())
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;
        let has_rest = parsed_params.rest.is_some();

        // Create a child compiler for the macro body
        let mut macro_compiler = Compiler::new(self.interner, self.registry, self.source_id);
        macro_compiler.chunk = Chunk::with_name(alloc::format!("macro:{name}"));
        macro_compiler.chunk.set_arity(arity);
        macro_compiler.chunk.set_has_rest(has_rest);

        // Set up parameter locals
        Self::setup_params_on_compiler(&mut macro_compiler, &parsed_params, arity, location)?;

        // Compile body
        let result_reg = {
            for expr in body
                .get(..body.len().saturating_sub(1_usize))
                .unwrap_or(&[])
            {
                let checkpoint = macro_compiler.next_register;
                let _result = macro_compiler.compile_expr(expr)?;
                macro_compiler.free_registers_to(checkpoint);
            }
            let last = body
                .last()
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, location))?;
            let result = macro_compiler.compile_expr(last)?;
            result.register
        };

        // Emit return
        macro_compiler
            .chunk
            .emit(encode_abc(Opcode::Return, result_reg, 1, 0), span);
        macro_compiler.locals.pop_scope();

        // Finalize
        macro_compiler
            .chunk
            .set_max_registers(macro_compiler.max_register.saturating_add(1));

        Ok(MacroBody::new(
            Arc::new(macro_compiler.chunk),
            arity,
            has_rest,
        ))
    }
}
