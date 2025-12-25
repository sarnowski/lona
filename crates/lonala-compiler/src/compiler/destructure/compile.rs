// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Bytecode compilation for destructuring patterns.
//!
//! This module implements the `Compiler` methods that emit bytecode for
//! sequential and associative destructuring patterns.

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abc, encode_abx, encode_asbx};
use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::Spanned;

use super::{Ast, Binding, MapPattern, SeqPattern};
use crate::compiler::Compiler;
use crate::error::{Error, Kind as ErrorKind};

/// The kind of key used in shorthand map destructuring.
///
/// Used by `compile_key_binding` to determine how to construct
/// the lookup key from a symbol name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyKind {
    /// `:keys` - look up by keyword (`:symbol_name`)
    Keyword,
    /// `:strs` - look up by string (`"symbol_name"`)
    String,
    /// `:syms` - look up by symbol (`'symbol_name`)
    Symbol,
}

/// Finds the default expression for a symbol in the defaults list.
///
/// Returns a reference to the default AST if found.
fn find_default(
    defaults: &[(symbol::Id, Spanned<Ast>)],
    sym_id: symbol::Id,
) -> Option<&Spanned<Ast>> {
    for entry in defaults {
        if entry.0 == sym_id {
            return Some(&entry.1);
        }
    }
    None
}

