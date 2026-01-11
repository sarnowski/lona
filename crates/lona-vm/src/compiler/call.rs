// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Function and intrinsic call compilation.

#[cfg(any(test, feature = "std"))]
use std::vec::Vec;

#[cfg(not(any(test, feature = "std")))]
use alloc::vec::Vec;

use crate::Vaddr;
use crate::bytecode::{encode_abc, op};
use crate::intrinsics::id as intrinsic_id;
use crate::platform::MemorySpace;
use crate::realm::lookup_var_in_ns;
use crate::value::{Value, VarContent, VarSlot, var_flags};

use super::{CompileError, Compiler, MAX_ARGS, MAX_SYMBOL_NAME_LEN};

impl<M: MemorySpace> Compiler<'_, M> {
    /// Compile a list expression (special form, intrinsic call, or function call).
    ///
    /// Resolution order for symbols in call position:
    /// 1. Special forms (hardcoded by name: def, fn*, quote, do, var, match)
    /// 2. Local bindings (function parameters)
    /// 3. Namespace lookup via `*ns*`
    pub(super) fn compile_list(
        &mut self,
        list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        let pair = self
            .proc
            .read_pair(self.mem, list)
            .ok_or(CompileError::InvalidSyntax)?;

        // Check if head is a symbol (could be special form, var, or bound parameter)
        if let Value::Symbol(_) = pair.first {
            // Look up the symbol name - copy to local buffer to avoid borrow conflicts
            let name_str = self
                .proc
                .read_string(self.mem, pair.first)
                .ok_or(CompileError::InvalidSyntax)?;

            let mut name_buf = [0u8; MAX_SYMBOL_NAME_LEN];
            let name_bytes = name_str.as_bytes();
            let name_len = name_bytes.len().min(MAX_SYMBOL_NAME_LEN);
            name_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);
            let name = core::str::from_utf8(&name_buf[..name_len])
                .map_err(|_| CompileError::InvalidSyntax)?;

            // Check for special forms first (these are hardcoded, not looked up via vars)
            if name == "quote" {
                return self.compile_quote(pair.rest, target, temp_base);
            }
            if name == "fn*" {
                return self.compile_fn(pair.rest, target, temp_base);
            }
            if name == "do" {
                return self.compile_do(pair.rest, target, temp_base);
            }
            if name == "var" {
                return self.compile_var(pair.rest, target, temp_base);
            }
            if name == "def" {
                return self.compile_def(pair.rest, target, temp_base);
            }
            if name == "match" {
                // match is a special form but not yet implemented
                // For now, fall through to var lookup which will find it as special form var
            }

            // Check if it's a bound parameter (function value in local scope)
            if self.lookup_binding_by_name(name).is_some() {
                // It's a function call with a bound function
                return self.compile_call(pair.first, pair.rest, target, temp_base);
            }

            // Check if symbol is a captured variable
            if self.lookup_capture(name).is_some() {
                return self.compile_call(pair.first, pair.rest, target, temp_base);
            }

            // Check outer bindings for capture candidates
            if self.lookup_outer_binding(name).is_some() {
                return self.compile_call(pair.first, pair.rest, target, temp_base);
            }

            // Resolve via namespace lookup
            if let Some(var) = self.resolve_symbol(name) {
                return self.compile_var_call(var, pair.rest, target, temp_base);
            }

            // Unknown symbol
            return Err(CompileError::UnboundSymbol);
        }

        // Head is not a symbol - compile it and call
        self.compile_call(pair.first, pair.rest, target, temp_base)
    }

    /// Compile a call where the function is obtained from a var.
    ///
    /// At compile time, we look at the var's root value:
    /// - If it's a `NativeFn`, emit an INTRINSIC instruction directly (optimization)
    /// - Otherwise, emit `VAR_GET` + CALL for late binding
    fn compile_var_call(
        &mut self,
        var: Value,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        let Value::Var(slot_addr) = var else {
            return Err(CompileError::InvalidSyntax);
        };

        // Read var content to check if it's a NativeFn (optimization)
        let slot: VarSlot = self.mem.read(slot_addr);
        let content: VarContent = self.mem.read(slot.content);

        // Special forms cannot be called
        if content.flags & var_flags::SPECIAL_FORM != 0 {
            return Err(CompileError::InvalidSyntax);
        }

        // Optimization: if root is NativeFn and var is native, emit INTRINSIC directly
        // This avoids the VAR_GET overhead for intrinsics
        if content.is_native() {
            if let Value::NativeFn(id) = content.root {
                return self.compile_intrinsic_call(id as u8, arg_list, target, temp_base);
            }
        }

        // General case: emit VAR_GET + CALL for late binding
        self.compile_var_call_late_binding(var, arg_list, target, temp_base)
    }

    /// Compile a var call with late binding (`VAR_GET` + CALL).
    ///
    /// IMPORTANT: Arguments are compiled FIRST, before `VAR_GET`, to preserve
    /// parameter registers (X1..Xn) that may be referenced by arguments.
    fn compile_var_call_late_binding(
        &mut self,
        var: Value,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Collect arguments
        let mut args: Vec<Value> = Vec::new();
        let mut arg_count: u8 = 0;
        let mut current = arg_list;

        while !current.is_nil() {
            let pair = self
                .proc
                .read_pair(self.mem, current)
                .ok_or(CompileError::InvalidSyntax)?;

            arg_count = arg_count
                .checked_add(1)
                .ok_or(CompileError::TooManyArguments)?;
            if arg_count > MAX_ARGS {
                return Err(CompileError::TooManyArguments);
            }

            args.push(pair.first);
            current = pair.rest;
        }

        // Allocate temps: one for var, one for function, then one per argument
        let var_temp = temp_base;
        let fn_temp = temp_base
            .checked_add(1)
            .ok_or(CompileError::ExpressionTooComplex)?;
        let arg_temps_base = temp_base
            .checked_add(2)
            .ok_or(CompileError::ExpressionTooComplex)?;
        let next_temp = arg_temps_base
            .checked_add(arg_count)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // CRITICAL: Compile arguments FIRST, before VAR_GET clobbers X registers.
        // This ensures parameter references (e.g., `y` in `(fn* [y] (f y))`) are
        // read from X1..Xn before we use those registers for the VAR_GET call.
        let mut current_next_temp = next_temp;
        for (i, arg) in args.iter().enumerate() {
            let arg_temp = arg_temps_base
                .checked_add(i as u8)
                .ok_or(CompileError::ExpressionTooComplex)?;
            current_next_temp = self.compile_expr(*arg, arg_temp, current_next_temp)?;
        }

        // Load var as constant
        self.compile_constant(var, var_temp)?;

        // Call VAR_GET to get the function value
        self.chunk
            .emit(encode_abc(op::MOVE, 1, u16::from(var_temp), 0));
        self.chunk
            .emit(encode_abc(op::INTRINSIC, intrinsic_id::VAR_GET, 1, 0));
        // Move result to fn_temp
        self.chunk.emit(encode_abc(op::MOVE, fn_temp, 0, 0));

        // Move argument temps to X1..Xn
        for i in 0..arg_count {
            self.chunk.emit(encode_abc(
                op::MOVE,
                i + 1,
                u16::from(arg_temps_base + i),
                0,
            ));
        }

        // Emit CALL
        self.chunk
            .emit(encode_abc(op::CALL, fn_temp, u16::from(arg_count), 0));

        // Move result to target if needed
        if target != 0 {
            self.chunk.emit(encode_abc(op::MOVE, target, 0, 0));
        }

        Ok(current_next_temp)
    }

    /// Compile a function call.
    ///
    /// The head expression is compiled to get the function, then arguments
    /// are compiled and a CALL instruction is emitted.
    pub(super) fn compile_call(
        &mut self,
        head: Value,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // First, collect arguments
        let mut args: Vec<Value> = Vec::new();
        let mut arg_count: u8 = 0;
        let mut current = arg_list;

        while !current.is_nil() {
            let pair = self
                .proc
                .read_pair(self.mem, current)
                .ok_or(CompileError::InvalidSyntax)?;

            arg_count = arg_count
                .checked_add(1)
                .ok_or(CompileError::TooManyArguments)?;
            if arg_count > MAX_ARGS {
                return Err(CompileError::TooManyArguments);
            }

            args.push(pair.first);
            current = pair.rest;
        }

        // Allocate temps: one for the function, then one per argument
        let fn_temp = temp_base;
        let arg_temps_base = temp_base
            .checked_add(1)
            .ok_or(CompileError::ExpressionTooComplex)?;
        let next_temp = arg_temps_base
            .checked_add(arg_count)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // Compile the head (function) to fn_temp
        let mut current_next_temp = self.compile_expr(head, fn_temp, next_temp)?;

        // Compile each argument to arg temps
        for (i, arg) in args.iter().enumerate() {
            let arg_temp = arg_temps_base
                .checked_add(i as u8)
                .ok_or(CompileError::ExpressionTooComplex)?;
            current_next_temp = self.compile_expr(*arg, arg_temp, current_next_temp)?;
        }

        // Move argument temps to X1..Xn
        for i in 0..arg_count {
            self.chunk.emit(encode_abc(
                op::MOVE,
                i + 1,
                u16::from(arg_temps_base + i),
                0,
            ));
        }

        // Emit CALL: fn_temp holds the function, argc is argument count
        // Result will be in X0
        self.chunk
            .emit(encode_abc(op::CALL, fn_temp, u16::from(arg_count), 0));

        // If target != 0, move X0 to target
        if target != 0 {
            self.chunk.emit(encode_abc(op::MOVE, target, 0, 0));
        }

        Ok(current_next_temp)
    }

    /// Compile an intrinsic call.
    ///
    /// Arguments are first compiled to temp registers, then moved to X1..Xn.
    /// This prevents nested calls from clobbering already-computed arguments.
    /// The INTRINSIC instruction puts the result in X0.
    /// If target != 0, we emit a MOVE to copy X0 to target.
    ///
    /// Returns the next available temp register after compilation.
    pub(super) fn compile_intrinsic_call(
        &mut self,
        intrinsic_id: u8,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // First, collect all arguments while counting
        let mut args: Vec<Value> = Vec::new();
        let mut arg_count: u8 = 0;
        let mut current = arg_list;

        while !current.is_nil() {
            let pair = self
                .proc
                .read_pair(self.mem, current)
                .ok_or(CompileError::InvalidSyntax)?;

            arg_count = arg_count
                .checked_add(1)
                .ok_or(CompileError::TooManyArguments)?;
            if arg_count > MAX_ARGS {
                return Err(CompileError::TooManyArguments);
            }

            args.push(pair.first);
            current = pair.rest;
        }

        // Handle zero-arg case
        if arg_count == 0 {
            self.chunk
                .emit(encode_abc(op::INTRINSIC, intrinsic_id, 0, 0));
            if target != 0 {
                self.chunk.emit(encode_abc(op::MOVE, target, 0, 0));
            }
            return Ok(temp_base);
        }

        // Allocate temp registers for our args: temp_base..temp_base+argc-1
        // Nested calls will use temps starting at temp_base+argc
        let next_temp = temp_base
            .checked_add(arg_count)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // Compile each argument to its temp register
        let mut current_next_temp = next_temp;
        for (i, arg) in args.iter().enumerate() {
            let temp_reg = temp_base
                .checked_add(i as u8)
                .ok_or(CompileError::ExpressionTooComplex)?;
            current_next_temp = self.compile_expr(*arg, temp_reg, current_next_temp)?;
        }

        // Move temps to argument positions X1..Xn
        for i in 0..arg_count {
            self.chunk
                .emit(encode_abc(op::MOVE, i + 1, u16::from(temp_base + i), 0));
        }

        // Emit INTRINSIC instruction
        // Format: INTRINSIC id, arg_count (id in A field, arg_count in B field)
        self.chunk.emit(encode_abc(
            op::INTRINSIC,
            intrinsic_id,
            u16::from(arg_count),
            0,
        ));

        // If target != 0, move X0 to target
        if target != 0 {
            self.chunk.emit(encode_abc(op::MOVE, target, 0, 0));
        }

        Ok(current_next_temp)
    }

    /// Compile the `quote` special form.
    ///
    /// `(quote expr)` returns `expr` unevaluated.
    pub(super) fn compile_quote(
        &mut self,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Get the single argument
        let pair = self
            .proc
            .read_pair(self.mem, arg_list)
            .ok_or(CompileError::InvalidSyntax)?;

        // quote takes exactly one argument
        if !pair.rest.is_nil() {
            return Err(CompileError::InvalidSyntax);
        }

        // Load the quoted expression as a constant (unevaluated)
        self.compile_constant(pair.first, target)?;
        Ok(temp_base)
    }

    /// Compile the `var` special form.
    ///
    /// `(var sym)` returns the var object for the given symbol.
    /// This is also the expansion of reader syntax `#'sym`.
    ///
    /// For qualified symbols like `user/x`, looks up the namespace and var.
    /// For unqualified symbols, looks up via `*ns*` (current namespace).
    pub(super) fn compile_var(
        &mut self,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Get the single argument (must be a symbol)
        let pair = self
            .proc
            .read_pair(self.mem, arg_list)
            .ok_or(CompileError::InvalidSyntax)?;

        // var takes exactly one argument
        if !pair.rest.is_nil() {
            return Err(CompileError::InvalidSyntax);
        }

        // Argument must be a symbol
        if !pair.first.is_symbol() {
            return Err(CompileError::InvalidSyntax);
        }

        // Get the symbol name - copy to local buffer to avoid borrow conflicts
        let name_str = self
            .proc
            .read_string(self.mem, pair.first)
            .ok_or(CompileError::InvalidSyntax)?;

        let mut name_buf = [0u8; MAX_SYMBOL_NAME_LEN];
        let name_bytes = name_str.as_bytes();
        let name_len = name_bytes.len().min(MAX_SYMBOL_NAME_LEN);
        name_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);
        let name =
            core::str::from_utf8(&name_buf[..name_len]).map_err(|_| CompileError::InvalidSyntax)?;

        // Use the compiler's symbol resolution (handles both qualified and unqualified)
        let var = self
            .resolve_symbol(name)
            .ok_or(CompileError::UnboundSymbol)?;

        // Load the var as a constant
        self.compile_constant(var, target)?;
        Ok(temp_base)
    }

    /// Compile the `def` special form.
    ///
    /// Syntax:
    /// - `(def name)` - create unbound var
    /// - `(def name value)` - create var with value
    /// - `(def ^:process-bound name value)` - create process-bound var
    /// - `(def ^%{:doc "..."} name value)` - create var with metadata
    ///
    /// At compile time:
    /// - Creates or finds the var in the current namespace
    /// - For process-bound vars, checks the flag
    ///
    /// At runtime:
    /// - Evaluates the value expression (if present)
    /// - Calls `DEF_ROOT` or `DEF_BINDING` intrinsic to set the value
    /// - Returns the var
    pub(super) fn compile_def(
        &mut self,
        arg_list: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Parse def arguments
        // def syntax: (def [^meta] name [value])
        let (metadata, name_sym, init_expr) = self.parse_def_args(arg_list)?;

        // Get current namespace from *ns*
        let current_ns = self.get_current_namespace()?;
        let Value::Namespace(ns_addr) = current_ns else {
            return Err(CompileError::InvalidSyntax);
        };

        // Get symbol name for creating/finding var
        let name_str = self
            .proc
            .read_string(self.mem, name_sym)
            .ok_or(CompileError::InvalidSyntax)?;

        let mut name_buf = [0u8; MAX_SYMBOL_NAME_LEN];
        let name_bytes = name_str.as_bytes();
        let name_len = name_bytes.len().min(MAX_SYMBOL_NAME_LEN);
        name_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);
        let name =
            core::str::from_utf8(&name_buf[..name_len]).map_err(|_| CompileError::InvalidSyntax)?;

        // Determine if this is a process-bound definition (from metadata)
        let has_process_bound_meta = self.has_process_bound_meta(&metadata);

        // Get or create the var in the namespace at compile time
        // Returns the var and its ACTUAL process-bound status (may differ from metadata
        // when redefining an existing process-bound var without :process-bound metadata)
        let (var, is_process_bound) =
            self.intern_or_get_var(current_ns, ns_addr, name_sym, name, has_process_bound_meta)?;

        // If there's an init expression, compile it and emit the appropriate intrinsic
        let mut next_temp = temp_base;
        if let Some(expr) = init_expr {
            // Compile the value to a temp register
            let value_temp = temp_base;
            let temps_after_value = temp_base
                .checked_add(1)
                .ok_or(CompileError::ExpressionTooComplex)?;
            next_temp = self.compile_expr(expr, value_temp, temps_after_value)?;

            // Allocate temp for var
            let var_temp = next_temp;
            next_temp = next_temp
                .checked_add(1)
                .ok_or(CompileError::ExpressionTooComplex)?;

            // Load var constant
            self.compile_constant(var, var_temp)?;

            // Move var to X1, value to X2
            self.chunk
                .emit(encode_abc(op::MOVE, 1, u16::from(var_temp), 0));
            self.chunk
                .emit(encode_abc(op::MOVE, 2, u16::from(value_temp), 0));

            // Emit the appropriate intrinsic
            if is_process_bound {
                // DEF_BINDING: sets process-local binding
                self.chunk
                    .emit(encode_abc(op::INTRINSIC, intrinsic_id::DEF_BINDING, 2, 0));
            } else {
                // DEF_ROOT: deep copies value to realm and sets var root
                self.chunk
                    .emit(encode_abc(op::INTRINSIC, intrinsic_id::DEF_ROOT, 2, 0));
            }

            // Result is in X0, move to target if needed
            if target != 0 {
                self.chunk.emit(encode_abc(op::MOVE, target, 0, 0));
            }
        } else {
            // No init expression, just return the var
            self.compile_constant(var, target)?;
        }

        // Store metadata if present (deep copies to realm and stores in realm's metadata table)
        if !metadata.is_nil() {
            next_temp = self.emit_store_metadata(var, metadata, next_temp)?;
        }

        Ok(next_temp)
    }

    /// Emit code to store metadata on a var in the realm.
    ///
    /// Compiles the metadata expression, then emits `DEF_META` intrinsic which
    /// deep copies the metadata to the realm and stores it in the realm's
    /// metadata table.
    fn emit_store_metadata(
        &mut self,
        var: Value,
        metadata: Value,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Allocate temps for var and metadata
        let var_temp = temp_base;
        let meta_temp = temp_base
            .checked_add(1)
            .ok_or(CompileError::ExpressionTooComplex)?;
        let next_temp = temp_base
            .checked_add(2)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // Load var constant
        self.compile_constant(var, var_temp)?;

        // Load metadata constant (it's already parsed, just load it)
        self.compile_constant(metadata, meta_temp)?;

        // Move var to X1, metadata to X2
        self.chunk
            .emit(encode_abc(op::MOVE, 1, u16::from(var_temp), 0));
        self.chunk
            .emit(encode_abc(op::MOVE, 2, u16::from(meta_temp), 0));

        // Emit DEF_META intrinsic
        self.chunk
            .emit(encode_abc(op::INTRINSIC, intrinsic_id::DEF_META, 2, 0));

        Ok(next_temp)
    }

    /// Parse def arguments: `[^meta] name [value]`
    ///
    /// Returns `(metadata, name_symbol, optional_init_expr)`
    fn parse_def_args(
        &self,
        arg_list: Value,
    ) -> Result<(Value, Value, Option<Value>), CompileError> {
        // def requires at least a name
        if arg_list.is_nil() {
            return Err(CompileError::InvalidSyntax);
        }

        let first_pair = self
            .proc
            .read_pair(self.mem, arg_list)
            .ok_or(CompileError::InvalidSyntax)?;

        // First element must be a symbol (the name)
        // Note: Metadata is attached to the symbol via reader macros ^meta
        // The reader already attached metadata to the symbol, we read it here
        if !first_pair.first.is_symbol() {
            return Err(CompileError::InvalidSyntax);
        }

        let name_sym = first_pair.first;

        // Get metadata from the symbol (if any)
        let metadata = self.proc.get_metadata_value(name_sym);

        // Check for optional value
        if first_pair.rest.is_nil() {
            // (def name) - unbound var
            return Ok((metadata, name_sym, None));
        }

        let second_pair = self
            .proc
            .read_pair(self.mem, first_pair.rest)
            .ok_or(CompileError::InvalidSyntax)?;

        // (def name value) - exactly two args
        if !second_pair.rest.is_nil() {
            // More than 2 args - error
            return Err(CompileError::InvalidSyntax);
        }

        Ok((metadata, name_sym, Some(second_pair.first)))
    }

    /// Get the current namespace from `*ns*`.
    fn get_current_namespace(&self) -> Result<Value, CompileError> {
        // Look up *ns* var
        let core_ns = self.get_core_ns().ok_or(CompileError::InvalidSyntax)?;
        let ns_var = lookup_var_in_ns(self.realm, self.mem, core_ns, "*ns*")
            .ok_or(CompileError::InvalidSyntax)?;

        // Get the value (process binding or root)
        let ns = self
            .proc
            .var_get(self.mem, ns_var)
            .ok_or(CompileError::InvalidSyntax)?;

        Ok(ns)
    }

    /// Check if metadata contains `:process-bound true`.
    fn has_process_bound_meta(&self, metadata: &Value) -> bool {
        if metadata.is_nil() {
            return false;
        }

        // Metadata should be a map
        if !metadata.is_map() {
            return false;
        }

        // Look for :process-bound key
        let pb_keyword = self
            .proc
            .find_interned_keyword(self.mem, "process-bound")
            .or_else(|| self.realm.find_keyword(self.mem, "process-bound"));

        if let Some(keyword) = pb_keyword {
            // Check if this key exists in the map
            if let Some(value) = self.proc.map_get(self.mem, *metadata, keyword) {
                // Check if value is truthy (not nil and not false)
                return !value.is_nil() && value != Value::bool(false);
            }
        }

        false
    }

    /// Get or create a var in the namespace.
    ///
    /// If the var already exists, returns it (validating process-bound consistency).
    /// If it doesn't exist, creates it at compile time in the realm.
    ///
    /// Returns `(var, is_process_bound)` where `is_process_bound` reflects the var's
    /// actual flags, not just the metadata. This is important for redefining existing
    /// process-bound vars without explicit `:process-bound` metadata.
    fn intern_or_get_var(
        &mut self,
        ns: Value,
        ns_addr: Vaddr,
        name_sym: Value,
        name: &str,
        is_process_bound: bool,
    ) -> Result<(Value, bool), CompileError> {
        // Check if var already exists in namespace
        if let Some(existing_var) = lookup_var_in_ns(self.realm, self.mem, ns, name) {
            // Validate process-bound consistency
            let Value::Var(slot_addr) = existing_var else {
                return Err(CompileError::InvalidSyntax);
            };
            let slot: VarSlot = self.mem.read(slot_addr);
            let content: VarContent = self.mem.read(slot.content);

            // If redefining a non-process-bound var as process-bound, that's an error
            if is_process_bound && !content.is_process_bound() {
                return Err(CompileError::InvalidSyntax);
            }

            // Return the existing var and its ACTUAL process-bound status
            // (which may differ from the metadata if redefining without :process-bound)
            return Ok((existing_var, content.is_process_bound()));
        }

        // Var doesn't exist - create it in the realm at compile time
        if !name_sym.is_symbol() {
            return Err(CompileError::InvalidSyntax);
        }

        // We need to intern the symbol in the realm (it might be on process heap)
        let realm_sym = self
            .realm
            .intern_symbol(self.mem, name)
            .ok_or(CompileError::InternalError)?;
        let Value::Symbol(realm_sym_addr) = realm_sym else {
            return Err(CompileError::InternalError);
        };

        // Determine flags
        let flags = if is_process_bound {
            var_flags::PROCESS_BOUND
        } else {
            0
        };

        // Allocate var in realm
        let var = self
            .realm
            .alloc_var(self.mem, realm_sym_addr, ns_addr, Value::Unbound, flags)
            .ok_or(CompileError::InternalError)?;

        // Add mapping to namespace
        self.realm
            .add_ns_mapping(self.mem, ns, realm_sym, var)
            .ok_or(CompileError::InternalError)?;

        Ok((var, is_process_bound))
    }
}
