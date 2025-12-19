// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Function and macro definition compilation.
//!
//! This module handles compilation of:
//! - `fn` - function definitions (single and multi-arity)
//! - `defmacro` - macro definitions (single and multi-arity)

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lona_core::chunk::{Chunk, Constant, FunctionBodyData};
use lona_core::opcode::{Opcode, encode_abc, encode_abx};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::macros::{MacroBody, MacroDefinition};
use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind, SourceLocation};

/// Parsed parameter information from a parameter vector.
///
/// Contains the fixed (required) parameters and an optional rest parameter.
#[derive(Debug)]
pub(super) struct ParsedParams {
    /// Fixed (required) parameters.
    pub fixed: Vec<String>,
    /// Optional rest parameter that collects remaining arguments.
    pub rest: Option<String>,
}

/// Result of parsing an `fn` special form.
///
/// Distinguishes between single-arity and multi-arity syntax.
#[derive(Debug)]
pub(super) enum FnForm<'args> {
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

impl Compiler<'_, '_, '_> {
    /// Compiles a `fn` special form.
    ///
    /// Supports both single-arity and multi-arity syntax:
    /// - Single arity: `(fn [params...] body...)` or `(fn name [params...] body...)`
    /// - Multi-arity: `(fn ([params1] body1...) ([params2] body2...))` or
    ///   `(fn name ([params1] body1...) ([params2] body2...))`
    ///
    /// Creates a new function value. Parameters become local variables in the
    /// function's scope. Each arity body is compiled into a separate chunk.
    /// Supports rest parameters: `(fn [a b & rest] ...)`.
    ///
    /// Note: In Phase 3.3, closures are not supported - functions cannot
    /// reference variables from enclosing scopes.
    pub(super) fn compile_fn(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let location = self.location(span);
        let form = self.parse_fn_form(args, span)?;

        match form {
            FnForm::SingleArity { name, params, body } => {
                // Compile single body, wrap in vec
                let body_data = self.compile_fn_body(name.as_deref(), params, body, span)?;
                self.emit_function(alloc::vec![body_data], name, span)
            }
            FnForm::MultiArity { name, bodies } => {
                // Compile each arity body
                let mut body_datas = Vec::with_capacity(bodies.len());
                for body_ast in bodies {
                    let (params, body) =
                        Self::parse_arity_body(body_ast, self.location(body_ast.span))?;
                    let body_data =
                        self.compile_fn_body(name.as_deref(), params, body, body_ast.span)?;
                    body_datas.push(body_data);
                }
                // Validate arities
                Self::validate_arities(&body_datas, location)?;
                self.emit_function(body_datas, name, span)
            }
        }
    }

    /// Parses the `fn` form to determine if it's single-arity or multi-arity.
    fn parse_fn_form<'args>(
        &self,
        args: &'args [Spanned<Ast>],
        span: Span,
    ) -> Result<FnForm<'args>, Error> {
        let location = self.location(span);
        let first = args.first().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "expected (fn [params] body...) or (fn name [params] body...) or (fn ([params] body...)...)",
                },
                location,
            )
        })?;

        match first.node {
            Ast::Vector(_) => {
                // (fn [params] body...) - single arity
                let body = args.get(1_usize..).unwrap_or(&[]);
                Ok(FnForm::SingleArity {
                    name: None,
                    params: first,
                    body,
                })
            }
            Ast::Symbol(ref name) => {
                // (fn name ...) - need to check second arg
                let second = args.get(1_usize).ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "fn",
                            message: "expected parameter vector or arity body after function name",
                        },
                        location,
                    )
                })?;

                match second.node {
                    Ast::Vector(_) => {
                        // (fn name [params] body...) - single arity
                        let body = args.get(2_usize..).unwrap_or(&[]);
                        Ok(FnForm::SingleArity {
                            name: Some(name.clone()),
                            params: second,
                            body,
                        })
                    }
                    Ast::List(_) if Self::is_arity_body(&second.node) => {
                        // (fn name ([params] body...)...) - multi arity
                        let bodies = args.get(1_usize..).unwrap_or(&[]);
                        Ok(FnForm::MultiArity {
                            name: Some(name.clone()),
                            bodies,
                        })
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
                    | _ => Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "fn",
                            message: "expected [params] or ([params] body...) after function name",
                        },
                        self.location(second.span),
                    )),
                }
            }
            Ast::List(_) if Self::is_arity_body(&first.node) => {
                // (fn ([params] body...)...) - multi arity (anonymous)
                Ok(FnForm::MultiArity {
                    name: None,
                    bodies: args,
                })
            }
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
                    message: "expected [params], name, or ([params] body...)",
                },
                self.location(first.span),
            )),
        }
    }

    /// Checks if an AST node is an arity body: a list whose first element is a vector.
    fn is_arity_body(ast: &Ast) -> bool {
        match *ast {
            Ast::List(ref elements) => elements
                .first()
                .is_some_and(|first| matches!(first.node, Ast::Vector(_))),
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Symbol(_)
            | Ast::Keyword(_)
            | Ast::Vector(_)
            | Ast::Map(_)
            | _ => false,
        }
    }

    /// Parses an arity body: `([params] body...)`
    fn parse_arity_body(
        body_ast: &Spanned<Ast>,
        location: SourceLocation,
    ) -> Result<(&Spanned<Ast>, &[Spanned<Ast>]), Error> {
        let Ast::List(ref elements) = body_ast.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "arity body must be a list ([params] body...)",
                },
                location,
            ));
        };

        let params = elements.first().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "arity body must have parameter vector",
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
    fn compile_fn_body(
        &mut self,
        name: Option<&str>,
        params_ast: &Spanned<Ast>,
        body: &[Spanned<Ast>],
        span: Span,
    ) -> Result<FunctionBodyData, Error> {
        let location = self.location(span);

        // Extract parameter names
        let parsed_params = self.extract_params(params_ast)?;
        let arity = u8::try_from(parsed_params.fixed.len())
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;
        let has_rest = parsed_params.rest.is_some();

        // Create a new compiler for the function body
        let mut fn_compiler = Compiler::new(self.interner, self.registry, self.source_id);
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

        Ok(FunctionBodyData::new(
            Box::new(fn_compiler.chunk),
            arity,
            has_rest,
        ))
    }

    /// Validates multi-arity constraints.
    fn validate_arities(
        bodies: &[FunctionBodyData],
        location: SourceLocation,
    ) -> Result<(), Error> {
        let mut fixed_arities = Vec::new();
        let mut variadic: Option<u8> = None;

        for body in bodies {
            if body.has_rest {
                if variadic.is_some() {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "fn",
                            message: "only one variadic arity allowed",
                        },
                        location,
                    ));
                }
                variadic = Some(body.arity);
            } else {
                if fixed_arities.contains(&body.arity) {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "fn",
                            message: "duplicate arity",
                        },
                        location,
                    ));
                }
                fixed_arities.push(body.arity);
            }
        }

        // Check variadic constraint: fixed arities cannot exceed variadic's fixed param count.
        // Equal is allowed (exact match beats variadic), but greater is an error because
        // those calls would never reach the fixed arity body.
        // Example: (fn ([x] 1) ([x & r] 2)) is VALID - 1-arg calls use exact match
        // Example: (fn ([x y] 1) ([x & r] 2)) is INVALID - 2-arg calls never reach first body
        if let Some(var_arity) = variadic {
            for &fixed in &fixed_arities {
                if fixed > var_arity {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "fn",
                            message: "fixed arity cannot exceed variadic arity",
                        },
                        location,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Emits the function constant and loads it into a register.
    fn emit_function(
        &mut self,
        bodies: Vec<FunctionBodyData>,
        name: Option<String>,
        span: Span,
    ) -> Result<ExprResult, Error> {
        let const_idx = self.add_constant(Constant::Function { bodies, name }, span)?;
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Extracts parameter names from a parameter vector AST.
    ///
    /// Handles rest parameters using `&` syntax:
    /// - `[a b]` - two fixed parameters
    /// - `[a & rest]` - one fixed, rest collects remaining
    /// - `[& rest]` - zero fixed, rest collects all
    pub(super) fn extract_params(&self, params_ast: &Spanned<Ast>) -> Result<ParsedParams, Error> {
        let Ast::Vector(ref params_vec) = params_ast.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "parameters must be a vector",
                },
                self.location(params_ast.span),
            ));
        };

        let mut fixed = Vec::new();
        let mut rest = None;
        let mut found_ampersand = false;
        let mut ampersand_span = None;

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

            if name == "&" {
                if found_ampersand {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "fn",
                            message: "multiple & in parameter list",
                        },
                        self.location(param.span),
                    ));
                }
                found_ampersand = true;
                ampersand_span = Some(param.span);
            } else if found_ampersand {
                // This symbol follows &, so it's the rest parameter
                if rest.is_some() {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "fn",
                            message: "only one parameter allowed after &",
                        },
                        self.location(param.span),
                    ));
                }
                rest = Some(name.clone());
            } else {
                // Regular fixed parameter
                fixed.push(name.clone());
            }
        }

        // Check that & was followed by a parameter
        if found_ampersand && rest.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "& must be followed by a rest parameter name",
                },
                self.location(ampersand_span.unwrap_or(params_ast.span)),
            ));
        }

        Ok(ParsedParams { fixed, rest })
    }

    /// Sets up parameters as local variables on a child compiler.
    ///
    /// This helper is used by both `compile_fn` and `compile_defmacro` to set up
    /// the parameter locals. Fixed parameters are placed in R[0..arity], and if
    /// a rest parameter exists, it is placed in R[arity].
    fn setup_params_on_compiler(
        child: &mut Compiler<'_, '_, '_>,
        parsed: &ParsedParams,
        arity: u8,
        location: crate::error::SourceLocation,
    ) -> Result<(), Error> {
        child.locals.push_scope();

        // Fixed parameters: R[0], R[1], ..., R[arity-1]
        for (idx, param) in parsed.fixed.iter().enumerate() {
            let symbol_id = child.interner.intern(param);
            let reg = u8::try_from(idx)
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;
            child.locals.define(symbol_id, reg);
            child.next_register = reg.saturating_add(1);
            if reg > child.max_register {
                child.max_register = reg;
            }
        }

        // Rest parameter: R[arity] (if present)
        if let Some(ref rest_name) = parsed.rest {
            let symbol_id = child.interner.intern(rest_name);
            let reg = arity; // Rest param is at R[arity]
            child.locals.define(symbol_id, reg);
            child.next_register = reg.saturating_add(1);
            if reg > child.max_register {
                child.max_register = reg;
            }
        }

        Ok(())
    }

    /// Compiles a `defmacro` special form.
    ///
    /// Supports both single-arity and multi-arity syntax:
    /// - Single arity: `(defmacro name [params...] body...)`
    /// - Multi-arity: `(defmacro name ([] body1) ([x] body2) ([x y] body3))`
    ///
    /// Defines a compile-time macro. The macro body is compiled to bytecode
    /// and stored in the compiler's macro registry. When the macro is called,
    /// it receives unevaluated arguments and returns transformed AST.
    /// Supports rest parameters: `(defmacro when [test & body] ...)`.
    ///
    /// Returns the macro's symbol name.
    pub(super) fn compile_defmacro(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let location = self.location(span);
        if args.len() < 2_usize {
            return Err(Self::defmacro_error(
                "expected (defmacro name [params...] body...) or (defmacro name ([params] body)...)",
                location,
            ));
        }
        // Extract name (must be a symbol)
        let name_ast = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, location))?;
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

        self.registry
            .register(name_id, MacroDefinition::new(macro_bodies, name));
        let const_idx = self.add_constant(Constant::Symbol(name_id), span)?;
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
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
            .map(|mb| FunctionBodyData::new(Box::new((*mb.chunk).clone()), mb.arity, mb.has_rest))
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
