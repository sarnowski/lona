// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Function and macro definition compilation.
//!
//! This module handles compilation of:
//! - `fn` - function definitions
//! - `defmacro` - macro definitions

use alloc::sync::Arc;
use alloc::vec::Vec;

use lona_core::chunk::{Chunk, Constant};
use lona_core::opcode::{Opcode, encode_abc, encode_abx};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::macros::MacroDefinition;
use super::{Compiler, ExprResult, FnArgsResult};
use crate::error::{Error, Kind as ErrorKind};

impl Compiler<'_, '_, '_> {
    /// Compiles a `fn` special form.
    ///
    /// Syntax:
    /// - `(fn [params...] body...)`
    /// - `(fn name [params...] body...)` - named for recursion/debugging
    ///
    /// Creates a new function value. Parameters become local variables in the
    /// function's scope. The function body is compiled into a separate chunk.
    ///
    /// Note: In Phase 3.3, closures are not supported - functions cannot
    /// reference variables from enclosing scopes.
    pub(super) fn compile_fn(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Pre-compute location for error messages (before creating child compiler)
        let location = self.location(span);

        // Parse: (fn [params] body...) or (fn name [params] body...)
        let (name, params_ast, body) = self.parse_fn_args(args, span)?;

        // Extract parameter names
        let params = self.extract_params(params_ast)?;
        let arity = u8::try_from(params.len())
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;

        // Create a new compiler for the function body
        // Note: We share the registry so macros are available inside function bodies
        let mut fn_compiler = Compiler::new(self.interner, self.registry, self.source_id);
        let fn_name_str = name
            .clone()
            .unwrap_or_else(|| alloc::string::String::from("lambda"));
        fn_compiler.chunk = Chunk::with_name(fn_name_str);
        fn_compiler.chunk.set_arity(arity);

        // Parameters become locals at R[0], R[1], etc.
        fn_compiler.locals.push_scope();
        for (i, param) in params.iter().enumerate() {
            let symbol_id = fn_compiler.interner.intern(param);
            let reg = u8::try_from(i)
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;
            fn_compiler.locals.define(symbol_id, reg);
            fn_compiler.next_register = reg.saturating_add(1);
            if reg > fn_compiler.max_register {
                fn_compiler.max_register = reg;
            }
        }

        // Compile body
        let result_reg = if body.is_empty() {
            // Empty body returns nil
            let reg = fn_compiler.alloc_register(span)?;
            fn_compiler
                .chunk
                .emit(encode_abc(Opcode::LoadNil, reg, 0, 0), span);
            reg
        } else {
            // Compile all but last expression, discarding results
            for expr in body.get(..body.len().saturating_sub(1)).unwrap_or(&[]) {
                let checkpoint = fn_compiler.next_register;
                let _result = fn_compiler.compile_expr(expr)?;
                fn_compiler.free_registers_to(checkpoint);
            }
            // Compile last expression to return
            let last = body
                .last()
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, location))?;
            let result = fn_compiler.compile_expr(last)?;
            result.register
        };

        // Emit return
        fn_compiler
            .chunk
            .emit(encode_abc(Opcode::Return, result_reg, 1, 0), span);
        fn_compiler.locals.pop_scope();

        // Finalize function chunk
        fn_compiler
            .chunk
            .set_max_registers(fn_compiler.max_register.saturating_add(1));
        let fn_chunk = fn_compiler.chunk;

        // Add function as constant in parent chunk
        let const_idx = self.add_constant(
            Constant::Function {
                chunk: alloc::boxed::Box::new(fn_chunk),
                arity,
                name,
            },
            span,
        )?;

        // Load function into destination register
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);

        Ok(ExprResult { register: dest })
    }

    /// Parses the arguments to a `fn` special form.
    ///
    /// Returns (name, `params_ast`, body) where name is optional.
    pub(super) fn parse_fn_args<'args>(
        &self,
        args: &'args [Spanned<Ast>],
        span: Span,
    ) -> Result<FnArgsResult<'args>, Error> {
        let location = self.location(span);
        let first = args.first().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "expected (fn [params] body...) or (fn name [params] body...)",
                },
                location,
            )
        })?;

        // Check if first arg is a name (symbol) or params (vector)
        match first.node {
            Ast::Vector(_) => {
                // (fn [params] body...)
                let body = args.get(1_usize..).unwrap_or(&[]);
                Ok((None, first, body))
            }
            Ast::Symbol(ref name) => {
                // (fn name [params] body...)
                let params_ast = args.get(1_usize).ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "fn",
                            message: "expected parameter vector after function name",
                        },
                        location,
                    )
                })?;
                let body = args.get(2_usize..).unwrap_or(&[]);
                Ok((Some(name.clone()), params_ast, body))
            }
            // All other AST variants are invalid as the first argument to fn
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Map(_)
            | _ => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "expected [params] or name",
                },
                self.location(first.span),
            )),
        }
    }

    /// Extracts parameter names from a parameter vector AST.
    pub(super) fn extract_params(
        &self,
        params_ast: &Spanned<Ast>,
    ) -> Result<Vec<alloc::string::String>, Error> {
        let Ast::Vector(ref params_vec) = params_ast.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "parameters must be a vector",
                },
                self.location(params_ast.span),
            ));
        };

        let mut params = Vec::new();
        for param in params_vec {
            let Ast::Symbol(ref name) = param.node else {
                return Err(Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "fn",
                        message: "parameter must be a symbol",
                    },
                    self.location(param.span),
                ));
            };
            params.push(name.clone());
        }
        Ok(params)
    }

    /// Compiles a `defmacro` special form.
    ///
    /// Syntax: `(defmacro name [params...] body...)`
    ///
    /// Defines a compile-time macro. The macro body is compiled to bytecode
    /// and stored in the compiler's macro registry. When the macro is called,
    /// it receives unevaluated arguments and returns transformed AST.
    ///
    /// Returns the macro's symbol name.
    pub(super) fn compile_defmacro(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Pre-compute location for error messages (before creating child compiler)
        let location = self.location(span);

        // Need at least name and params
        if args.len() < 2_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "defmacro",
                    message: "expected (defmacro name [params...] body...)",
                },
                location,
            ));
        }

        // Extract name (must be a symbol)
        let name_ast = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, location))?;
        let Ast::Symbol(ref name_ref) = name_ast.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "defmacro",
                    message: "macro name must be a symbol",
                },
                self.location(name_ast.span),
            ));
        };
        let name = name_ref.clone();

        // Extract params (must be a vector of symbols)
        let params_ast = args
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, location))?;
        let params = self.extract_params(params_ast).map_err(|_err| {
            Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "defmacro",
                    message: "parameters must be a vector of symbols",
                },
                self.location(params_ast.span),
            )
        })?;
        let arity = u8::try_from(params.len())
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;

        // Body is everything after params
        let body = args.get(2_usize..).unwrap_or(&[]);
        if body.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "defmacro",
                    message: "macro body cannot be empty",
                },
                location,
            ));
        }

        // Intern the macro name before creating the child compiler to avoid
        // double mutable borrow of self.interner
        let name_id = self.interner.intern(&name);

        // Create a child compiler for the macro body.
        // Note: We share the registry so macros can be used inside macro bodies.
        let mut macro_compiler = Compiler::new(self.interner, self.registry, self.source_id);
        macro_compiler.chunk = Chunk::with_name(alloc::format!("macro:{name}"));
        macro_compiler.chunk.set_arity(arity);

        // Parameters become locals at R[0], R[1], etc.
        macro_compiler.locals.push_scope();
        for (i, param) in params.iter().enumerate() {
            let symbol_id = macro_compiler.interner.intern(param);
            let reg = u8::try_from(i)
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;
            macro_compiler.locals.define(symbol_id, reg);
            macro_compiler.next_register = reg.saturating_add(1);
            if reg > macro_compiler.max_register {
                macro_compiler.max_register = reg;
            }
        }

        // Compile body (all expressions, last one is return value)
        let result_reg = {
            // Compile all but last, discarding results
            for expr in body
                .get(..body.len().saturating_sub(1_usize))
                .unwrap_or(&[])
            {
                let checkpoint = macro_compiler.next_register;
                let _result = macro_compiler.compile_expr(expr)?;
                macro_compiler.free_registers_to(checkpoint);
            }
            // Compile last expression as return value
            let last = body
                .last()
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, location))?;
            let result = macro_compiler.compile_expr(last)?;
            result.register
        };

        // Emit return instruction
        macro_compiler
            .chunk
            .emit(encode_abc(Opcode::Return, result_reg, 1, 0), span);
        macro_compiler.locals.pop_scope();

        // Finalize the macro chunk
        macro_compiler
            .chunk
            .set_max_registers(macro_compiler.max_register.saturating_add(1));

        // Extract the macro chunk before using self again
        let macro_chunk = macro_compiler.chunk;

        // Store in macro registry
        self.registry.register(
            name_id,
            MacroDefinition::new(Arc::new(macro_chunk), arity, name),
        );

        // Return the macro's symbol name
        // This mimics `def` behavior - the expression evaluates to the defined name
        let const_idx = self.add_constant(Constant::Symbol(name_id), span)?;
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);

        Ok(ExprResult { register: dest })
    }
}
