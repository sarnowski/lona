// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Function parameter parsing and setup.
//!
//! This module handles parsing function parameter vectors and setting up
//! parameter bindings in child compilers, including destructuring patterns.

use alloc::string::String;
use alloc::vec::Vec;

use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::ParsedBinding;
use crate::compiler::Compiler;
use crate::compiler::destructure;
use crate::error::{Error, Kind as ErrorKind, SourceLocation};

/// Parsed parameter information from a parameter vector.
///
/// Contains the fixed (required) parameters and an optional rest parameter.
#[derive(Debug)]
pub(in crate::compiler) struct ParsedParams {
    /// Fixed (required) parameters.
    pub fixed: Vec<ParsedBinding>,
    /// Optional rest parameter that collects remaining arguments.
    pub rest: Option<ParsedBinding>,
}

/// State machine for parsing function parameters.
///
/// Tracks position in parameter list and whether `&` has been seen.
pub(super) struct ParamParseState {
    fixed: Vec<ParsedBinding>,
    rest: Option<ParsedBinding>,
    found_ampersand: bool,
    ampersand_span: Option<Span>,
}

impl ParamParseState {
    /// Creates a new parameter parse state.
    #[inline]
    pub const fn new() -> Self {
        Self {
            fixed: Vec::new(),
            rest: None,
            found_ampersand: false,
            ampersand_span: None,
        }
    }

    /// Handles the `&` marker.
    #[inline]
    pub const fn handle_ampersand(&mut self, location: SourceLocation) -> Result<(), Error> {
        if self.found_ampersand {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "multiple & in parameter list",
                },
                location,
            ));
        }
        self.found_ampersand = true;
        self.ampersand_span = Some(location.span);
        Ok(())
    }

    /// Handles an ignored parameter `_`.
    #[inline]
    pub fn handle_ignore(&mut self, location: SourceLocation) -> Result<(), Error> {
        if self.found_ampersand {
            self.set_rest(ParsedBinding::Ignore, location)
        } else {
            self.fixed.push(ParsedBinding::Ignore);
            Ok(())
        }
    }

    /// Handles a symbol parameter.
    #[inline]
    pub fn handle_symbol(&mut self, name: &str, location: SourceLocation) -> Result<(), Error> {
        if self.found_ampersand {
            self.set_rest(ParsedBinding::Symbol(String::from(name)), location)
        } else {
            self.fixed.push(ParsedBinding::Symbol(String::from(name)));
            Ok(())
        }
    }

    /// Handles a vector (destructuring) parameter.
    #[inline]
    pub fn handle_vector(
        &mut self,
        param: &Spanned<Ast>,
        location: SourceLocation,
    ) -> Result<(), Error> {
        if self.found_ampersand {
            self.set_rest(ParsedBinding::Pattern(param.clone()), location)
        } else {
            self.fixed.push(ParsedBinding::Pattern(param.clone()));
            Ok(())
        }
    }

    /// Handles a map (associative destructuring) parameter.
    #[inline]
    pub fn handle_map(
        &mut self,
        param: &Spanned<Ast>,
        location: SourceLocation,
    ) -> Result<(), Error> {
        if self.found_ampersand {
            self.set_rest(ParsedBinding::Pattern(param.clone()), location)
        } else {
            self.fixed.push(ParsedBinding::Pattern(param.clone()));
            Ok(())
        }
    }

    /// Sets the rest parameter, erroring if already set.
    fn set_rest(&mut self, binding: ParsedBinding, location: SourceLocation) -> Result<(), Error> {
        if self.rest.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "only one parameter allowed after &",
                },
                location,
            ));
        }
        self.rest = Some(binding);
        Ok(())
    }

    /// Finalizes parsing and returns the result.
    #[inline]
    pub fn finalize(self, fallback_location: SourceLocation) -> Result<ParsedParams, Error> {
        if self.found_ampersand && self.rest.is_none() {
            let location = self.ampersand_span.map_or(fallback_location, |span| {
                SourceLocation::new(fallback_location.source, span)
            });
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "fn",
                    message: "& must be followed by a rest parameter",
                },
                location,
            ));
        }
        Ok(ParsedParams {
            fixed: self.fixed,
            rest: self.rest,
        })
    }
}

