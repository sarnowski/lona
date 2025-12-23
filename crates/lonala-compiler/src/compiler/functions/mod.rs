// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Function and macro definition compilation.
//!
//! This module handles compilation of:
//! - `fn` - function definitions (single and multi-arity)
//! - `defmacro` - macro definitions (single and multi-arity)

mod macros;
mod params;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use lona_core::chunk::{Chunk, Constant, FunctionBodyData};
use lona_core::opcode::{Opcode, encode_abc, encode_abx};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::{CaptureContext, Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind, SourceLocation};

/// A parsed binding target in a function parameter list.
///
/// Represents a single parameter position, which can be a simple symbol,
/// an ignored position, or a destructuring pattern.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(super) enum ParsedBinding {
    /// A simple symbol binding: `x` in `[x y z]`.
    Symbol(String),

    /// An ignored position: `_` in `[x _ z]`.
    Ignore,

    /// A destructuring pattern: `[a b]` in `[[a b] c]`.
    /// Stores the original AST for later parsing by the destructure module.
    Pattern(Spanned<Ast>),
}

/// Result of parsing an `fn` special form.
///
/// Distinguishes between single-arity and multi-arity syntax.
#[derive(Debug)]
enum FnForm<'args> {
    /// Single arity: `(fn [params] body...)` or `(fn name [params] body...)`
    SingleArity {
        name: Option<String>,
        params: &'args Spanned<Ast>,
        body: &'args [Spanned<Ast>],
    },
    /// Multi arity: `(fn ([p1] b1) ([p2] b2)...)` or `(fn name ([p1] b1)...)`
    MultiArity {
        name: Option<String>,
        /// Each element is a list `([params] body...)`
        bodies: &'args [Spanned<Ast>],
    },
}

/// Helper to create `fn` form errors.
#[inline]
pub(super) const fn fn_error(message: &'static str, location: SourceLocation) -> Error {
    Error::new(
        ErrorKind::InvalidSpecialForm {
            form: "fn",
            message,
        },
        location,
    )
}

