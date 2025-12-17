// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Compiler from Lonala AST to bytecode.
//!
//! Transforms parsed [`Ast`] expressions into executable [`Chunk`] bytecode.
//! This module implements the core compilation logic for the Lonala language.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lona_core::chunk::{Chunk, Constant};
use lona_core::opcode::{
    Opcode, RK_MAX_CONSTANT, encode_abc, encode_abx, encode_asbx, rk_constant, rk_register,
};
use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use crate::error::Error;

pub mod conversion;
pub mod macros;

pub use macros::{MacroDefinition, MacroExpander, MacroExpansionError, MacroRegistry};

#[cfg(test)]
mod tests;

/// Result type for parsing `fn` arguments: (optional name, params AST, body expressions).
type FnArgsResult<'args> = (
    Option<alloc::string::String>,
    &'args Spanned<Ast>,
    &'args [Spanned<Ast>],
);

/// Represents a part of a quasiquote expansion - either a single value or
/// a sequence to be spliced.
enum ExpandedPart {
    /// A single element (not spliced).
    Single(Spanned<Ast>),
    /// A sequence to be spliced into the parent.
    Splice(Spanned<Ast>),
}

/// Tracks local variable bindings across nested scopes.
///
/// Used to implement `let` bindings and function parameters. Each scope
/// maps symbol IDs to the register where that variable is stored.
struct LocalEnv {
    /// Stack of scopes, each mapping symbol ID to register.
    scopes: Vec<BTreeMap<symbol::Id, u8>>,
}

impl LocalEnv {
    /// Creates a new empty local environment.
    const fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    /// Pushes a new scope (entering `let`, `fn`, etc.).
    fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    /// Pops the current scope (exiting `let`, `fn`, etc.).
    fn pop_scope(&mut self) {
        let _: Option<BTreeMap<symbol::Id, u8>> = self.scopes.pop();
    }

    /// Defines a local variable in the current scope.
    fn define(&mut self, name: symbol::Id, register: u8) {
        if let Some(scope) = self.scopes.last_mut() {
            let _: Option<u8> = scope.insert(name, register);
        }
    }

    /// Looks up a local variable, searching from innermost to outermost scope.
    fn lookup(&self, name: symbol::Id) -> Option<u8> {
        for scope in self.scopes.iter().rev() {
            if let Some(&reg) = scope.get(&name) {
                return Some(reg);
            }
        }
        None
    }
}

/// Result of compiling an expression.
///
/// Contains the register where the expression's value is stored after
/// the compiled instructions execute.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ExprResult {
    /// Register containing the expression's result.
    pub register: u8,
}

/// Maximum depth for nested macro expansions.
///
/// This limit prevents infinite macro recursion where a macro expands
/// to code that calls itself (directly or indirectly).
const MAX_MACRO_EXPANSION_DEPTH: usize = 256;

/// Compiler state for transforming AST to bytecode.
///
/// Manages register allocation, constant pool, and instruction emission
/// during compilation of a single chunk (function or top-level code).
///
/// # Macro Expansion
///
/// The compiler supports optional macro expansion through the `MacroExpander` trait.
/// When an expander is provided, macro calls are expanded at compile time.
/// Without an expander, macro calls are compiled as regular function calls.
pub struct Compiler<'interner, 'registry, 'expander> {
    /// The bytecode chunk being built.
    chunk: Chunk,
    /// Symbol interner for looking up and creating symbol IDs.
    interner: &'interner mut symbol::Interner,
    /// Next available register.
    next_register: u8,
    /// Maximum register used so far (for tracking chunk metadata).
    max_register: u8,
    /// Local variable bindings across nested scopes.
    locals: LocalEnv,
    /// Registry of macro definitions.
    /// Macros defined during this compilation are added here.
    registry: &'registry mut MacroRegistry,
    /// Optional macro expander for compile-time expansion.
    expander: Option<&'expander mut dyn MacroExpander>,
    /// Current depth of nested macro expansions.
    /// Used to detect and prevent infinite macro recursion.
    macro_expansion_depth: usize,
}

impl<'interner, 'registry, 'expander> Compiler<'interner, 'registry, 'expander> {
    /// Creates a new compiler with the given symbol interner and macro registry.
    ///
    /// This creates a compiler without macro expansion capability. Macro calls
    /// will be compiled as regular function calls.
    #[inline]
    #[must_use]
    pub fn new(
        interner: &'interner mut symbol::Interner,
        registry: &'registry mut MacroRegistry,
    ) -> Self {
        Self {
            chunk: Chunk::new(),
            interner,
            next_register: 0,
            max_register: 0,
            locals: LocalEnv::new(),
            registry,
            expander: None,
            macro_expansion_depth: 0,
        }
    }

    /// Creates a new compiler with macro expansion capability.
    ///
    /// The expander will be used to execute macro transformers at compile time,
    /// expanding macro calls into their expanded forms before compilation.
    #[inline]
    #[must_use]
    pub fn with_expander(
        interner: &'interner mut symbol::Interner,
        registry: &'registry mut MacroRegistry,
        expander: &'expander mut dyn MacroExpander,
    ) -> Self {
        Self {
            chunk: Chunk::new(),
            interner,
            next_register: 0,
            max_register: 0,
            locals: LocalEnv::new(),
            registry,
            expander: Some(expander),
            macro_expansion_depth: 0,
        }
    }

    /// Returns `true` if the given symbol is defined as a macro.
    #[inline]
    #[must_use]
    pub fn is_macro(&self, symbol: symbol::Id) -> bool {
        self.registry.contains(symbol)
    }

    /// Returns the macro definition for a symbol, if it exists.
    #[inline]
    #[must_use]
    pub fn get_macro(&self, symbol: symbol::Id) -> Option<&MacroDefinition> {
        self.registry.get(symbol)
    }

    /// Returns `true` if a macro with the given name is defined.
    ///
    /// This is a convenience method that interns the name and checks the
    /// macro registry. Prefer `is_macro` if you already have a symbol ID.
    #[inline]
    #[must_use]
    pub fn is_macro_by_name(&mut self, name: &str) -> bool {
        let sym_id = self.interner.intern(name);
        self.registry.contains(sym_id)
    }

    /// Returns the macro definition for a name, if it exists.
    ///
    /// This is a convenience method that interns the name and looks up the
    /// macro. Prefer `get_macro` if you already have a symbol ID.
    #[inline]
    #[must_use]
    pub fn get_macro_by_name(&mut self, name: &str) -> Option<&MacroDefinition> {
        let sym_id = self.interner.intern(name);
        self.registry.get(sym_id)
    }

