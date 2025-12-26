// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Expression compilation for literals and symbols.
//!
//! This module handles compilation of simple expressions:
//! - Integer, float, boolean, nil, string, and keyword literals
//! - Symbol lookups (local and global variables)

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abc, encode_abx};
use lona_core::span::Span;

use super::{Compiler, ExprResult, SymbolResolution};
use crate::error::Error;

impl Compiler<'_, '_, '_> {
    // =========================================================================
    // Literal Compilation
    // =========================================================================

    /// Compiles an integer literal.
    pub(super) fn compile_integer(&mut self, value: i64, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let const_idx = self.add_constant(Constant::Integer(value), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a float literal.
    pub(super) fn compile_float(&mut self, value: f64, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let const_idx = self.add_constant(Constant::Float(value), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a boolean literal.
    pub(super) fn compile_bool(&mut self, value: bool, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let opcode = if value {
            Opcode::LoadTrue
        } else {
            Opcode::LoadFalse
        };
        self.chunk.emit(encode_abc(opcode, dest, 0, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a nil literal.
    pub(super) fn compile_nil(&mut self, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a string literal.
    pub(super) fn compile_string(&mut self, value: &str, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let const_idx =
            self.add_constant(Constant::String(alloc::string::String::from(value)), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a keyword literal.
    ///
    /// Keywords are self-evaluating values that evaluate to themselves.
    /// The keyword name is stored without the colon prefix (colon is syntax).
    pub(super) fn compile_keyword(&mut self, name: &str, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let keyword_id = self.interner.intern(name);
        let const_idx = self.add_constant(Constant::Keyword(keyword_id), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    // =========================================================================
    // Collection Literals
    // =========================================================================

    /// Compiles a set literal.
    ///
    /// Set literals are compiled as calls to the `hash-set` function.
    /// For example, `#{1 2 3}` compiles to `(hash-set 1 2 3)`.
    pub(super) fn compile_set(
        &mut self,
        elements: &[lonala_parser::Spanned<lonala_parser::Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Allocate contiguous registers: R_base = hash-set fn, R_base+1..N = elements
        let base = self.next_register;

        // Load the `hash-set` native function into base register
        let hash_set_sym = self.interner.intern("hash-set");
        let dest = self.alloc_register(span)?;
        let const_idx = self.add_constant(Constant::Symbol(hash_set_sym), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, dest, const_idx), span);

        // Compile each element into consecutive registers
        for element in elements {
            let _element_result = self.compile_expr(element)?;
            // Elements are automatically placed in consecutive registers
        }

        // Emit call instruction
        let arg_count = u8::try_from(elements.len()).map_err(|_err| {
            Error::new(crate::error::Kind::TooManyRegisters, self.location(span))
        })?;

        self.chunk
            .emit(encode_abc(Opcode::Call, base, arg_count, 1), span);

        // Result is left in base register
        // Free element registers
        self.free_registers_to(base.saturating_add(1));

        Ok(ExprResult { register: base })
    }

    /// Compiles a vector literal.
    ///
    /// Vector literals are compiled as calls to the `vector` function.
    /// For example, `[1 2 3]` compiles to `(vector 1 2 3)`.
    pub(super) fn compile_vector(
        &mut self,
        elements: &[lonala_parser::Spanned<lonala_parser::Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Allocate contiguous registers: R_base = vector fn, R_base+1..N = elements
        let base = self.next_register;

        // Load the `vector` native function into base register
        let vector_sym = self.interner.intern("vector");
        let dest = self.alloc_register(span)?;
        let const_idx = self.add_constant(Constant::Symbol(vector_sym), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, dest, const_idx), span);

        // Compile each element into consecutive registers
        for element in elements {
            let _element_result = self.compile_expr(element)?;
            // Elements are automatically placed in consecutive registers
        }

        // Emit call instruction
        let arg_count = u8::try_from(elements.len()).map_err(|_err| {
            Error::new(crate::error::Kind::TooManyRegisters, self.location(span))
        })?;

        self.chunk
            .emit(encode_abc(Opcode::Call, base, arg_count, 1), span);

        // Result is left in base register
        // Free element registers
        self.free_registers_to(base.saturating_add(1));

        Ok(ExprResult { register: base })
    }

    /// Compiles a map literal.
    ///
    /// Map literals are compiled as calls to the `hash-map` function.
    /// For example, `{:a 1 :b 2}` compiles to `(hash-map :a 1 :b 2)`.
    pub(super) fn compile_map(
        &mut self,
        elements: &[lonala_parser::Spanned<lonala_parser::Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Allocate contiguous registers: R_base = hash-map fn, R_base+1..N = elements
        let base = self.next_register;

        // Load the `hash-map` native function into base register
        let hash_map_sym = self.interner.intern("hash-map");
        let dest = self.alloc_register(span)?;
        let const_idx = self.add_constant(Constant::Symbol(hash_map_sym), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, dest, const_idx), span);

        // Compile each key-value pair into consecutive registers
        for element in elements {
            let _element_result = self.compile_expr(element)?;
            // Elements are automatically placed in consecutive registers
        }

        // Emit call instruction
        let arg_count = u8::try_from(elements.len()).map_err(|_err| {
            Error::new(crate::error::Kind::TooManyRegisters, self.location(span))
        })?;

        self.chunk
            .emit(encode_abc(Opcode::Call, base, arg_count, 1), span);

        // Result is left in base register
        // Free element registers
        self.free_registers_to(base.saturating_add(1));

        Ok(ExprResult { register: base })
    }

    // =========================================================================
    // Symbol Compilation
    // =========================================================================

    /// Compiles a symbol as a local, upvalue, or global variable lookup.
    ///
    /// Resolution order:
    /// 1. Local variables in the current function (`Move` instruction)
    /// 2. Captured upvalues (`GetUpvalue` instruction)
    /// 3. Global lookup with namespace resolution (`GetGlobal` instruction)
    ///
    /// For global lookups:
    /// - Qualified symbols (`alias/name`) resolve the alias through namespace mappings
    /// - Unqualified symbols check refers first, then qualify with current namespace
    pub(super) fn compile_symbol(&mut self, name: &str, span: Span) -> Result<ExprResult, Error> {
        let sym_id = self.interner.intern(name);

        match self.resolve_symbol(sym_id) {
            SymbolResolution::Local(local_reg) => {
                // Local variable - copy from its register to dest if needed
                let dest = self.alloc_register(span)?;
                if local_reg != dest {
                    self.chunk
                        .emit(encode_abc(Opcode::Move, dest, local_reg, 0), span);
                }
                Ok(ExprResult { register: dest })
            }
            SymbolResolution::Upvalue(upvalue_idx) => {
                // Captured upvalue - load from upvalue array
                let dest = self.alloc_register(span)?;
                self.chunk
                    .emit(encode_abc(Opcode::GetUpvalue, dest, upvalue_idx, 0), span);
                Ok(ExprResult { register: dest })
            }
            SymbolResolution::Global => {
                // Global variable lookup with namespace resolution
                let lookup_sym_id = self.resolve_global_symbol(name, sym_id);

                let dest = self.alloc_register(span)?;
                let const_idx = self.add_constant(Constant::Symbol(lookup_sym_id), span)?;
                self.chunk
                    .emit(encode_abx(Opcode::GetGlobal, dest, const_idx), span);
                Ok(ExprResult { register: dest })
            }
        }
    }

    /// Resolves a global symbol to its fully qualified form.
    ///
    /// Resolution order for qualified symbols (`alias/name`):
    /// 1. Check if `alias` is a namespace alias → resolve to `full.ns/name`
    /// 2. Otherwise, use the symbol as-is (assume it's already a full namespace)
    ///
    /// Resolution order for unqualified symbols:
    /// 1. Check refers map → return qualified symbol if found
    /// 2. Qualify with current namespace → `current_ns/name`
    ///
    /// Used by `compile_symbol` and `compile_var` to ensure consistent resolution.
    pub(super) fn resolve_global_symbol(
        &self,
        name: &str,
        sym_id: lona_core::symbol::Id,
    ) -> lona_core::symbol::Id {
        if let Some((alias_part, local_part)) = name.split_once('/') {
            // Qualified symbol: check if alias_part is a registered alias
            let alias_sym_id = self.interner.intern(alias_part);
            self.namespace_ctx
                .resolve_alias(alias_sym_id)
                .map_or(sym_id, |resolved_ns| {
                    // Resolve alias to full namespace
                    let resolved_ns_name = self.interner.resolve(resolved_ns);
                    let qualified_name = alloc::format!("{resolved_ns_name}/{local_part}");
                    self.interner.intern(&qualified_name)
                })
        } else {
            // Unqualified symbol: check refers, then qualify with current namespace
            self.namespace_ctx.resolve_refer(sym_id).unwrap_or_else(|| {
                // Qualify with current namespace
                let current_ns = self.namespace_ctx.current();
                let current_ns_name = self.interner.resolve(current_ns);
                let qualified_name = alloc::format!("{current_ns_name}/{name}");
                self.interner.intern(&qualified_name)
            })
        }
    }
}
