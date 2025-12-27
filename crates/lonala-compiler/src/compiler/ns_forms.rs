// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Namespace form compilation.
//!
//! This module handles compilation of the `ns` special form and its clauses:
//! - `(:require [ns :as alias])` - namespace aliasing with runtime loading
//! - `(:require [ns :refer [symbols]])` - symbol importing with runtime loading
//! - `(:use ns)` - refer all with runtime loading
//!
//! The compiler emits calls to native primitives that perform actual namespace
//! loading at runtime:
//! - `require` - loads a namespace if not already loaded
//! - `namespace-add-alias` - adds an alias to the current namespace
//! - `namespace-add-refer` - adds a referred var to the current namespace

extern crate alloc;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abc, encode_abx};
use lona_core::span::Span;
use lona_core::symbol;
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
    ///
    /// Emits bytecode to load the namespace at runtime.
    fn parse_require_libspec(&mut self, libspec: &Spanned<Ast>) -> Result<(), Error> {
        match libspec.node {
            // Simple form: just a namespace symbol
            Ast::Symbol(ref ns_name) => {
                // Emit runtime require call
                let ns_id = self.interner.intern(ns_name);
                self.emit_require_call(ns_id, libspec.span)?;
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
    ///
    /// Emits bytecode to load the namespace and set up aliases/refers at runtime.
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

        // Emit runtime require call FIRST to ensure namespace is loaded
        self.emit_require_call(ns_id, ns_expr.span)?;

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

                    // Track at compile-time for symbol resolution
                    self.namespace_ctx.add_alias(alias_id, ns_id);

                    // Emit runtime alias call
                    self.emit_add_alias_call(alias_id, ns_id, alias_expr.span)?;
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

                    self.parse_refer_list(refer_expr, ns_name, ns_id)?;
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
    ///
    /// Emits bytecode to register each referred symbol at runtime.
    fn parse_refer_list(
        &mut self,
        refer_expr: &Spanned<Ast>,
        ns_name: &str,
        _ns_id: symbol::Id,
    ) -> Result<(), Error> {
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

            // Track at compile-time for symbol resolution
            self.namespace_ctx.add_refer(sym_id, qualified_id);

            // Emit runtime refer call
            self.emit_add_refer_call(sym_id, qualified_id, sym_expr.span)?;
        }

        Ok(())
    }

    /// Parses a `:use` clause.
    ///
    /// Syntax: `(:use ns.name ns.name2 ...)`
    ///
    /// Loads the namespace and refers all its public symbols.
    /// `:use` is essentially `(:require [ns :refer :all])`.
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

            // Emit runtime require call
            self.emit_require_call(ns_id, ns_expr.span)?;

            // Emit runtime use-all call to refer all public symbols
            // This is implemented by calling a special native that iterates
            // over ns-publics and adds each as a refer
            self.emit_use_all_call(ns_id, ns_expr.span)?;
        }
        Ok(())
    }

    // =========================================================================
    // Helper Methods for Emitting Runtime Calls
    // =========================================================================

    /// Emits bytecode to call `(require 'ns)`.
    ///
    /// This loads the namespace at runtime if not already loaded.
    fn emit_require_call(&mut self, ns_id: symbol::Id, span: Span) -> Result<(), Error> {
        let checkpoint = self.next_register;

        // Load the `require` function (registered as unqualified name)
        let require_sym = self.interner.intern("require");
        let require_const = self.add_constant(Constant::Symbol(require_sym), span)?;
        let func_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, func_reg, require_const), span);

        // Load the namespace symbol as argument
        let ns_const = self.add_constant(Constant::Symbol(ns_id), span)?;
        let arg_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, arg_reg, ns_const), span);

        // Call require with 1 argument
        self.chunk
            .emit(encode_abc(Opcode::Call, func_reg, 1_u8, 1_u8), span);

        // Free registers
        self.free_registers_to(checkpoint);

        Ok(())
    }

    /// Emits bytecode to call `(namespace-add-alias 'alias 'ns)`.
    ///
    /// This registers an alias in the current namespace at runtime.
    fn emit_add_alias_call(
        &mut self,
        alias_id: symbol::Id,
        ns_id: symbol::Id,
        span: Span,
    ) -> Result<(), Error> {
        let checkpoint = self.next_register;

        // Load the `namespace-add-alias` function (registered as unqualified name)
        let func_sym = self.interner.intern("namespace-add-alias");
        let func_const = self.add_constant(Constant::Symbol(func_sym), span)?;
        let func_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, func_reg, func_const), span);

        // Load the alias symbol as first argument
        let alias_const = self.add_constant(Constant::Symbol(alias_id), span)?;
        let alias_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, alias_reg, alias_const), span);

        // Load the namespace symbol as second argument
        let ns_const = self.add_constant(Constant::Symbol(ns_id), span)?;
        let ns_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, ns_reg, ns_const), span);

        // Call with 2 arguments
        self.chunk
            .emit(encode_abc(Opcode::Call, func_reg, 2_u8, 1_u8), span);

        // Free registers
        self.free_registers_to(checkpoint);

        Ok(())
    }

    /// Emits bytecode to call `(namespace-add-refer 'sym #'ns/sym)`.
    ///
    /// This refers a var from another namespace into the current namespace.
    fn emit_add_refer_call(
        &mut self,
        sym_id: symbol::Id,
        qualified_id: symbol::Id,
        span: Span,
    ) -> Result<(), Error> {
        let checkpoint = self.next_register;

        // Load the `namespace-add-refer` function (registered as unqualified name)
        let func_sym = self.interner.intern("namespace-add-refer");
        let func_const = self.add_constant(Constant::Symbol(func_sym), span)?;
        let func_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, func_reg, func_const), span);

        // Load the local symbol name as first argument
        let sym_const = self.add_constant(Constant::Symbol(sym_id), span)?;
        let sym_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, sym_reg, sym_const), span);

        // Load the var reference (using GetGlobalVar to get the Var, not its value)
        let qualified_const = self.add_constant(Constant::Symbol(qualified_id), span)?;
        let var_reg = self.alloc_register(span)?;
        self.chunk.emit(
            encode_abx(Opcode::GetGlobalVar, var_reg, qualified_const),
            span,
        );

        // Call with 2 arguments
        self.chunk
            .emit(encode_abc(Opcode::Call, func_reg, 2_u8, 1_u8), span);

        // Free registers
        self.free_registers_to(checkpoint);

        Ok(())
    }

    /// Emits bytecode to implement `:use` (refer all public vars).
    ///
    /// This calls `(namespace-use-all 'ns)` which iterates over `ns-publics`
    /// and adds each public var as a refer. This is a dedicated native to
    /// avoid needing runtime iteration in bytecode.
    fn emit_use_all_call(&mut self, ns_id: symbol::Id, span: Span) -> Result<(), Error> {
        let checkpoint = self.next_register;

        // Load the `namespace-use-all` function (registered as unqualified name)
        let func_sym = self.interner.intern("namespace-use-all");
        let func_const = self.add_constant(Constant::Symbol(func_sym), span)?;
        let func_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, func_reg, func_const), span);

        // Load the namespace symbol as argument
        let ns_const = self.add_constant(Constant::Symbol(ns_id), span)?;
        let arg_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, arg_reg, ns_const), span);

        // Call with 1 argument
        self.chunk
            .emit(encode_abc(Opcode::Call, func_reg, 1_u8, 1_u8), span);

        // Free registers
        self.free_registers_to(checkpoint);

        Ok(())
    }
}