impl Compiler<'_, '_, '_> {
    /// Compiles a sequential destructuring binding.
    ///
    /// Generates bytecode to destructure the collection in `source_reg` according
    /// to the given `pattern`, creating local bindings for each named element.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The parsed sequential pattern to compile
    /// * `source_reg` - Register containing the collection to destructure
    /// * `span` - Source span for error reporting
    ///
    /// # Algorithm
    ///
    /// 1. If `:as` binding present: copy source to new register, define local
    /// 2. Initialize cursor to source collection
    /// 3. For each positional item:
    ///    - Call `(first cursor)` to get head element
    ///    - Bind head (symbol → define local, ignore → free, nested → recurse)
    ///    - Call `(rest cursor)` to advance cursor
    /// 4. For rest binding (`& rest`): bind cursor directly
    ///
    /// # Errors
    ///
    /// Returns an error if register allocation fails.
    ///
    /// # Register Management
    ///
    /// Registers allocated for symbol bindings are NOT freed - they remain
    /// reserved for the duration of the binding's scope. Only temporary
    /// registers (for ignored bindings and intermediate cursor values) are
    /// reclaimed.
    ///
    /// The caller is responsible for managing scope and freeing binding
    /// registers when the scope ends (typically via `let` or `fn` cleanup).
    #[inline]
    pub fn compile_sequential_binding(
        &mut self,
        pattern: &SeqPattern,
        source_reg: u8,
        span: Span,
    ) -> Result<(), Error> {
        // 1. Handle :as binding first (binds to original collection)
        if let Some(as_sym) = pattern.as_binding {
            let as_reg = self.alloc_register(span)?;
            self.chunk
                .emit(encode_abc(Opcode::Move, as_reg, source_reg, 0), span);
            self.locals.define(as_sym, as_reg);
            // Note: as_reg is NOT freed - it's a live local binding
        }

        // 2. Initialize cursor to source collection
        // The cursor is a temporary that tracks our position in the collection
        let mut cursor_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::Move, cursor_reg, source_reg, 0), span);

        // 3. Process each positional item
        for binding in &pattern.items {
            // Call (first cursor) to get the head element
            let head_reg = self.emit_global_call("first", cursor_reg, span)?;

            // Bind the head based on binding type
            match *binding {
                Binding::Symbol(sym_id) => {
                    // Define local variable pointing to head_reg
                    // head_reg is now a live local - do NOT free it
                    self.locals.define(sym_id, head_reg);
                }
                Binding::Ignore => {
                    // Discard head - register can be reused immediately
                    self.free_registers_to(head_reg);
                }
                Binding::Seq(ref nested_pattern) => {
                    // Recursively compile nested pattern
                    // After recursion, nested bindings occupy registers above head_reg
                    // Do NOT free head_reg as nested locals may point to registers >= head_reg
                    self.compile_sequential_binding(nested_pattern, head_reg, span)?;
                    // Note: We cannot safely free head_reg here because nested
                    // bindings may have allocated registers after it. The caller
                    // handles cleanup when the entire binding scope ends.
                }
                Binding::Map(ref nested_pattern) => {
                    // Recursively compile nested map pattern
                    self.compile_map_binding(nested_pattern, head_reg, span)?;
                }
            }

            // Call (rest cursor) to advance to next element
            // Allocate a new register for the new cursor value
            let new_cursor = self.emit_global_call("rest", cursor_reg, span)?;

            // The old cursor is no longer needed, but we cannot safely free it
            // back to cursor_reg because that would free any local bindings that
            // were allocated after cursor_reg. Instead, we just update our cursor.
            // The old cursor register becomes "dead" but we don't reclaim it here.
            // This is acceptable overhead - register cleanup happens at scope end.
            cursor_reg = new_cursor;
        }

        // 4. Handle rest binding (& rest)
        if let Some(ref rest_binding) = pattern.rest {
            match **rest_binding {
                Binding::Symbol(sym_id) => {
                    // Cursor already contains remaining elements as a list
                    // cursor_reg is now a live local - do NOT free it
                    self.locals.define(sym_id, cursor_reg);
                }
                Binding::Ignore => {
                    // Discard remaining elements - cursor can be reused
                    self.free_registers_to(cursor_reg);
                }
                Binding::Seq(ref nested_pattern) => {
                    // Recursively compile nested pattern on remaining elements
                    self.compile_sequential_binding(nested_pattern, cursor_reg, span)?;
                    // Same as above - don't free as nested bindings may use higher registers
                }
                Binding::Map(ref nested_pattern) => {
                    // Recursively compile nested map pattern on remaining elements
                    self.compile_map_binding(nested_pattern, cursor_reg, span)?;
                }
            }
        } else {
            // No rest binding - cursor is temporary, free it
            self.free_registers_to(cursor_reg);
        }

        Ok(())
    }

    /// Compiles an associative (map) destructuring binding.
    ///
    /// Generates bytecode to destructure the map in `source_reg` according
    /// to the given `pattern`, creating local bindings for extracted values.
    ///
    /// # Algorithm
    ///
    /// 1. If `:as` binding present: copy source to new register, define local
    /// 2. For each binding (`:keys`, `:strs`, `:syms`, explicit):
    ///    - Load appropriate key constant
    ///    - Call `(get map key)` to extract value
    ///    - If symbol has `:or` default: emit nil check and conditional default
    ///    - Define local for symbol
    ///
    /// # Errors
    ///
    /// Returns an error if register allocation fails or if jump offsets exceed limits.
    #[inline]
    pub fn compile_map_binding(
        &mut self,
        pattern: &MapPattern,
        source_reg: u8,
        span: Span,
    ) -> Result<(), Error> {
        // 1. Handle :as binding first (binds to original map)
        if let Some(as_sym) = pattern.as_binding {
            let as_reg = self.alloc_register(span)?;
            self.chunk
                .emit(encode_abc(Opcode::Move, as_reg, source_reg, 0), span);
            self.locals.define(as_sym, as_reg);
            // Note: as_reg is NOT freed - it's a live local binding
        }

        // 2. Process :keys bindings (keyword keys)
        for &sym_id in &pattern.keys {
            self.compile_key_binding(
                sym_id,
                source_reg,
                KeyKind::Keyword,
                &pattern.defaults,
                span,
            )?;
        }

        // 3. Process :strs bindings (string keys)
        for &sym_id in &pattern.strs {
            self.compile_key_binding(sym_id, source_reg, KeyKind::String, &pattern.defaults, span)?;
        }

        // 4. Process :syms bindings (symbol keys)
        for &sym_id in &pattern.syms {
            self.compile_key_binding(sym_id, source_reg, KeyKind::Symbol, &pattern.defaults, span)?;
        }

        // 5. Process explicit bindings (binding pattern -> key expression)
        for entry in &pattern.explicit {
            self.compile_explicit_binding(&entry.0, source_reg, &entry.1, &pattern.defaults, span)?;
        }

        Ok(())
    }

    /// Compiles a key-based binding for `:keys`, `:strs`, or `:syms`.
    ///
    /// The symbol name is used to construct the key:
    /// - `KeyKind::Keyword`: `:symbol_name`
    /// - `KeyKind::String`: `"symbol_name"`
    /// - `KeyKind::Symbol`: `'symbol_name`
    fn compile_key_binding(
        &mut self,
        sym_id: symbol::Id,
        source_reg: u8,
        key_kind: KeyKind,
        defaults: &[(symbol::Id, Spanned<Ast>)],
        span: Span,
    ) -> Result<(), Error> {
        // Get the symbol name to construct the key
        let sym_name = self.interner.resolve(sym_id);

        // Allocate permanent register for this binding first
        let binding_reg = self.alloc_register(span)?;

        // Track checkpoint for temps
        let checkpoint = self.next_register;

        // Load key constant based on kind (temp register)
        let key_reg = self.alloc_register(span)?;
        let key_constant = match key_kind {
            KeyKind::Keyword => {
                let keyword_sym = self.interner.intern(&sym_name);
                Constant::Keyword(keyword_sym)
            }
            KeyKind::String => Constant::String(sym_name),
            KeyKind::Symbol => {
                // For symbol keys, we use the symbol value directly
                Constant::Symbol(sym_id)
            }
        };
        let const_idx = self.add_constant(key_constant, span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, key_reg, const_idx), span);

        // Call (get map key) - result in temp register
        let result_reg = self.emit_global_call_2("get", source_reg, key_reg, span)?;

        // Move result to permanent binding register
        self.chunk
            .emit(encode_abc(Opcode::Move, binding_reg, result_reg, 0), span);

        // Free all temps (key_reg and call temps)
        self.free_registers_to(checkpoint);

        // Check if this symbol has a default and apply if needed
        if let Some(default_ast) = find_default(defaults, sym_id) {
            self.apply_default_if_nil(binding_reg, default_ast, span)?;
        }

        // Define local for symbol
        self.locals.define(sym_id, binding_reg);

        Ok(())
    }

    /// Compiles an explicit binding like `{a :key-a}`, `{[a b] :point}`, or `{{:keys [x]} :inner}`.
    ///
    /// The key expression is compiled to produce the lookup key.
    /// The binding can be a symbol, sequential pattern, or map pattern.
    fn compile_explicit_binding(
        &mut self,
        binding: &Binding,
        source_reg: u8,
        key_ast: &Spanned<Ast>,
        defaults: &[(symbol::Id, Spanned<Ast>)],
        span: Span,
    ) -> Result<(), Error> {
        match *binding {
            Binding::Symbol(sym_id) => {
                // For symbol bindings, allocate permanent register BEFORE checkpoint
                // so it won't be freed when we clean up temps
                let binding_reg = self.alloc_register(span)?;

                // Track checkpoint for temps (after permanent register)
                let checkpoint = self.next_register;

                // Compile the key expression (temp register)
                let key_result = self.compile_expr(key_ast)?;
                let key_reg = key_result.register;

                // Call (get map key) - result in temp register
                let result_reg = self.emit_global_call_2("get", source_reg, key_reg, span)?;

                // Move result to permanent binding register
                self.chunk
                    .emit(encode_abc(Opcode::Move, binding_reg, result_reg, 0), span);

                // Free all temps (key expression and call temps), but NOT binding_reg
                self.free_registers_to(checkpoint);

                // Check if this symbol has a default and apply if needed
                if let Some(default_ast) = find_default(defaults, sym_id) {
                    self.apply_default_if_nil(binding_reg, default_ast, span)?;
                }

                // Define local for symbol
                self.locals.define(sym_id, binding_reg);
            }
            Binding::Ignore => {
                // Track checkpoint for temps
                let checkpoint = self.next_register;

                // Compile the key expression (temp register)
                let key_result = self.compile_expr(key_ast)?;
                let key_reg = key_result.register;

                // Call (get map key) - result discarded
                let _result_reg = self.emit_global_call_2("get", source_reg, key_reg, span)?;

                // Free all temps - the value is discarded
                self.free_registers_to(checkpoint);
            }
            Binding::Seq(ref nested_pattern) => {
                // Compile the key expression (temp register)
                let key_result = self.compile_expr(key_ast)?;
                let key_reg = key_result.register;

                // Call (get map key) - result in temp register
                let result_reg = self.emit_global_call_2("get", source_reg, key_reg, span)?;

                // Recursively compile nested sequential pattern on the extracted value
                // result_reg holds the value extracted from the map
                self.compile_sequential_binding(nested_pattern, result_reg, span)?;

                // Note: We do NOT free temp registers here because:
                // 1. Nested bindings allocate permanent registers above our temps
                // 2. Calling free_registers_to() would free those permanent registers
                // 3. The caller handles cleanup when the entire binding scope ends
                // This matches the pattern in compile_sequential_binding lines 126-130
            }
            Binding::Map(ref nested_pattern) => {
                // Compile the key expression (temp register)
                let key_result = self.compile_expr(key_ast)?;
                let key_reg = key_result.register;

                // Call (get map key) - result in temp register
                let result_reg = self.emit_global_call_2("get", source_reg, key_reg, span)?;

                // Recursively compile nested map pattern on the extracted value
                // result_reg holds the value extracted from the map
                self.compile_map_binding(nested_pattern, result_reg, span)?;

                // Note: Same as Binding::Seq - don't free temps to preserve nested bindings
            }
        }

        Ok(())
    }

    /// Applies a default value if the current value in `value_reg` is nil.
    ///
    /// Emits bytecode equivalent to:
    /// ```text
    /// if (= value nil) then
    ///   value = default_expr
    /// ```
    ///
    /// Note: Per Clojure semantics, defaults apply only when value is nil,
    /// not when value is false.
    fn apply_default_if_nil(
        &mut self,
        value_reg: u8,
        default_ast: &Spanned<Ast>,
        span: Span,
    ) -> Result<(), Error> {
        // Load nil constant for comparison
        let nil_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::LoadNil, nil_reg, 0, 0), span);

        // Compare value with nil using Eq opcode
        // Eq stores result (bool) in destination register
        let cmp_reg = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::Eq, cmp_reg, value_reg, nil_reg), span);

        // Free nil_reg (no longer needed)
        self.free_registers_to(cmp_reg.saturating_add(1));

        // JumpIfNot: if NOT equal to nil, skip the default assignment
        // (i.e., if value != nil, jump past the default code)
        let jump_idx = self.chunk.emit(
            encode_asbx(Opcode::JumpIfNot, cmp_reg, 0), // Offset will be patched
            span,
        );

        // Free comparison result register
        self.free_registers_to(cmp_reg);

        // Compile default expression
        let default_result = self.compile_expr(default_ast)?;

        // Move default value to value_reg if different
        if default_result.register != value_reg {
            self.chunk.emit(
                encode_abc(Opcode::Move, value_reg, default_result.register, 0),
                default_ast.span,
            );
        }

        // Free temps from default expression
        self.free_registers_to(value_reg.saturating_add(1));

        // Patch the jump offset
        let current_idx = self.chunk.len();
        let offset = current_idx.saturating_sub(jump_idx).saturating_sub(1);
        let offset_i16 = i16::try_from(offset)
            .map_err(|_err| Error::new(ErrorKind::JumpTooLarge, self.location(span)))?;
        self.chunk.patch(
            jump_idx,
            encode_asbx(Opcode::JumpIfNot, cmp_reg, offset_i16),
        );

        Ok(())
    }

    /// Emits bytecode to call a global function with one argument.
    ///
    /// This is used by destructuring to call `first` and `rest` primitives.
    /// Using `GetGlobal` ensures late binding (hot-patching works) and avoids
    /// local capture issues if the user shadows these names.
    ///
    /// # Arguments
    ///
    /// * `fn_name` - Name of the global function to call
    /// * `arg_reg` - Register containing the argument
    /// * `span` - Source span for bytecode attribution
    ///
    /// # Returns
    ///
    /// The register containing the function's return value.
    ///
    /// # Generated Bytecode
    ///
    /// ```text
    /// R_base = GetGlobal fn_name   ; Load function
    /// R_base+1 = Move arg_reg      ; Copy argument (for call convention)
    /// Call R_base 1 1              ; Call with 1 arg, 1 result
    /// ; Result is in R_base
    /// ```
    fn emit_global_call(&mut self, fn_name: &str, arg_reg: u8, span: Span) -> Result<u8, Error> {
        // Allocate base register for function
        let base = self.alloc_register(span)?;

        // Load the function via GetGlobal
        let fn_sym = self.interner.intern(fn_name);
        let const_idx = self.add_constant(Constant::Symbol(fn_sym), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, base, const_idx), span);

        // Allocate register for argument and copy it
        let arg_dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::Move, arg_dest, arg_reg, 0), span);

        // Emit call: base register, 1 argument, expecting 1 result
        self.chunk
            .emit(encode_abc(Opcode::Call, base, 1_u8, 1_u8), span);

        // Free the argument register (but keep base which holds result)
        self.free_registers_to(base.saturating_add(1));

        // Result is left in base register
        Ok(base)
    }

    /// Emits bytecode to call a global function with two arguments.
    ///
    /// This is used by map destructuring to call `get` primitive.
    /// Using `GetGlobal` ensures late binding (hot-patching works) and avoids
    /// local capture issues if the user shadows these names.
    ///
    /// # Arguments
    ///
    /// * `fn_name` - Name of the global function to call
    /// * `arg1_reg` - Register containing the first argument
    /// * `arg2_reg` - Register containing the second argument
    /// * `span` - Source span for bytecode attribution
    ///
    /// # Returns
    ///
    /// The register containing the function's return value.
    ///
    /// # Generated Bytecode
    ///
    /// ```text
    /// R_base = GetGlobal fn_name   ; Load function
    /// R_base+1 = Move arg1_reg     ; Copy first argument
    /// R_base+2 = Move arg2_reg     ; Copy second argument
    /// Call R_base 2 1              ; Call with 2 args, 1 result
    /// ; Result is in R_base
    /// ```
    fn emit_global_call_2(
        &mut self,
        fn_name: &str,
        arg1_reg: u8,
        arg2_reg: u8,
        span: Span,
    ) -> Result<u8, Error> {
        // Allocate base register for function
        let base = self.alloc_register(span)?;

        // Load the function via GetGlobal
        let fn_sym = self.interner.intern(fn_name);
        let const_idx = self.add_constant(Constant::Symbol(fn_sym), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, base, const_idx), span);

        // Allocate register for first argument and copy it
        let arg1_dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::Move, arg1_dest, arg1_reg, 0), span);

        // Allocate register for second argument and copy it
        let arg2_dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::Move, arg2_dest, arg2_reg, 0), span);

        // Emit call: base register, 2 arguments, expecting 1 result
        self.chunk
            .emit(encode_abc(Opcode::Call, base, 2_u8, 1_u8), span);

        // Free the argument registers (but keep base which holds result)
        self.free_registers_to(base.saturating_add(1));

        // Result is left in base register
        Ok(base)
    }
}