    /// Returns a reference to the macro registry.
    #[inline]
    #[must_use]
    pub const fn registry(&self) -> &MacroRegistry {
        self.registry
    }

    /// Compiles a sequence of expressions into a chunk.
    ///
    /// Compiles each expression in order and emits a `Return` for the
    /// last expression's value (or nil if the sequence is empty).
    ///
    /// The compiled chunk is extracted from the compiler. After calling this
    /// method, the compiler can still be inspected (e.g., for macro definitions)
    /// but should not be used to compile more code.
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails (too many constants,
    /// registers, or unsupported features).
    #[inline]
    pub fn compile_program(&mut self, exprs: &[Spanned<Ast>]) -> Result<Chunk, Error> {
        let last_result = if exprs.is_empty() {
            // Empty program returns nil
            let span = Span::new(0_usize, 0_usize);
            let reg = self.alloc_register(span)?;
            self.chunk
                .emit(encode_abc(Opcode::LoadNil, reg, 0, 0), span);
            ExprResult { register: reg }
        } else {
            // Compile all expressions, keeping the last result
            let mut result = ExprResult { register: 0 };
            for expr in exprs {
                // Reset registers before each top-level expression
                self.next_register = 0;
                result = self.compile_expr(expr)?;
            }
            result
        };

        // Emit final return
        let span = exprs
            .last()
            .map_or(Span::new(0_usize, 0_usize), |last| last.span);
        self.chunk
            .emit(encode_abc(Opcode::Return, last_result.register, 1, 0), span);

        // Update chunk metadata
        self.chunk
            .set_max_registers(self.max_register.saturating_add(1));

        // Extract the chunk from the compiler
        Ok(core::mem::take(&mut self.chunk))
    }

    /// Compiles a single expression.
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails.
    #[inline]
    pub fn compile_expr(&mut self, expr: &Spanned<Ast>) -> Result<ExprResult, Error> {
        match expr.node {
            // Literals
            Ast::Integer(num) => self.compile_integer(num, expr.span),
            Ast::Float(num) => self.compile_float(num, expr.span),
            Ast::Bool(val) => self.compile_bool(val, expr.span),
            Ast::Nil => self.compile_nil(expr.span),

            // Symbols (global variable lookup)
            Ast::Symbol(ref name) => self.compile_symbol(name, expr.span),

            // Lists (function calls or special forms)
            Ast::List(ref elements) => self.compile_list(elements, expr.span),

            // String literals
            Ast::String(ref string) => self.compile_string(string, expr.span),
            Ast::Keyword(ref _kw) => Err(Error::NotImplemented {
                feature: "keyword literals",
                span: expr.span,
            }),
            Ast::Vector(ref _elements) => Err(Error::NotImplemented {
                feature: "vector literals",
                span: expr.span,
            }),
            Ast::Map(ref _elements) => Err(Error::NotImplemented {
                feature: "map literals",
                span: expr.span,
            }),

            // Handle future Ast variants (Ast is #[non_exhaustive])
            _ => Err(Error::NotImplemented {
                feature: "unknown AST node",
                span: expr.span,
            }),
        }
    }

    // =========================================================================
    // Literal Compilation
    // =========================================================================

    /// Compiles an integer literal.
    fn compile_integer(&mut self, value: i64, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let const_idx = self.chunk.add_constant_at(Constant::Integer(value), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a float literal.
    fn compile_float(&mut self, value: f64, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let const_idx = self.chunk.add_constant_at(Constant::Float(value), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a boolean literal.
    fn compile_bool(&mut self, value: bool, span: Span) -> Result<ExprResult, Error> {
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
    fn compile_nil(&mut self, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Compiles a string literal.
    fn compile_string(&mut self, value: &str, span: Span) -> Result<ExprResult, Error> {
        let dest = self.alloc_register(span)?;
        let const_idx = self
            .chunk
            .add_constant_at(Constant::String(alloc::string::String::from(value)), span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    // =========================================================================
    // Symbol Compilation
    // =========================================================================

    /// Compiles a symbol as a local or global variable lookup.
    ///
    /// First checks local scopes (for `let` bindings and function parameters),
    /// falling back to global lookup if not found locally.
    fn compile_symbol(&mut self, name: &str, span: Span) -> Result<ExprResult, Error> {
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
        let const_idx = self.chunk.add_constant_at(Constant::Symbol(sym_id), span)?;
        self.chunk
            .emit(encode_abx(Opcode::GetGlobal, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    // =========================================================================
    // List (Call) Compilation
    // =========================================================================

    /// Compiles a list as a function call, special form, or arithmetic operation.
    fn compile_list(&mut self, elements: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        if elements.is_empty() {
            return Err(Error::EmptyCall { span });
        }

        // Check if the first element is a symbol (could be special form or operator)
        if let Some(spanned_func) = elements.first()
            && let Ast::Symbol(ref name) = spanned_func.node
        {
            // Check for special forms first
            let args = elements.get(1_usize..).unwrap_or(&[]);
            match name.as_str() {
                "do" => return self.compile_do(args, span),
                "if" => return self.compile_if(args, span),
                "def" => return self.compile_def(args, span),
                "let" => return self.compile_let(args, span),
                "quote" => return self.compile_quote(args, span),
                "syntax-quote" => return self.compile_syntax_quote(args, span),
                "unquote" => {
                    return Err(Error::InvalidSpecialForm {
                        form: "unquote",
                        message: "unquote (~) not inside syntax-quote (`)",
                        span,
                    });
                }
                "unquote-splicing" => {
                    return Err(Error::InvalidSpecialForm {
                        form: "unquote-splicing",
                        message: "unquote-splicing (~@) not inside syntax-quote (`)",
                        span,
                    });
                }
                "fn" => return self.compile_fn(args, span),
                "defmacro" => return self.compile_defmacro(args, span),
                _ => {}
            }

            // Check for unary 'not'
            if name == "not" && elements.len() == 2_usize {
                return self.compile_not(elements, span);
            }

            // Check for binary operators (arithmetic and comparison)
            if let Some(opcode) = Self::binary_opcode(name) {
                return self.compile_binary_op(opcode, elements, span);
            }

            // Check for macro call - only if we have an expander to actually expand it
            // Without an expander, macro calls are treated as regular (undefined) function calls
            if self.expander.is_some() {
                let sym_id = self.interner.intern(name);
                if self.registry.contains(sym_id) {
                    return self.compile_macro_call(sym_id, args, span);
                }
            }
        }

        // General function call
        self.compile_call(elements, span)
    }

    /// Compiles a macro call by expanding and then compiling the result.
    ///
    /// This method is only called when an expander is available (checked by
    /// `compile_list` before calling this method). The macro transformer is
    /// executed at compile time to produce the expanded form.
    ///
    /// # Expansion Depth
    ///
    /// The compiler tracks macro expansion depth to prevent infinite recursion.
    /// If a macro expands to code that calls itself (directly or indirectly),
    /// the depth will eventually exceed `MAX_MACRO_EXPANSION_DEPTH` and
    /// compilation will fail with `Error::MacroExpansionDepthExceeded`.
    fn compile_macro_call(
        &mut self,
        macro_name: symbol::Id,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Check expansion depth before proceeding
        if self.macro_expansion_depth >= MAX_MACRO_EXPANSION_DEPTH {
            return Err(Error::MacroExpansionDepthExceeded {
                depth: self.macro_expansion_depth,
                span,
            });
        }

        // Get the macro definition
        let macro_def = self
            .registry
            .get(macro_name)
            .ok_or(Error::InvalidSpecialForm {
                form: "macro",
                message: "macro not found in registry",
                span,
            })?
            .clone();

        // Get the expander (we know it exists because compile_list checked)
        let Some(ref mut expander) = self.expander else {
            // This should not happen - compile_list only calls us with an expander
            return Err(Error::InvalidSpecialForm {
                form: "macro",
                message: "internal error: macro expansion without expander",
                span,
            });
        };

        // Convert AST arguments to Values
        let value_args: Vec<lona_core::value::Value> = args
            .iter()
            .map(|arg| conversion::ast_to_value(arg, self.interner))
            .collect();

        // Run the macro transformer
        let expanded_value = expander
            .expand(&macro_def, value_args, self.interner)
            .map_err(|err| Error::MacroExpansionFailed {
                message: err.message,
                span,
            })?;

        // Convert result back to AST
        let expanded_ast = conversion::value_to_ast(&expanded_value, self.interner, span)?;

        // Increment depth before recursive compilation
        self.macro_expansion_depth = self.macro_expansion_depth.saturating_add(1);

        // Recursively compile the expanded AST
        let result = self.compile_expr(&expanded_ast);

        // Decrement depth after compilation (even on error, for consistency)
        self.macro_expansion_depth = self.macro_expansion_depth.saturating_sub(1);

        result
    }

    /// Returns the opcode for a binary operator symbol, if any.
    ///
    /// Handles both arithmetic operators (`+`, `-`, `*`, `/`, `mod`) and
    /// comparison operators (`=`, `<`, `>`, `<=`, `>=`).
    fn binary_opcode(name: &str) -> Option<Opcode> {
        match name {
            // Arithmetic operators
            "+" => Some(Opcode::Add),
            "-" => Some(Opcode::Sub),
            "*" => Some(Opcode::Mul),
            "/" => Some(Opcode::Div),
            "mod" => Some(Opcode::Mod),
            // Comparison operators
            "=" => Some(Opcode::Eq),
            "<" => Some(Opcode::Lt),
            ">" => Some(Opcode::Gt),
            "<=" => Some(Opcode::Le),
            ">=" => Some(Opcode::Ge),
            _ => None,
        }
    }

    /// Compiles a binary operation (arithmetic or comparison).
    ///
    /// Also handles unary negation `(- x)` as a special case.
    fn compile_binary_op(
        &mut self,
        opcode: Opcode,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        match elements.len() {
            // Unary: (- x) → Neg
            2_usize if opcode == Opcode::Sub => {
                let arg = elements.get(1_usize).ok_or(Error::EmptyCall { span })?;
                let checkpoint = self.next_register;
                let result = self.compile_expr(arg)?;

                self.free_registers_to(checkpoint);
                let dest = self.alloc_register(span)?;
                self.chunk
                    .emit(encode_abc(Opcode::Neg, dest, result.register, 0), span);
                Ok(ExprResult { register: dest })
            }
            // Binary: (op x y)
            3_usize => {
                let arg1 = elements.get(1_usize).ok_or(Error::EmptyCall { span })?;
                let arg2 = elements.get(2_usize).ok_or(Error::EmptyCall { span })?;

                // Save register checkpoint
                let checkpoint = self.next_register;

                // Try to use RK encoding for constant operands
                let rk_b = self.try_compile_rk_operand(arg1)?;
                let rk_c = self.try_compile_rk_operand(arg2)?;

                // Allocate destination register (reuse checkpoint if possible)
                self.free_registers_to(checkpoint);
                let dest = self.alloc_register(span)?;

                self.chunk.emit(encode_abc(opcode, dest, rk_b, rk_c), span);
                Ok(ExprResult { register: dest })
            }
            _ => Err(Error::NotImplemented {
                feature: "n-ary arithmetic",
                span,
            }),
        }
    }

    /// Compiles unary `not` operation.
    fn compile_not(&mut self, elements: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        let arg = elements.get(1_usize).ok_or(Error::EmptyCall { span })?;

        let checkpoint = self.next_register;
        let result = self.compile_expr(arg)?;

        self.free_registers_to(checkpoint);
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abc(Opcode::Not, dest, result.register, 0), span);
        Ok(ExprResult { register: dest })
    }

    /// Tries to compile an operand as an RK value (constant if possible).
    ///
    /// Returns the RK-encoded value (either a register index or constant index).
    fn try_compile_rk_operand(&mut self, expr: &Spanned<Ast>) -> Result<u8, Error> {
        // Check if this can be a direct constant
        if let Some(rk) = self.try_constant_rk(expr)? {
            return Ok(rk);
        }

        // Otherwise compile to a register
        let result = self.compile_expr(expr)?;
        rk_register(result.register).ok_or(Error::TooManyRegisters { span: expr.span })
    }

    /// Tries to encode an expression as a constant in an RK field.
    ///
    /// Returns `Some(rk)` if the expression is a simple constant that fits in
    /// the RK constant range (index <= 127), `None` otherwise.
    fn try_constant_rk(&mut self, expr: &Spanned<Ast>) -> Result<Option<u8>, Error> {
        let constant = match expr.node {
            Ast::Integer(num) => Constant::Integer(num),
            Ast::Float(num) => Constant::Float(num),
            // Other AST types are not simple constants for RK encoding
            Ast::Nil
            | Ast::Bool(_)
            | Ast::String(_)
            | Ast::Symbol(_)
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Vector(_)
            | Ast::Map(_)
            // Handle future Ast variants (Ast is #[non_exhaustive])
            | _ => return Ok(None),
        };

        // Check if the next constant index would fit in RK range BEFORE adding.
        // This avoids adding the constant twice if it doesn't fit (once here,
        // once when falling back to register compilation).
        let next_idx = self.chunk.constants().len();
        if next_idx > usize::from(RK_MAX_CONSTANT) {
            return Ok(None);
        }

        // Add constant - index is guaranteed to fit in RK range
        let idx = self.chunk.add_constant_at(constant, expr.span)?;
        // The index must fit since we checked above
        let idx_u8 = u8::try_from(idx).ok();
        match idx_u8 {
            Some(i) if i <= RK_MAX_CONSTANT => Ok(rk_constant(i)),
            _ => Ok(None), // Should not happen, but handle gracefully
        }
    }

    /// Compiles a general function call.
    fn compile_call(&mut self, elements: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        let func_expr = elements.first().ok_or(Error::EmptyCall { span })?;
        let args = elements.get(1_usize..).unwrap_or(&[]);

        // Allocate contiguous registers: R_base = func, R_base+1..N = args
        let base = self.next_register;

        // Compile function into base register
        let func_result = self.compile_expr(func_expr)?;
        // Ensure function is at base (should be since we just allocated)
        if func_result.register != base {
            // Move to base if needed (shouldn't happen with current design)
            self.chunk.emit(
                encode_abc(Opcode::Move, base, func_result.register, 0),
                func_expr.span,
            );
        }

        // Compile arguments into consecutive registers
        for arg in args {
            let _arg_result = self.compile_expr(arg)?;
            // Arguments are automatically placed in consecutive registers
        }

        // Emit call instruction
        let arg_count =
            u8::try_from(args.len()).map_err(|_err| Error::TooManyRegisters { span })?;

        self.chunk
            .emit(encode_abc(Opcode::Call, base, arg_count, 1), span);

        // Result is left in base register
        // Free argument registers
        self.free_registers_to(base.saturating_add(1));

        Ok(ExprResult { register: base })
    }

    // =========================================================================
    // Special Forms
    // =========================================================================

    /// Compiles a `do` special form.
    ///
    /// Syntax: `(do)` or `(do expr1 expr2 ... exprN)`
    ///
    /// Evaluates expressions left to right and returns the value of the last
    /// expression. Empty `(do)` returns nil.
    fn compile_do(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        if args.is_empty() {
            // Empty do returns nil
            let dest = self.alloc_register(span)?;
            self.chunk
                .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
            return Ok(ExprResult { register: dest });
        }

        // Compile all but last expression, discarding results
        for expr in args
            .get(..args.len().saturating_sub(1_usize))
            .unwrap_or(&[])
        {
            let checkpoint = self.next_register;
            let _result = self.compile_expr(expr)?;
            self.free_registers_to(checkpoint);
        }

        // Compile last expression and return its result
        let last_expr = args.last().ok_or(Error::EmptyCall { span })?;
        self.compile_expr(last_expr)
    }

    /// Compiles an `if` special form.
    ///
    /// Syntax: `(if test then)` or `(if test then else)`
    ///
    /// Evaluates `test`. If truthy (not nil or false), evaluates and returns
    /// `then`. Otherwise evaluates and returns `else` (or nil if no else).
    fn compile_if(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        // Validate: need 2 or 3 args (test, then, [else])
        if args.len() < 2_usize || args.len() > 3_usize {
            return Err(Error::InvalidSpecialForm {
                form: "if",
                message: "expected (if test then) or (if test then else)",
                span,
            });
        }

        let test_expr = args.first().ok_or(Error::EmptyCall { span })?;
        let then_expr = args.get(1_usize).ok_or(Error::EmptyCall { span })?;
        let else_expr = args.get(2_usize);

        // Compile test expression
        let checkpoint = self.next_register;
        let test_result = self.compile_expr(test_expr)?;

        // Emit JumpIfNot (will patch offset later)
        let jump_to_else_idx = self.chunk.emit(
            encode_asbx(Opcode::JumpIfNot, test_result.register, 0),
            span,
        );

        // Free test register
        self.free_registers_to(checkpoint);

        // Allocate destination register for result
        let dest = self.alloc_register(span)?;

        // Compile then branch into dest
        let then_result = self.compile_expr(then_expr)?;
        if then_result.register != dest {
            self.chunk.emit(
                encode_abc(Opcode::Move, dest, then_result.register, 0),
                then_expr.span,
            );
        }
        // Free any temps from then branch but keep dest
        self.free_registers_to(dest.saturating_add(1));

        // Emit Jump over else branch (will patch offset later)
        let jump_to_end_idx = self.chunk.emit(encode_asbx(Opcode::Jump, 0, 0), span);

        // Patch jump_to_else to point here (current instruction index)
        let else_offset = self
            .chunk
            .len()
            .saturating_sub(jump_to_else_idx)
            .saturating_sub(1);
        let else_offset_i16 =
            i16::try_from(else_offset).map_err(|_err| Error::JumpTooLarge { span })?;
        self.chunk.patch(
            jump_to_else_idx,
            encode_asbx(Opcode::JumpIfNot, test_result.register, else_offset_i16),
        );

        // Compile else branch (or nil) into dest
        if let Some(else_branch) = else_expr {
            let else_result = self.compile_expr(else_branch)?;
            if else_result.register != dest {
                self.chunk.emit(
                    encode_abc(Opcode::Move, dest, else_result.register, 0),
                    else_branch.span,
                );
            }
        } else {
            self.chunk
                .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
        }
        // Free any temps from else branch but keep dest
        self.free_registers_to(dest.saturating_add(1));

        // Patch jump_to_end to point here
        let end_offset = self
            .chunk
            .len()
            .saturating_sub(jump_to_end_idx)
            .saturating_sub(1);
        let end_offset_i16 =
            i16::try_from(end_offset).map_err(|_err| Error::JumpTooLarge { span })?;
        self.chunk.patch(
            jump_to_end_idx,
            encode_asbx(Opcode::Jump, 0, end_offset_i16),
        );

        Ok(ExprResult { register: dest })
    }

    /// Compiles a `def` special form.
    ///
    /// Syntax: `(def name value)`
    ///
    /// Evaluates `value` and binds it to the global variable `name`.
    /// Returns the symbol `name`.
    fn compile_def(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        // Validate: need exactly 2 args (name, value)
        if args.len() != 2_usize {
            return Err(Error::InvalidSpecialForm {
                form: "def",
                message: "expected (def name value)",
                span,
            });
        }

        // First arg must be a symbol
        let name_expr = args.first().ok_or(Error::EmptyCall { span })?;
        let Ast::Symbol(ref name) = name_expr.node else {
            return Err(Error::InvalidSpecialForm {
                form: "def",
                message: "first argument must be a symbol",
                span: name_expr.span,
            });
        };

        let value_expr = args.get(1_usize).ok_or(Error::EmptyCall { span })?;

        // Compile value expression
        let checkpoint = self.next_register;
        let value_result = self.compile_expr(value_expr)?;

        // Intern the symbol and add to constants
        let symbol_id = self.interner.intern(name);
        let symbol_const = self
            .chunk
            .add_constant_at(Constant::Symbol(symbol_id), span)?;

        // Emit SetGlobal
        self.chunk.emit(
            encode_abx(Opcode::SetGlobal, value_result.register, symbol_const),
            span,
        );

        // Free value register
        self.free_registers_to(checkpoint);

        // Return the symbol (load it into destination)
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, symbol_const), span);

        Ok(ExprResult { register: dest })
    }

    /// Compiles a `let` special form.
    ///
    /// Syntax: `(let [name1 val1 name2 val2 ...] body...)`
    ///
    /// Bindings are evaluated left to right, and each binding can reference
    /// previous bindings. Body expressions are evaluated with bindings in scope,
    /// and the value of the last body expression is returned.
    fn compile_let(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        // Need at least bindings vector
        if args.is_empty() {
            return Err(Error::InvalidSpecialForm {
                form: "let",
                message: "expected (let [bindings...] body...)",
                span,
            });
        }

        // First arg must be a vector of bindings
        let bindings_ast = args.first().ok_or(Error::EmptyCall { span })?;
        let Ast::Vector(ref bindings) = bindings_ast.node else {
            return Err(Error::InvalidSpecialForm {
                form: "let",
                message: "first argument must be a vector of bindings",
                span: bindings_ast.span,
            });
        };

        // Bindings must come in pairs
        if bindings.len() % 2_usize != 0_usize {
            return Err(Error::InvalidSpecialForm {
                form: "let",
                message: "bindings must be pairs of [name value ...]",
                span: bindings_ast.span,
            });
        }

        let body = args.get(1_usize..).unwrap_or(&[]);

        // Save register state for cleanup
        let checkpoint = self.next_register;

        // Push new scope
        self.locals.push_scope();

        // Process bindings in pairs
        let mut binding_idx: usize = 0;
        while binding_idx < bindings.len() {
            let name_ast = bindings.get(binding_idx).ok_or(Error::EmptyCall { span })?;
            let value_ast = bindings
                .get(binding_idx.saturating_add(1))
                .ok_or(Error::EmptyCall { span })?;

            // Name must be a symbol
            let Ast::Symbol(ref name) = name_ast.node else {
                return Err(Error::InvalidSpecialForm {
                    form: "let",
                    message: "binding name must be a symbol",
                    span: name_ast.span,
                });
            };

            // Allocate register for this binding
            let reg = self.alloc_register(value_ast.span)?;

            // Compile value into the register
            let value_result = self.compile_expr(value_ast)?;
            if value_result.register != reg {
                self.chunk.emit(
                    encode_abc(Opcode::Move, reg, value_result.register, 0),
                    value_ast.span,
                );
            }

            // Free any temps but keep the binding register
            self.free_registers_to(reg.saturating_add(1));

            // Register the binding
            let symbol_id = self.interner.intern(name);
            self.locals.define(symbol_id, reg);

            binding_idx = binding_idx.saturating_add(2);
        }

        // Compile body (like do)
        let result = if body.is_empty() {
            // Empty body returns nil
            let dest = self.alloc_register(span)?;
            self.chunk
                .emit(encode_abc(Opcode::LoadNil, dest, 0, 0), span);
            ExprResult { register: dest }
        } else {
            // Compile all but last expression, discarding results
            for expr in body.get(..body.len().saturating_sub(1)).unwrap_or(&[]) {
                let temp_checkpoint = self.next_register;
                let _temp_result = self.compile_expr(expr)?;
                self.free_registers_to(temp_checkpoint);
            }
            // Compile last expression to return
            let last = body.last().ok_or(Error::EmptyCall { span })?;
            self.compile_expr(last)?
        };

        // Pop scope and restore registers
        self.locals.pop_scope();
        self.free_registers_to(checkpoint);

        // Move result to checkpoint register if needed
        let dest = self.alloc_register(span)?;
        if result.register != dest {
            self.chunk
                .emit(encode_abc(Opcode::Move, dest, result.register, 0), span);
        }

        Ok(ExprResult { register: dest })
    }

    /// Compiles a `quote` special form.
    ///
    /// Syntax: `(quote datum)`
    ///
    /// Returns the datum as a value without evaluating it.
    fn compile_quote(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        if args.len() != 1_usize {
            return Err(Error::InvalidSpecialForm {
                form: "quote",
                message: "expected (quote datum)",
                span,
            });
        }

        let datum = args.first().ok_or(Error::EmptyCall { span })?;
        let constant = self.ast_to_constant(datum)?;
        let const_idx = self.chunk.add_constant_at(constant, datum.span)?;
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);
        Ok(ExprResult { register: dest })
    }

    /// Converts an AST node to a compile-time constant.
    ///
    /// Used by `quote` to convert the quoted datum to a constant value.
    fn ast_to_constant(&mut self, ast: &Spanned<Ast>) -> Result<Constant, Error> {
        match ast.node {
            Ast::Nil => Ok(Constant::Nil),
            Ast::Bool(bool_val) => Ok(Constant::Bool(bool_val)),
            Ast::Integer(num) => Ok(Constant::Integer(num)),
            Ast::Float(num) => Ok(Constant::Float(num)),
            Ast::String(ref text) => {
                Ok(Constant::String(alloc::string::String::from(text.as_str())))
            }
            Ast::Symbol(ref name) => {
                let id = self.interner.intern(name);
                Ok(Constant::Symbol(id))
            }
            Ast::Keyword(ref name) => {
                // Keywords are stored as symbols with a : prefix
                let keyword_name = alloc::format!(":{name}");
                let id = self.interner.intern(&keyword_name);
                Ok(Constant::Symbol(id))
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
            Ast::Map(_) => Err(Error::NotImplemented {
                feature: "quoted maps",
                span: ast.span,
            }),
            // Ast is non-exhaustive, handle future variants
            _ => Err(Error::NotImplemented {
                feature: "unknown AST node in quote",
                span: ast.span,
            }),
        }
    }

    /// Compiles a `syntax-quote` special form (quasiquote).
    ///
    /// Syntax: `` `datum `` or `(syntax-quote datum)`
    ///
    /// Expands the datum at compile time, allowing `~` (unquote) and `~@`
    /// (unquote-splicing) to interpolate evaluated expressions into the
    /// quoted structure.
    fn compile_syntax_quote(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        if args.len() != 1_usize {
            return Err(Error::InvalidSpecialForm {
                form: "syntax-quote",
                message: "expected exactly 1 argument",
                span,
            });
        }

        let datum = args.first().ok_or(Error::EmptyCall { span })?;

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
    fn expand_quasiquote(&mut self, ast: &Spanned<Ast>, depth: u32) -> Result<Spanned<Ast>, Error> {
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
                    Err(Error::InvalidSpecialForm {
                        form: "unquote-splicing",
                        message: "~@ not in list or vector context",
                        span: ast.span,
                    })
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
            Ast::Map(ref _elements) => Err(Error::NotImplemented {
                feature: "quasiquoted maps",
                span: ast.span,
            }),

            // Atoms: wrap in (quote ...)
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Symbol(_)
            | Ast::Keyword(_) => Ok(Self::quote_atom(ast)),

            // Handle future AST variants
            _ => Err(Error::NotImplemented {
                feature: "unknown AST node in quasiquote",
                span: ast.span,
            }),
        }
    }

    /// Checks if a list is an `(unquote x)` form.
    fn is_unquote(elements: &[Spanned<Ast>]) -> bool {
        matches!(
            elements.first().map(|elem| &elem.node),
            Some(Ast::Symbol(name)) if name == "unquote"
        )
    }

    /// Checks if a list is an `(unquote-splicing x)` form.
    fn is_unquote_splicing(elements: &[Spanned<Ast>]) -> bool {
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
            return Err(Error::InvalidSpecialForm {
                form: "unquote",
                message: "expected exactly 1 argument",
                span,
            });
        }

        let inner = elements.get(1_usize).ok_or(Error::EmptyCall { span })?;

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
            return Err(Error::InvalidSpecialForm {
                form: "unquote-splicing",
                message: "expected exactly 1 argument",
                span,
            });
        }

        let inner = elements.get(1_usize).ok_or(Error::EmptyCall { span })?;
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
            return Err(Error::InvalidSpecialForm {
                form: "syntax-quote",
                message: "expected exactly 1 argument",
                span,
            });
        }

        let inner = elements.get(1_usize).ok_or(Error::EmptyCall { span })?;
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
                let inner = Self::get_unquote_splicing_arg(elem, span)?;
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
    fn get_unquote_splicing_arg(ast: &Spanned<Ast>, span: Span) -> Result<&Spanned<Ast>, Error> {
        if let Ast::List(ref elements) = ast.node {
            if elements.len() != 2_usize {
                return Err(Error::InvalidSpecialForm {
                    form: "unquote-splicing",
                    message: "expected exactly 1 argument",
                    span,
                });
            }
            elements.get(1_usize).ok_or(Error::EmptyCall { span })
        } else {
            Err(Error::InvalidSpecialForm {
                form: "unquote-splicing",
                message: "internal error: not a list",
                span,
            })
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
                let inner = Self::get_unquote_splicing_arg(elem, span)?;
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
    fn quote_atom(ast: &Spanned<Ast>) -> Spanned<Ast> {
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
    fn make_symbol(name: &str, span: Span) -> Spanned<Ast> {
        Spanned::new(Ast::Symbol(alloc::string::String::from(name)), span)
    }

    /// Creates a list AST node.
    const fn make_list(elements: Vec<Spanned<Ast>>, span: Span) -> Spanned<Ast> {
        Spanned::new(Ast::List(elements), span)
    }

    /// Creates an empty list AST node.
    const fn make_empty_list(span: Span) -> Spanned<Ast> {
        Self::make_list(Vec::new(), span)
    }

    /// Creates a quoted symbol: `'name` -> `(quote name)`.
    fn make_quoted_symbol(name: &str, span: Span) -> Spanned<Ast> {
        Self::make_list(
            alloc::vec![
                Self::make_symbol("quote", span),
                Self::make_symbol(name, span),
            ],
            span,
        )
    }

    /// Compiles a `fn` special form.
    ///
    /// Syntax:
    /// - `(fn [params...] body...)`
    /// - `(fn name [params...] body...)` - named for recursion/debugging
    ///
    /// Creates a new function value. Parameters become local variables in the
    /// function's scope. The function body is compiled into a separate chunk.
    ///
    /// Note: In Phase 3.3, closures are not supported - functions cannot
    /// reference variables from enclosing scopes.
    fn compile_fn(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        // Parse: (fn [params] body...) or (fn name [params] body...)
        let (name, params_ast, body) = Self::parse_fn_args(args, span)?;

        // Extract parameter names
        let params = Self::extract_params(params_ast)?;
        let arity = u8::try_from(params.len()).map_err(|_err| Error::TooManyRegisters { span })?;

        // Create a new compiler for the function body
        // Note: We share the registry so macros are available inside function bodies
        let mut fn_compiler = Compiler::new(self.interner, self.registry);
        let fn_name_str = name
            .clone()
            .unwrap_or_else(|| alloc::string::String::from("lambda"));
        fn_compiler.chunk = Chunk::with_name(fn_name_str);
        fn_compiler.chunk.set_arity(arity);

        // Parameters become locals at R[0], R[1], etc.
        fn_compiler.locals.push_scope();
        for (i, param) in params.iter().enumerate() {
            let symbol_id = fn_compiler.interner.intern(param);
            let reg = u8::try_from(i).map_err(|_err| Error::TooManyRegisters { span })?;
            fn_compiler.locals.define(symbol_id, reg);
            fn_compiler.next_register = reg.saturating_add(1);
            if reg > fn_compiler.max_register {
                fn_compiler.max_register = reg;
            }
        }

        // Compile body
        let result_reg = if body.is_empty() {
            // Empty body returns nil
            let reg = fn_compiler.alloc_register(span)?;
            fn_compiler
                .chunk
                .emit(encode_abc(Opcode::LoadNil, reg, 0, 0), span);
            reg
        } else {
            // Compile all but last expression, discarding results
            for expr in body.get(..body.len().saturating_sub(1)).unwrap_or(&[]) {
                let checkpoint = fn_compiler.next_register;
                let _result = fn_compiler.compile_expr(expr)?;
                fn_compiler.free_registers_to(checkpoint);
            }
            // Compile last expression to return
            let last = body.last().ok_or(Error::EmptyCall { span })?;
            let result = fn_compiler.compile_expr(last)?;
            result.register
        };

        // Emit return
        fn_compiler
            .chunk
            .emit(encode_abc(Opcode::Return, result_reg, 1, 0), span);
        fn_compiler.locals.pop_scope();

        // Finalize function chunk
        fn_compiler
            .chunk
            .set_max_registers(fn_compiler.max_register.saturating_add(1));
        let fn_chunk = fn_compiler.chunk;

        // Add function as constant in parent chunk
        let const_idx = self.chunk.add_constant_at(
            Constant::Function {
                chunk: alloc::boxed::Box::new(fn_chunk),
                arity,
                name,
            },
            span,
        )?;

        // Load function into destination register
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);

        Ok(ExprResult { register: dest })
    }

    /// Parses the arguments to a `fn` special form.
    ///
    /// Returns (name, `params_ast`, body) where name is optional.
    fn parse_fn_args(args: &[Spanned<Ast>], span: Span) -> Result<FnArgsResult<'_>, Error> {
        let first = args.first().ok_or(Error::InvalidSpecialForm {
            form: "fn",
            message: "expected (fn [params] body...) or (fn name [params] body...)",
            span,
        })?;

        // Check if first arg is a name (symbol) or params (vector)
        match first.node {
            Ast::Vector(_) => {
                // (fn [params] body...)
                let body = args.get(1_usize..).unwrap_or(&[]);
                Ok((None, first, body))
            }
            Ast::Symbol(ref name) => {
                // (fn name [params] body...)
                let params_ast = args.get(1_usize).ok_or(Error::InvalidSpecialForm {
                    form: "fn",
                    message: "expected parameter vector after function name",
                    span,
                })?;
                let body = args.get(2_usize..).unwrap_or(&[]);
                Ok((Some(name.clone()), params_ast, body))
            }
            // All other AST variants are invalid as the first argument to fn
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Map(_)
            | _ => Err(Error::InvalidSpecialForm {
                form: "fn",
                message: "expected [params] or name",
                span: first.span,
            }),
        }
    }

    /// Extracts parameter names from a parameter vector AST.
    fn extract_params(params_ast: &Spanned<Ast>) -> Result<Vec<alloc::string::String>, Error> {
        let Ast::Vector(ref params_vec) = params_ast.node else {
            return Err(Error::InvalidSpecialForm {
                form: "fn",
                message: "parameters must be a vector",
                span: params_ast.span,
            });
        };

        let mut params = Vec::new();
        for param in params_vec {
            let Ast::Symbol(ref name) = param.node else {
                return Err(Error::InvalidSpecialForm {
                    form: "fn",
                    message: "parameter must be a symbol",
                    span: param.span,
                });
            };
            params.push(name.clone());
        }
        Ok(params)
    }

    /// Compiles a `defmacro` special form.
    ///
    /// Syntax: `(defmacro name [params...] body...)`
    ///
    /// Defines a compile-time macro. The macro body is compiled to bytecode
    /// and stored in the compiler's macro registry. When the macro is called,
    /// it receives unevaluated arguments and returns transformed AST.
    ///
    /// Returns the macro's symbol name.
    fn compile_defmacro(&mut self, args: &[Spanned<Ast>], span: Span) -> Result<ExprResult, Error> {
        // Need at least name and params
        if args.len() < 2_usize {
            return Err(Error::InvalidSpecialForm {
                form: "defmacro",
                message: "expected (defmacro name [params...] body...)",
                span,
            });
        }

        // Extract name (must be a symbol)
        let name_ast = args.first().ok_or(Error::EmptyCall { span })?;
        let Ast::Symbol(ref name_ref) = name_ast.node else {
            return Err(Error::InvalidSpecialForm {
                form: "defmacro",
                message: "macro name must be a symbol",
                span: name_ast.span,
            });
        };
        let name = name_ref.clone();

        // Extract params (must be a vector of symbols)
        let params_ast = args.get(1_usize).ok_or(Error::EmptyCall { span })?;
        let params =
            Self::extract_params(params_ast).map_err(|_err| Error::InvalidSpecialForm {
                form: "defmacro",
                message: "parameters must be a vector of symbols",
                span: params_ast.span,
            })?;
        let arity = u8::try_from(params.len()).map_err(|_err| Error::TooManyRegisters { span })?;

        // Body is everything after params
        let body = args.get(2_usize..).unwrap_or(&[]);
        if body.is_empty() {
            return Err(Error::InvalidSpecialForm {
                form: "defmacro",
                message: "macro body cannot be empty",
                span,
            });
        }

        // Intern the macro name before creating the child compiler to avoid
        // double mutable borrow of self.interner
        let name_id = self.interner.intern(&name);

        // Create a child compiler for the macro body.
        // Note: We share the registry so macros can be used inside macro bodies.
        let mut macro_compiler = Compiler::new(self.interner, self.registry);
        macro_compiler.chunk = Chunk::with_name(alloc::format!("macro:{name}"));
        macro_compiler.chunk.set_arity(arity);

        // Parameters become locals at R[0], R[1], etc.
        macro_compiler.locals.push_scope();
        for (i, param) in params.iter().enumerate() {
            let symbol_id = macro_compiler.interner.intern(param);
            let reg = u8::try_from(i).map_err(|_err| Error::TooManyRegisters { span })?;
            macro_compiler.locals.define(symbol_id, reg);
            macro_compiler.next_register = reg.saturating_add(1);
            if reg > macro_compiler.max_register {
                macro_compiler.max_register = reg;
            }
        }

        // Compile body (all expressions, last one is return value)
        let result_reg = {
            // Compile all but last, discarding results
            for expr in body
                .get(..body.len().saturating_sub(1_usize))
                .unwrap_or(&[])
            {
                let checkpoint = macro_compiler.next_register;
                let _result = macro_compiler.compile_expr(expr)?;
                macro_compiler.free_registers_to(checkpoint);
            }
            // Compile last expression as return value
            let last = body.last().ok_or(Error::EmptyCall { span })?;
            let result = macro_compiler.compile_expr(last)?;
            result.register
        };

        // Emit return instruction
        macro_compiler
            .chunk
            .emit(encode_abc(Opcode::Return, result_reg, 1, 0), span);
        macro_compiler.locals.pop_scope();

        // Finalize the macro chunk
        macro_compiler
            .chunk
            .set_max_registers(macro_compiler.max_register.saturating_add(1));

        // Extract the macro chunk before using self again
        let macro_chunk = macro_compiler.chunk;

        // Store in macro registry
        self.registry.register(
            name_id,
            MacroDefinition::new(Arc::new(macro_chunk), arity, name),
        );

        // Return the macro's symbol name
        // This mimics `def` behavior - the expression evaluates to the defined name
        let const_idx = self
            .chunk
            .add_constant_at(Constant::Symbol(name_id), span)?;
        let dest = self.alloc_register(span)?;
        self.chunk
            .emit(encode_abx(Opcode::LoadK, dest, const_idx), span);

        Ok(ExprResult { register: dest })
    }

    // =========================================================================
    // Register Management
    // =========================================================================

    /// Allocates the next available register.
    ///
    /// # Errors
    ///
    /// Returns `Error::TooManyRegisters` if all 256 registers are in use.
    const fn alloc_register(&mut self, span: Span) -> Result<u8, Error> {
        let reg = self.next_register;
        if reg == u8::MAX {
            return Err(Error::TooManyRegisters { span });
        }
        self.next_register = reg.saturating_add(1);
        if reg > self.max_register {
            self.max_register = reg;
        }
        Ok(reg)
    }

    /// Releases registers back to a checkpoint.
    ///
    /// Used to reclaim temporary registers after a subexpression.
    const fn free_registers_to(&mut self, checkpoint: u8) {
        self.next_register = checkpoint;
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Error type for high-level compilation that includes parse errors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompileError {
    /// Error during parsing.
    Parse(lonala_parser::Error),
    /// Error during compilation.
    Compile(Error),
}

impl core::fmt::Display for CompileError {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Parse(ref err) => write!(f, "parse error: {err}"),
            Self::Compile(ref err) => write!(f, "compile error: {err}"),
        }
    }
}

impl From<lonala_parser::Error> for CompileError {
    #[inline]
    fn from(err: lonala_parser::Error) -> Self {
        Self::Parse(err)
    }
}

impl From<Error> for CompileError {
    #[inline]
    fn from(err: Error) -> Self {
        Self::Compile(err)
    }
}

/// Parses and compiles Lonala source code to bytecode.
///
/// Convenience function that combines parsing and compilation.
///
/// # Errors
///
/// Returns `CompileError::Parse` if parsing fails, or
/// `CompileError::Compile` if compilation fails.
///
/// # Example
///
/// ```
/// use lona_core::symbol::Interner;
/// use lonala_compiler::compile;
///
/// let mut interner = Interner::new();
/// let chunk = compile("(+ 1 2)", &mut interner).unwrap();
/// ```
#[inline]
pub fn compile(source: &str, interner: &mut symbol::Interner) -> Result<Chunk, CompileError> {
    let mut registry = MacroRegistry::new();
    compile_with_registry(source, interner, &mut registry)
}

/// Compiles Lonala source code to bytecode with a persistent macro registry.
///
/// This is the full-featured compilation function that accepts an external
/// macro registry. Use this for REPL sessions where macros should persist
/// across evaluations.
///
/// # Errors
///
/// Returns `CompileError::Parse` if parsing fails, or
/// `CompileError::Compile` if compilation fails.
///
/// # Example
///
/// ```
/// use lona_core::symbol::Interner;
/// use lonala_compiler::{compile_with_registry, MacroRegistry};
///
/// let mut interner = Interner::new();
/// let mut registry = MacroRegistry::new();
///
/// // Define a macro
/// let chunk1 = compile_with_registry(
///     "(defmacro double [x] (list '+ x x))",
///     &mut interner,
///     &mut registry
/// ).unwrap();
///
/// // Use the macro (it persists in the registry)
/// let chunk2 = compile_with_registry(
///     "(double 5)",
///     &mut interner,
///     &mut registry
/// ).unwrap();
/// ```
#[inline]
pub fn compile_with_registry(
    source: &str,
    interner: &mut symbol::Interner,
    registry: &mut MacroRegistry,
) -> Result<Chunk, CompileError> {
    let exprs = lonala_parser::parse(source)?;
    let mut compiler = Compiler::new(interner, registry);
    let chunk = compiler.compile_program(&exprs)?;
    Ok(chunk)
}

/// Compiles Lonala source code with macro expansion capability.
///
/// This is the most complete compilation function that supports:
/// - Persistent macro registry for cross-session macro definitions
/// - Compile-time macro expansion using the provided expander
///
/// Use this for REPL sessions where you want macros to be expanded at
/// compile time rather than called at runtime.
///
/// # Errors
///
/// Returns `CompileError::Parse` if parsing fails, or
/// `CompileError::Compile` if compilation or macro expansion fails.
///
/// # Example
///
/// ```ignore
/// use lona_core::symbol::Interner;
/// use lonala_compiler::{compile_with_expansion, MacroRegistry, MacroExpander};
///
/// let mut interner = Interner::new();
/// let mut registry = MacroRegistry::new();
/// let mut expander = MyMacroExpander::new(); // Implements MacroExpander
///
/// // Define a macro
/// let chunk1 = compile_with_expansion(
///     "(defmacro double [x] `(+ ~x ~x))",
///     &mut interner,
///     &mut registry,
///     &mut expander
/// ).unwrap();
///
/// // Use the macro - it will be expanded at compile time
/// let chunk2 = compile_with_expansion(
///     "(double 5)",
///     &mut interner,
///     &mut registry,
///     &mut expander
/// ).unwrap();
/// ```
#[inline]
pub fn compile_with_expansion(
    source: &str,
    interner: &mut symbol::Interner,
    registry: &mut MacroRegistry,
    expander: &mut dyn MacroExpander,
) -> Result<Chunk, CompileError> {
    let exprs = lonala_parser::parse(source)?;
    let mut compiler = Compiler::with_expander(interner, registry, expander);
    let chunk = compiler.compile_program(&exprs)?;
    Ok(chunk)
}