impl Compiler<'_, '_, '_> {
    /// Extracts parameter information from a parameter vector AST.
    pub(in crate::compiler) fn extract_params(
        &self,
        params_ast: &Spanned<Ast>,
    ) -> Result<ParsedParams, Error> {
        let Ast::Vector(ref params_vec) = params_ast.node else {
            return Err(super::fn_error(
                "parameters must be a vector",
                self.location(params_ast.span),
            ));
        };

        let mut state = ParamParseState::new();

        for param in params_vec {
            self.process_param(param, &mut state)?;
        }

        state.finalize(self.location(params_ast.span))
    }

    /// Processes a single parameter in the parameter list.
    fn process_param(
        &self,
        param: &Spanned<Ast>,
        state: &mut ParamParseState,
    ) -> Result<(), Error> {
        match param.node {
            Ast::Symbol(ref name) if name == "&" => {
                state.handle_ampersand(self.location(param.span))
            }
            Ast::Symbol(ref name) if name == "_" => state.handle_ignore(self.location(param.span)),
            Ast::Symbol(ref name) => state.handle_symbol(name, self.location(param.span)),
            Ast::Vector(_) => state.handle_vector(param, self.location(param.span)),
            Ast::Map(_) => state.handle_map(param, self.location(param.span)),
            // All other AST types are invalid parameter bindings
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Set(_)
            | Ast::WithMeta { .. }
            | _ => Err(super::fn_error(
                "parameter must be a symbol, _, vector, or map pattern",
                self.location(param.span),
            )),
        }
    }

    /// Sets up parameters as local variables on a child compiler.
    ///
    /// This helper is used by both `compile_fn` and `compile_defmacro` to set up
    /// the parameter locals. Fixed parameters are placed in R[0..arity], and if
    /// a rest parameter exists, it is placed in R[arity].
    ///
    /// Supports destructuring patterns: when a parameter is a vector pattern,
    /// the argument value is destructured using the pattern, creating multiple
    /// local bindings from the single argument.
    ///
    /// # Register Layout
    ///
    /// Arguments occupy registers `R[0..total_params]` where `total_params` includes
    /// both fixed params and the rest param (if any). Destructuring patterns
    /// allocate temporary registers AFTER the argument region.
    pub(in crate::compiler) fn setup_params_on_compiler(
        child: &mut Compiler<'_, '_, '_>,
        parsed: &ParsedParams,
        arity: u8,
        location: SourceLocation,
    ) -> Result<(), Error> {
        child.locals.push_scope();

        // Calculate total parameter slots (fixed + optional rest)
        let total_param_slots = if parsed.rest.is_some() {
            arity.saturating_add(1)
        } else {
            arity
        };

        // Reserve registers R[0..total_param_slots] for arguments.
        // Destructuring will allocate temporaries starting at R[total_param_slots].
        child.next_register = total_param_slots;
        child.max_register = total_param_slots.saturating_sub(1);

        // Phase 1: Bind simple symbol parameters directly to their arg registers
        // (no bytecode needed, these are already in place)
        for (idx, param) in parsed.fixed.iter().enumerate() {
            let arg_reg = u8::try_from(idx)
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;

            if let ParsedBinding::Symbol(ref name) = *param {
                let symbol_id = child.interner.intern(name);
                child.locals.define(symbol_id, arg_reg);
            }
            // Ignore bindings need no action
            // Pattern bindings handled in phase 2
        }

        // Bind rest parameter if present
        if let Some(ParsedBinding::Symbol(ref name)) = parsed.rest {
            let symbol_id = child.interner.intern(name);
            child.locals.define(symbol_id, arity);
        }

        // Phase 2: Compile destructuring patterns (allocates temps after arg region)
        for (idx, param) in parsed.fixed.iter().enumerate() {
            let arg_reg = u8::try_from(idx)
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, location))?;

            if let ParsedBinding::Pattern(ref pattern_ast) = *param {
                Self::compile_pattern_binding(child, pattern_ast, arg_reg)?;
            }
            // Ignore bindings need no action
        }

        // Compile rest parameter destructuring if it's a pattern
        if let Some(ParsedBinding::Pattern(ref pattern_ast)) = parsed.rest {
            Self::compile_pattern_binding(child, pattern_ast, arity)?;
        }

        Ok(())
    }

    /// Compiles a destructuring pattern binding (vector or map).
    ///
    /// Dispatches to the appropriate destructuring compilation based on AST type.
    fn compile_pattern_binding(
        child: &mut Compiler<'_, '_, '_>,
        pattern_ast: &Spanned<Ast>,
        arg_reg: u8,
    ) -> Result<(), Error> {
        match pattern_ast.node {
            Ast::Vector(_) => {
                let pattern = destructure::parse_sequential_pattern(
                    child.interner,
                    pattern_ast,
                    child.source_id,
                )?;
                child.compile_sequential_binding(&pattern, arg_reg, pattern_ast.span)?;
            }
            Ast::Map(_) => {
                let pattern =
                    destructure::parse_map_pattern(child.interner, pattern_ast, child.source_id)?;
                child.compile_map_binding(&pattern, arg_reg, pattern_ast.span)?;
            }
            // These cases should not happen since we only store vector/map patterns
            // in ParsedBinding::Pattern via handle_vector/handle_map.
            // We still need to handle them for the non-exhaustive Ast enum.
            Ast::Nil | _ => {}
        }
        Ok(())
    }
}
