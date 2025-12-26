// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Quote form compilation.
//!
//! This module handles compilation of the `quote` special form and the
//! conversion of AST nodes to compile-time constants.

use alloc::string::String;
use alloc::vec::Vec;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abx};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind};

impl Compiler<'_, '_, '_> {
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
            Ast::String(ref text) => Ok(Constant::String(String::from(text.as_str()))),
            Ast::Symbol(ref name) => {
                let id = self.interner.intern(name);
                Ok(Constant::Symbol(id))
            }
            Ast::Keyword(ref name) => {
                // Keywords use Constant::Keyword (name without : prefix)
                let id = self.interner.intern(name);
                Ok(Constant::Keyword(id))
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
            Ast::Map(ref elements) => {
                // Maps have alternating keys and values
                if elements.len() % 2_usize != 0 {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "quote",
                            message: "map literal must have even number of elements",
                        },
                        self.location(ast.span),
                    ));
                }
                let pairs: Result<Vec<(Constant, Constant)>, Error> = elements
                    .chunks_exact(2)
                    .map(|chunk| {
                        let key = self.ast_to_constant(chunk.first().ok_or_else(|| {
                            Error::new(ErrorKind::EmptyCall, self.location(ast.span))
                        })?)?;
                        let val = self.ast_to_constant(chunk.get(1_usize).ok_or_else(|| {
                            Error::new(ErrorKind::EmptyCall, self.location(ast.span))
                        })?)?;
                        Ok((key, val))
                    })
                    .collect();
                Ok(Constant::Map(pairs?))
            }
            Ast::Set(_) => Err(Error::new(
                ErrorKind::NotImplemented {
                    feature: "quoted sets",
                },
                self.location(ast.span),
            )),
            // Metadata: process the inner value (metadata ignored for now)
            Ast::WithMeta { ref value, .. } => self.ast_to_constant(value),
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
