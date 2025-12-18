// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Quasiquote expansion for syntax-quote.
//!
//! This module handles the expansion of quasiquoted forms (syntax-quote),
//! including unquote (~) and unquote-splicing (~@) within templates.

use alloc::vec::Vec;

use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind};

/// Represents a part of a quasiquote expansion - either a single value or
/// a sequence to be spliced.
pub(super) enum ExpandedPart {
    /// A single element (not spliced).
    Single(Spanned<Ast>),
    /// A sequence to be spliced into the parent.
    Splice(Spanned<Ast>),
}

impl Compiler<'_, '_, '_> {
    /// Compiles a `syntax-quote` special form (quasiquote).
    ///
    /// Syntax: `` `datum `` or `(syntax-quote datum)`
    ///
    /// Expands the datum at compile time, allowing `~` (unquote) and `~@`
    /// (unquote-splicing) to interpolate evaluated expressions into the
    /// quoted structure.
    pub(super) fn compile_syntax_quote(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        if args.len() != 1_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "syntax-quote",
                    message: "expected exactly 1 argument",
                },
                self.location(span),
            ));
        }

        let datum = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        // Expand the quasiquote template at depth 1
        let expanded = self.expand_quasiquote(datum, 1_u32)?;

        // Compile the expanded form
        self.compile_expr(&expanded)
    }

    // =========================================================================
    // Quasiquote Expansion Helpers
    // =========================================================================

    /// Expands a quasiquoted form at the given depth.
    ///
    /// `depth` tracks nesting of syntax-quote forms. At depth 1, unquote and
    /// unquote-splicing are active. At higher depths, they become quoted.
    pub(super) fn expand_quasiquote(
        &mut self,
        ast: &Spanned<Ast>,
        depth: u32,
    ) -> Result<Spanned<Ast>, Error> {
        match ast.node {
            // Handle (unquote x)
            Ast::List(ref elements) if Self::is_unquote(elements) => {
                self.expand_unquote(elements, ast.span, depth)
            }

            // Handle (unquote-splicing x) at top level of list element
            // This case is handled by expand_quasiquote_list, but if we see
            // it here directly, it's an error (can't splice into non-sequence)
            Ast::List(ref elements) if Self::is_unquote_splicing(elements) => {
                if depth == 1 {
                    Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "unquote-splicing",
                            message: "~@ not in list or vector context",
                        },
                        self.location(ast.span),
                    ))
                } else {
                    // At deeper depth, treat as a regular list
                    self.expand_nested_unquote_splicing(elements, ast.span, depth)
                }
            }

            // Handle (syntax-quote x) - nested quasiquote
            Ast::List(ref elements) if Self::is_syntax_quote(elements) => {
                self.expand_nested_syntax_quote(elements, ast.span, depth)
            }

            // Handle regular lists
            Ast::List(ref elements) => self.expand_quasiquote_list(elements, ast.span, depth),

            // Handle vectors
            Ast::Vector(ref elements) => self.expand_quasiquote_vector(elements, ast.span, depth),

            // Handle maps (basic support)
            Ast::Map(ref _elements) => Err(Error::new(
                ErrorKind::NotImplemented {
                    feature: "quasiquoted maps",
                },
                self.location(ast.span),
            )),

            // Atoms: wrap in (quote ...)
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Symbol(_)
            | Ast::Keyword(_) => Ok(Self::quote_atom(ast)),

            // Handle future AST variants
            _ => Err(Error::new(
                ErrorKind::NotImplemented {
                    feature: "unknown AST node in quasiquote",
                },
                self.location(ast.span),
            )),
        }
    }

    /// Checks if a list is an `(unquote x)` form.
    pub(super) fn is_unquote(elements: &[Spanned<Ast>]) -> bool {
        matches!(
            elements.first().map(|elem| &elem.node),
            Some(Ast::Symbol(name)) if name == "unquote"
        )
    }

    /// Checks if a list is an `(unquote-splicing x)` form.
    pub(super) fn is_unquote_splicing(elements: &[Spanned<Ast>]) -> bool {
        matches!(
            elements.first().map(|elem| &elem.node),
            Some(Ast::Symbol(name)) if name == "unquote-splicing"
        )
    }

    /// Checks if a list is a `(syntax-quote x)` form.
    fn is_syntax_quote(elements: &[Spanned<Ast>]) -> bool {
        matches!(
            elements.first().map(|elem| &elem.node),
            Some(Ast::Symbol(name)) if name == "syntax-quote"
        )
    }

    /// Expands an `(unquote x)` form.
    fn expand_unquote(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
        depth: u32,
    ) -> Result<Spanned<Ast>, Error> {
        if elements.len() != 2_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "unquote",
                    message: "expected exactly 1 argument",
                },
                self.location(span),
            ));
        }

        let inner = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        if depth == 1 {
            // At depth 1: return the expression to be evaluated
            Ok(inner.clone())
        } else {
            // At deeper depth: keep structure, decrease depth for inner
            let expanded_inner = self.expand_quasiquote(inner, depth.saturating_sub(1))?;
            Ok(Self::make_list(
                alloc::vec![
                    Self::make_list(
                        alloc::vec![
                            Self::make_symbol("list", span),
                            Self::make_quoted_symbol("unquote", span),
                        ],
                        span,
                    ),
                    expanded_inner,
                ],
                span,
            ))
        }
    }

    /// Expands an `(unquote-splicing x)` at depth > 1.
    fn expand_nested_unquote_splicing(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
        depth: u32,
    ) -> Result<Spanned<Ast>, Error> {
        if elements.len() != 2_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "unquote-splicing",
                    message: "expected exactly 1 argument",
                },
                self.location(span),
            ));
        }

        let inner = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let expanded_inner = self.expand_quasiquote(inner, depth.saturating_sub(1))?;

        // (list 'unquote-splicing expanded_inner)
        Ok(Self::make_list(
            alloc::vec![
                Self::make_symbol("list", span),
                Self::make_quoted_symbol("unquote-splicing", span),
                expanded_inner,
            ],
            span,
        ))
    }

    /// Expands a nested `(syntax-quote x)` form.
    fn expand_nested_syntax_quote(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
        depth: u32,
    ) -> Result<Spanned<Ast>, Error> {
        if elements.len() != 2_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "syntax-quote",
                    message: "expected exactly 1 argument",
                },
                self.location(span),
            ));
        }

        let inner = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        // Increase depth for nested syntax-quote
        let expanded_inner = self.expand_quasiquote(inner, depth.saturating_add(1))?;

        // (list 'syntax-quote expanded_inner)
        Ok(Self::make_list(
            alloc::vec![
                Self::make_symbol("list", span),
                Self::make_quoted_symbol("syntax-quote", span),
                expanded_inner,
            ],
            span,
        ))
    }

    /// Expands a list within a quasiquote, handling potential splices.
    fn expand_quasiquote_list(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
        depth: u32,
    ) -> Result<Spanned<Ast>, Error> {
        if elements.is_empty() {
            // Empty list: (quote ())
            return Ok(Self::make_list(
                alloc::vec![
                    Self::make_symbol("quote", span),
                    Self::make_empty_list(span),
                ],
                span,
            ));
        }

        // Expand all elements, tracking splice markers
        let mut expanded_parts: Vec<ExpandedPart> = Vec::new();

        for elem in elements {
            if Self::is_unquote_splicing_form(elem) && depth == 1 {
                // This element should be spliced
                let inner = self.get_unquote_splicing_arg(elem, span)?;
                expanded_parts.push(ExpandedPart::Splice(inner.clone()));
            } else {
                // Regular element
                let expanded = self.expand_quasiquote(elem, depth)?;
                expanded_parts.push(ExpandedPart::Single(expanded));
            }
        }

        // Build the list construction code
        Ok(Self::build_list_construction(expanded_parts, span))
    }

    /// Checks if an AST node is an `(unquote-splicing x)` form.
    fn is_unquote_splicing_form(ast: &Spanned<Ast>) -> bool {
        if let Ast::List(ref elements) = ast.node {
            Self::is_unquote_splicing(elements)
        } else {
            false
        }
    }

    /// Gets the argument from an `(unquote-splicing x)` form.
    fn get_unquote_splicing_arg<'ast>(
        &self,
        ast: &'ast Spanned<Ast>,
        span: Span,
    ) -> Result<&'ast Spanned<Ast>, Error> {
        let location = self.location(span);
        if let Ast::List(ref elements) = ast.node {
            if elements.len() != 2_usize {
                return Err(Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "unquote-splicing",
                        message: "expected exactly 1 argument",
                    },
                    location,
                ));
            }
            elements
                .get(1_usize)
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, location))
        } else {
            Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "unquote-splicing",
                    message: "internal error: not a list",
                },
                location,
            ))
        }
    }

    /// Builds a list construction expression from expanded parts.
    ///
    /// If there are no splices, generates `(list e1 e2 ...)`.
    /// If there are splices, generates `(concat (list e1) splice1 (list e2) ...)`.
    fn build_list_construction(parts: Vec<ExpandedPart>, span: Span) -> Spanned<Ast> {
        let has_splices = parts
            .iter()
            .any(|part| matches!(part, ExpandedPart::Splice(_)));

        if has_splices {
            // Complex case: (concat (list e1) splice1 (list e2) ...)
            let groups = Self::group_for_concat(parts, span);
            let mut concat_args = alloc::vec![Self::make_symbol("concat", span)];
            concat_args.extend(groups);
            Self::make_list(concat_args, span)
        } else {
            // Simple case: (list e1 e2 e3 ...)
            let mut list_args = alloc::vec![Self::make_symbol("list", span)];
            for part in parts {
                if let ExpandedPart::Single(ast) = part {
                    list_args.push(ast);
                }
            }
            Self::make_list(list_args, span)
        }
    }

    /// Groups expanded parts for concat: non-splices are wrapped in `(list ...)`.
    fn group_for_concat(parts: Vec<ExpandedPart>, span: Span) -> Vec<Spanned<Ast>> {
        let mut result = Vec::new();
        let mut current_singles: Vec<Spanned<Ast>> = Vec::new();

        for part in parts {
            match part {
                ExpandedPart::Single(ast) => {
                    current_singles.push(ast);
                }
                ExpandedPart::Splice(ast) => {
                    // Flush accumulated singles as (list ...)
                    if !current_singles.is_empty() {
                        let mut list_call = alloc::vec![Self::make_symbol("list", span)];
                        list_call.append(&mut current_singles);
                        result.push(Self::make_list(list_call, span));
                    }
                    // Add the splice expression directly
                    result.push(ast);
                }
            }
        }

        // Flush remaining singles
        if !current_singles.is_empty() {
            let mut list_call = alloc::vec![Self::make_symbol("list", span)];
            list_call.append(&mut current_singles);
            result.push(Self::make_list(list_call, span));
        }

        result
    }

    /// Expands a vector within a quasiquote.
    ///
    /// Vectors are expanded like lists, then wrapped in `(vec ...)` to preserve
    /// the vector type.
    fn expand_quasiquote_vector(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
        depth: u32,
    ) -> Result<Spanned<Ast>, Error> {
        if elements.is_empty() {
            // Empty vector: (vec nil) or (vec (list))
            return Ok(Self::make_list(
                alloc::vec![
                    Self::make_symbol("vec", span),
                    Self::make_list(alloc::vec![Self::make_symbol("list", span),], span),
                ],
                span,
            ));
        }

        // Expand all elements, tracking splice markers
        let mut expanded_parts: Vec<ExpandedPart> = Vec::new();

        for elem in elements {
            if Self::is_unquote_splicing_form(elem) && depth == 1 {
                let inner = self.get_unquote_splicing_arg(elem, span)?;
                expanded_parts.push(ExpandedPart::Splice(inner.clone()));
            } else {
                let expanded = self.expand_quasiquote(elem, depth)?;
                expanded_parts.push(ExpandedPart::Single(expanded));
            }
        }

        // Build the list construction, then wrap in (vec ...)
        let list_construction = Self::build_list_construction(expanded_parts, span);
        Ok(Self::make_list(
            alloc::vec![Self::make_symbol("vec", span), list_construction,],
            span,
        ))
    }

    /// Wraps an atom in a quote form: `x` -> `(quote x)`.
    pub(super) fn quote_atom(ast: &Spanned<Ast>) -> Spanned<Ast> {
        Spanned::new(
            Ast::List(alloc::vec![
                Spanned::new(Ast::Symbol(alloc::string::String::from("quote")), ast.span),
                ast.clone(),
            ]),
            ast.span,
        )
    }

    // =========================================================================
    // AST Construction Helpers
    // =========================================================================

    /// Creates a symbol AST node.
    pub(super) fn make_symbol(name: &str, span: Span) -> Spanned<Ast> {
        Spanned::new(Ast::Symbol(alloc::string::String::from(name)), span)
    }

    /// Creates a list AST node.
    pub(super) const fn make_list(elements: Vec<Spanned<Ast>>, span: Span) -> Spanned<Ast> {
        Spanned::new(Ast::List(elements), span)
    }

    /// Creates an empty list AST node.
    pub(super) const fn make_empty_list(span: Span) -> Spanned<Ast> {
        Self::make_list(Vec::new(), span)
    }

    /// Creates a quoted symbol: `'name` -> `(quote name)`.
    pub(super) fn make_quoted_symbol(name: &str, span: Span) -> Spanned<Ast> {
        Self::make_list(
            alloc::vec![
                Self::make_symbol("quote", span),
                Self::make_symbol(name, span),
            ],
            span,
        )
    }
}