impl Compiler<'_, '_, '_> {
    /// Compiles a `fn` special form.
    ///
    /// Supports both single-arity and multi-arity syntax:
    /// - Single: `(fn [params] body...)` or `(fn name [params] body...)`
    /// - Multi: `(fn ([p1] b1) ([p2] b2)...)` or `(fn name ([p1] b1)...)`
    #[inline]
    pub fn compile_fn(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        let form = self.parse_fn_form(args, span)?;

        match form {
            FnForm::SingleArity { name, params, body } => {
                let body_data = self.compile_fn_body(name.as_deref(), params, body, span)?;
                self.emit_function(alloc::vec![body_data], name, span)
            }
            FnForm::MultiArity { name, bodies } => self.compile_multi_arity_fn(name, bodies, span),
        }
    }

    /// Parses an `fn` form into a structured representation.
    fn parse_fn_form<'args>(
        &self,
        args: &'args [Spanned<Ast>],
        span: Span,
    ) -> Result<FnForm<'args>, Error> {
        let location = self.location(span);

        if args.is_empty() {
            return Err(fn_error("expected parameters and body", location));
        }

        let first = args
            .first()
            .ok_or_else(|| fn_error("expected parameters", location))?;

        match first.node {
            // (fn [params] body...) - anonymous, single arity
            Ast::Vector(_) => {
                let body = args.get(1_usize..).unwrap_or(&[]);
                Ok(FnForm::SingleArity {
                    name: None,
                    params: first,
                    body,
                })
            }
            // (fn name ...) - named function
            Ast::Symbol(ref name) => {
                let name_str = name.clone();
                self.parse_named_fn_form(&name_str, args, location)
            }
            // (fn ([params] body...) ...) - anonymous, multi-arity
            Ast::List(_) if Self::is_arity_body(&first.node) => Ok(FnForm::MultiArity {
                name: None,
                bodies: args,
            }),
            // Error case
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Map(_)
            | Ast::Set(_)
            | Ast::WithMeta { .. }
            | _ => Err(fn_error(
                "expected [params], name, or ([params] body...)",
                self.location(first.span),
            )),
        }
    }

    /// Parses a named function form.
    fn parse_named_fn_form<'args>(
        &self,
        name: &str,
        args: &'args [Spanned<Ast>],
        location: SourceLocation,
    ) -> Result<FnForm<'args>, Error> {
        let second = args
            .get(1_usize)
            .ok_or_else(|| fn_error("expected parameters after function name", location))?;

        match second.node {
            // (fn name [params] body...)
            Ast::Vector(_) => {
                let body = args.get(2_usize..).unwrap_or(&[]);
                Ok(FnForm::SingleArity {
                    name: Some(String::from(name)),
                    params: second,
                    body,
                })
            }
            // (fn name ([params] body...) ...) - named multi-arity
            Ast::List(_) if Self::is_arity_body(&second.node) => Ok(FnForm::MultiArity {
                name: Some(String::from(name)),
                bodies: args.get(1_usize..).unwrap_or(&[]),
            }),
            // All other cases are invalid
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Symbol(_)
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Map(_)
            | Ast::Set(_)
            | Ast::WithMeta { .. }
            | _ => Err(fn_error(
                "expected [params] or ([params] body...) after function name",
                self.location(second.span),
            )),
        }
    }

    /// Compiles a multi-arity function.
    fn compile_multi_arity_fn(
        &mut self,
        name: Option<String>,
        bodies_ast: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let location = self.location(span);

        // Build shared capture context for all bodies
        let shared_context = self.build_capture_context();

        let mut bodies = Vec::with_capacity(bodies_ast.len());
        for body_ast in bodies_ast {
            let (params, body) = Self::parse_arity_body(body_ast, self.location(body_ast.span))?;
            let body_data = self.compile_fn_body_with_context(
                name.as_deref(),
                params,
                body,
                body_ast.span,
                Some(&shared_context),
            )?;
            bodies.push(body_data);
        }

        // Validate arities are unique
        Self::validate_arities(&bodies, location)?;

        self.emit_function(bodies, name, span)
    }

    /// Validates that all arities are unique and compatible.
    ///
    /// Rules for multi-arity functions:
    /// 1. At most one body can have a rest parameter
    /// 2. No two bodies can have the same arity
    /// 3. Fixed arity bodies cannot have more parameters than a variadic body's
    ///    fixed parameter count (the variadic would catch those calls)
    pub(super) fn validate_arities(
        bodies: &[FunctionBodyData],
        location: SourceLocation,
    ) -> Result<(), Error> {
        // Find the variadic body (if any)
        let variadic_body = bodies.iter().find(|body| body.has_rest);

        // Check for duplicate arities and variadic constraints
        for (i, body_a) in bodies.iter().enumerate() {
            for body_b in bodies.iter().skip(i.saturating_add(1)) {
                // If both have rest params, they overlap
                if body_a.has_rest && body_b.has_rest {
                    return Err(fn_error("multiple arities with rest parameter", location));
                }
                // If same arity and same rest status, duplicate
                if body_a.arity == body_b.arity && body_a.has_rest == body_b.has_rest {
                    return Err(fn_error(
                        "duplicate arity in multi-arity function",
                        location,
                    ));
                }
            }

            // If there's a variadic body and this is a fixed body,
            // ensure this body's arity is not greater than the variadic's fixed params.
            // Rationale: A variadic body `([x & r] ...)` catches all arities >= 1.
            // A fixed body `([a b] ...)` with arity 2 would never be reached since
            // the variadic catches arity 2. This is an error in Clojure.
            if let Some(variadic) = variadic_body
                && !body_a.has_rest
                && body_a.arity > variadic.arity
            {
                return Err(fn_error(
                    "fixed arity cannot exceed variadic body's fixed parameter count",
                    location,
                ));
            }
        }
        Ok(())
    }

    /// Checks if an AST node looks like an arity body: a list starting with a vector.
    pub(super) fn is_arity_body(ast: &Ast) -> bool {
        if let Ast::List(ref elements) = *ast
            && let Some(first) = elements.first()
        {
            return matches!(first.node, Ast::Vector(_));
        }
        false
    }

    /// Parses an arity body `([params] body...)` returning params and body.
    pub(super) fn parse_arity_body(
        body_ast: &Spanned<Ast>,
        location: SourceLocation,
    ) -> Result<(&Spanned<Ast>, &[Spanned<Ast>]), Error> {
        let Ast::List(ref elements) = body_ast.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "arity body must be a list",
                },
                location,
            ));
        };

        let params = elements.first().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "arity body cannot be empty",
                },
                location,
            )
        })?;

        if !matches!(params.node, Ast::Vector(_)) {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "first element of arity body must be parameter vector",
                },
                location,
            ));
        }

        let body = elements.get(1_usize..).unwrap_or(&[]);
        Ok((params, body))
    }

    /// Compiles a single function body (params + body expressions) into a `FunctionBodyData`.
    ///
    /// For closures, this method:
    /// 1. Builds a capture context from the parent compiler's state
    /// 2. Passes it to the child compiler so symbols can be resolved as upvalues
    /// 3. Collects the upvalue sources after compilation
    fn compile_fn_body(
        &mut self,
        name: Option<&str>,
        params_ast: &Spanned<Ast>,
        body: &[Spanned<Ast>],
        span: Span,
    ) -> Result<FunctionBodyData, Error> {
        self.compile_fn_body_with_context(name, params_ast, body, span, None)
    }

    /// Compiles a function body with an optional shared capture context.
    ///
    /// When `shared_context` is provided, uses that context instead of building
    /// a new one. This is used for multi-arity functions where all bodies must
    /// share the same upvalue array.
    fn compile_fn_body_with_context(
        &mut self,
        name: Option<&str>,
        params_ast: &Spanned<Ast>,
        body: &[Spanned<Ast>],
        span: Span,
        shared_context: Option<&CaptureContext>,
    ) -> Result<FunctionBodyData, Error> {
        let location = self.location(span);

        // Extract parameter names
        let parsed_params = self.extract_params(params_ast)?;
        let arity = u8::try_from(parsed_params.fixed.len())
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;
        let has_rest = parsed_params.rest.is_some();

        // Build capture context from parent (or use shared context for multi-arity)
        let capture_context = shared_context
            .cloned()
            .unwrap_or_else(|| self.build_capture_context());

        // Create a new compiler for the function body
        let mut fn_compiler = Compiler::new(self.interner, self.registry, self.source_id);
        fn_compiler.set_capture_context(capture_context);

        let fn_name_str = name.map_or_else(|| String::from("lambda"), String::from);
        fn_compiler.chunk = Chunk::with_name(fn_name_str);
        fn_compiler.chunk.set_arity(arity);
        fn_compiler.chunk.set_has_rest(has_rest);

        // Set up parameter locals
        Self::setup_params_on_compiler(&mut fn_compiler, &parsed_params, arity, location)?;

        // Compile body
        let result_reg = if body.is_empty() {
            let reg = fn_compiler.alloc_register(span)?;
            fn_compiler
                .chunk
                .emit(encode_abc(Opcode::LoadNil, reg, 0, 0), span);
            reg
        } else {
            for expr in body.get(..body.len().saturating_sub(1)).unwrap_or(&[]) {
                let checkpoint = fn_compiler.next_register;
                let _result = fn_compiler.compile_expr(expr)?;
                fn_compiler.free_registers_to(checkpoint);
            }
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

        // Finalize
        fn_compiler
            .chunk
            .set_max_registers(fn_compiler.max_register.saturating_add(1));

        // Collect upvalue sources from the child compiler
        let upvalue_sources = fn_compiler.take_upvalue_sources();

        Ok(FunctionBodyData::new(
            Box::new(fn_compiler.chunk),
            arity,
            has_rest,
            upvalue_sources,
        ))
    }

    /// Emits the function constant and loads it into a register.
    ///
    /// For closures (functions with upvalues), emits a `Closure` instruction
    /// that captures values at runtime. For non-closures, emits a simple `LoadK`.
    fn emit_function(
        &mut self,
        bodies: Vec<FunctionBodyData>,
        name: Option<String>,
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Check if any body has upvalues (making this a closure)
        let has_upvalues = bodies.iter().any(|body| !body.upvalue_sources.is_empty());

        let const_idx = self.add_constant(Constant::Function { bodies, name }, span)?;
        let dest = self.alloc_register(span)?;

        if has_upvalues {
            // Emit Closure instruction which captures upvalues at runtime
            self.chunk
                .emit(encode_abx(Opcode::Closure, dest, const_idx), span);
        } else {
            // Simple function, just load the constant
            self.chunk
                .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        }

        Ok(ExprResult { register: dest })
    }
}
