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

use super::{Compiler, ExprResult};
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

    // =========================================================================
    // Symbol Compilation
    // =========================================================================

    /// Compiles a symbol as a local or global variable lookup.
    ///
    /// First checks local scopes (for `let` bindings and function parameters),
    /// falling back to global lookup if not found locally.
    pub(super) fn compile_symbol(&mut self, name: &str, span: Span) -> Result<ExprResult, Error> {
        let sym_id = self.interner.intern(name);

        // First, check local variables
        if let Some(local_reg) = self.locals.lookup(sym_id) {
            // Local variable - copy from its register to dest if needed
            let dest = self.alloc_register(span)?;
            if local_reg != dest {
                self.chunk
                    .emit(encode_abc(Opcode::Move, dest, local_reg, 0), span);
            }
            return Ok(ExprResult { register: dest });
        }

        // Not a local, fall back to global lookup
        let dest = self.alloc_register(span)?;
        let const_idx = self.add_constant(Constant::Symbol(sym_id), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }
}
