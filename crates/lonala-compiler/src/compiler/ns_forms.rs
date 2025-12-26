// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Namespace form compilation.
//!
//! This module handles compilation of the `ns` special form and its clauses:
//! - `(:require [ns :as alias])` - namespace aliasing
//! - `(:require [ns :refer [symbols]])` - symbol importing
//! - `(:use ns)` - refer all (loading deferred)

extern crate alloc;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abx};
use lona_core::span::Span;
use lonala_parser::ast::{Ast, Spanned};

use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind};

impl Compiler<'_, '_, '_> {
    /// Compiles an `ns` special form.
    ///
    /// Syntax: `(ns name clauses*)`
    ///
    /// Supported clauses:
    /// - `(:require [ns.name :as alias])` - loads namespace with alias
    /// - `(:require [ns.name :refer [sym1 sym2]])` - imports specific symbols
    /// - `(:require [ns.name :as alias :refer [sym1 sym2]])` - both
    /// - `(:use ns.name)` - marks for future refer-all (deferred to Task 1.3.4)
    ///
    /// Switches the current namespace for subsequent definitions.
    /// Updates the compile-time namespace context and emits `SetNamespace`
    /// to persist the change at runtime (for REPL session continuity).
    ///
    /// Returns the namespace name symbol.
    pub(super) fn compile_ns(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // At least namespace name is required
        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: "expected (ns name clauses*)",
                },
                self.location(span),
            ));
        }

        // Parse namespace name (must be a symbol)
        let name_expr = args.first().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: "expected (ns name clauses*)",
                },
                self.location(span),
            )
        })?;

        let Ast::Symbol(ref name) = name_expr.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: "namespace name must be a symbol",
                },
                self.location(name_expr.span),
            ));
        };

        // Intern the namespace symbol
        let ns_symbol_id = self.interner.intern(name);

        // Update compile-time namespace context
        self.namespace_ctx.clear_mappings();
        self.namespace_ctx.set_current(ns_symbol_id);

        // Process clauses (args after the namespace name)
        for clause in args.get(1_usize..).unwrap_or(&[]) {
            self.parse_ns_clause(clause)?;
        }

        // Emit SetNamespace opcode for runtime state
        let ns_const = self.add_constant(Constant::Symbol(ns_symbol_id), span)?;
        self.chunk
            .emit(encode_abx(Opcode::SetNamespace, 0, ns_const), span);

        // Return the namespace symbol
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, ns_const), span);

        Ok(ExprResult { register: dest })
    }

    /// Parses a single namespace clause.
    ///
    /// Handles `:require`, `:use`, and `:refer` clauses.
    fn parse_ns_clause(&mut self, clause: &Spanned<Ast>) -> Result<(), Error> {
        let Ast::List(ref elements) = clause.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: "clause must be a list like (:require ...) or (:use ...)",
                },
                self.location(clause.span),
            ));
        };

        let first = elements.first().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: "clause cannot be empty",
                },
                self.location(clause.span),
            )
        })?;

        // Check the clause type keyword
        let Ast::Keyword(ref kw_name) = first.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: "clause must start with :require or :use",
                },
                self.location(first.span),
            ));
        };

        match kw_name.as_str() {
            "require" => self.parse_require_clause(elements, clause.span),
            "use" => self.parse_use_clause(elements, clause.span),
            other => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: if other == "refer" {
                        ":refer is not a top-level clause; use (:require [ns :refer [...]])"
                    } else {
                        "unknown clause type; expected :require or :use"
                    },
                },
                self.location(first.span),
            )),
        }
    }

    /// Parses a `:require` clause.
    ///
    /// Syntax: `(:require [ns.name :as alias] [ns.name :refer [sym1 sym2]] ...)`
    fn parse_require_clause(
        &mut self,
        elements: &[Spanned<Ast>],
        _span: Span,
    ) -> Result<(), Error> {
        // Process each libspec after the :require keyword
        for libspec in elements.get(1_usize..).unwrap_or(&[]) {
            self.parse_require_libspec(libspec)?;
        }
        Ok(())
    }

    /// Parses a single require libspec.
    ///
    /// Libspec forms:
    /// - `[ns.name :as alias]`
    /// - `[ns.name :refer [sym1 sym2]]`
    /// - `[ns.name :as alias :refer [sym1 sym2]]`
    /// - `ns.name` (simple form - just namespace name, no aliases or refers)
    fn parse_require_libspec(&mut self, libspec: &Spanned<Ast>) -> Result<(), Error> {
        match libspec.node {
            // Simple form: just a namespace symbol
            Ast::Symbol(ref ns_name) => {
                // No alias or refers, just record for loading (Task 1.3.4)
                let _ns_id = self.interner.intern(ns_name);
                Ok(())
            }
            // Vector form: [ns.name :as alias :refer [...]]
            Ast::Vector(ref elements) => self.parse_require_vector_libspec(elements, libspec.span),
            // All other forms are invalid
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
            | _ => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: "require libspec must be a symbol or vector like [ns :as alias]",
                },
                self.location(libspec.span),
            )),
        }
    }

    /// Parses a vector libspec like `[ns.name :as alias :refer [sym1 sym2]]`.
    fn parse_require_vector_libspec(
        &mut self,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<(), Error> {
        // First element must be namespace symbol
        let ns_expr = elements.first().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: "require vector cannot be empty",
                },
                self.location(span),
            )
        })?;

        let Ast::Symbol(ref ns_name) = ns_expr.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: "first element of require vector must be namespace symbol",
                },
                self.location(ns_expr.span),
            ));
        };

        let ns_id = self.interner.intern(ns_name);

        // Parse options (key-value pairs after namespace)
        let mut index = 1_usize;
        while index < elements.len() {
            let key_expr = elements.get(index).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "ns",
                        message: "expected keyword option in require vector",
                    },
                    self.location(span),
                )
            })?;

            let Ast::Keyword(ref key_name) = key_expr.node else {
                return Err(Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "ns",
                        message: "expected keyword (:as or :refer) in require vector",
                    },
                    self.location(key_expr.span),
                ));
            };

            match key_name.as_str() {
                "as" => {
                    index = index.saturating_add(1_usize);
                    let alias_expr = elements.get(index).ok_or_else(|| {
                        Error::new(
                            ErrorKind::InvalidSpecialForm {
                                form: "ns",
                                message: ":as requires an alias symbol",
                            },
                            self.location(key_expr.span),
                        )
                    })?;

                    let Ast::Symbol(ref alias_name) = alias_expr.node else {
                        return Err(Error::new(
                            ErrorKind::InvalidSpecialForm {
                                form: "ns",
                                message: ":as value must be a symbol",
                            },
                            self.location(alias_expr.span),
                        ));
                    };

                    let alias_id = self.interner.intern(alias_name);
                    self.namespace_ctx.add_alias(alias_id, ns_id);
                }
                "refer" => {
                    index = index.saturating_add(1_usize);
                    let refer_expr = elements.get(index).ok_or_else(|| {
                        Error::new(
                            ErrorKind::InvalidSpecialForm {
                                form: "ns",
                                message: ":refer requires a vector of symbols",
                            },
                            self.location(key_expr.span),
                        )
                    })?;

                    self.parse_refer_list(refer_expr, ns_name)?;
                }
                other => {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "ns",
                            message: if other == "exclude" {
                                ":exclude is not yet supported"
                            } else if other == "rename" {
                                ":rename is not yet supported"
                            } else {
                                "unknown require option; expected :as or :refer"
                            },
                        },
                        self.location(key_expr.span),
                    ));
                }
            }

            index = index.saturating_add(1_usize);
        }

        Ok(())
    }

    /// Parses a `:refer` list and adds refers to the namespace context.
    fn parse_refer_list(&mut self, refer_expr: &Spanned<Ast>, ns_name: &str) -> Result<(), Error> {
        let Ast::Vector(ref syms) = refer_expr.node else {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "ns",
                    message: ":refer value must be a vector of symbols",
                },
                self.location(refer_expr.span),
            ));
        };

        for sym_expr in syms {
            let Ast::Symbol(ref sym_name) = sym_expr.node else {
                return Err(Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "ns",
                        message: ":refer vector must contain only symbols",
                    },
                    self.location(sym_expr.span),
                ));
            };

            let sym_id = self.interner.intern(sym_name);
            let qualified_name = alloc::format!("{ns_name}/{sym_name}");
            let qualified_id = self.interner.intern(&qualified_name);
            self.namespace_ctx.add_refer(sym_id, qualified_id);
        }

        Ok(())
    }

    /// Parses a `:use` clause.
    ///
    /// Syntax: `(:use ns.name ns.name2 ...)`
    ///
    /// Marks namespaces for "refer all" when loaded. Actual loading is
    /// deferred to Task 1.3.4.
    fn parse_use_clause(&mut self, elements: &[Spanned<Ast>], _span: Span) -> Result<(), Error> {
        // Process each namespace after the :use keyword
        for ns_expr in elements.get(1_usize..).unwrap_or(&[]) {
            let Ast::Symbol(ref ns_name) = ns_expr.node else {
                return Err(Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "ns",
                        message: ":use expects namespace symbols",
                    },
                    self.location(ns_expr.span),
                ));
            };

            let ns_id = self.interner.intern(ns_name);
            self.namespace_ctx.add_pending_use(ns_id);
        }
        Ok(())
    }
}
