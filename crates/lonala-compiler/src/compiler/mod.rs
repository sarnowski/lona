// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Compiler from Lonala AST to bytecode.
//!
//! Transforms parsed [`Ast`] expressions into executable [`Chunk`] bytecode.
//! This module implements the core compilation logic for the Lonala language.

use alloc::vec::Vec;

use lona_core::chunk::Chunk;
use lona_core::opcode::{Opcode, encode_abc};
use lona_core::source;
use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use crate::error::{Error, Kind as ErrorKind, SourceLocation};

// Submodules containing the actual compilation logic
mod api;
mod calls;
mod case;
mod closures;
pub mod conversion;
pub mod destructure;
mod expressions;
mod functions;
mod let_form;
mod locals;
pub mod macros;
mod nary;
mod operators;
mod quasiquote;
mod quote;
mod special_forms;

// Re-export closure types for internal use
use closures::{CaptureContext, SymbolResolution, UpvalueInfo};
use locals::LocalEnv;

// Re-export public API
pub use api::{CompileError, compile, compile_with_expansion, compile_with_registry};
pub use macros::{MacroBody, MacroDefinition, MacroExpander, MacroExpansionError, MacroRegistry};

#[cfg(test)]
mod tests;

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
///
/// # Closure Support
///
/// The compiler tracks upvalues for closures. When compiling a nested function,
/// a `CaptureContext` describes what variables are available from the parent.
/// Captured variables are recorded in `upvalues` and emitted as `UpvalueSource`
/// descriptors in the compiled function.
pub struct Compiler<'interner, 'registry, 'expander> {
    /// The bytecode chunk being built.
    chunk: Chunk,
    /// Symbol interner for looking up and creating symbol IDs.
    interner: &'interner mut symbol::Interner,
    /// Source identifier for error reporting.
    source_id: source::Id,
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
    /// Context for capturing variables from enclosing scopes.
    /// Empty for top-level code; populated when compiling nested functions.
    capture_context: CaptureContext,
    /// Upvalues captured by the current function.
    /// Each entry describes a variable captured from an enclosing scope.
    upvalues: Vec<UpvalueInfo>,
    /// Whether the current expression is in tail position.
    /// Used for tail call optimization - calls in tail position emit `TailCall`
    /// instead of `Call` so the VM can replace the current frame rather than
    /// pushing a new one.
    in_tail_position: bool,
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
        source_id: source::Id,
    ) -> Self {
        Self {
            chunk: Chunk::new(),
            interner,
            source_id,
            next_register: 0,
            max_register: 0,
            locals: LocalEnv::new(),
            registry,
            expander: None,
            macro_expansion_depth: 0,
            capture_context: CaptureContext::new(),
            upvalues: Vec::new(),
            in_tail_position: false,
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
        source_id: source::Id,
        expander: &'expander mut dyn MacroExpander,
    ) -> Self {
        Self {
            chunk: Chunk::new(),
            interner,
            source_id,
            next_register: 0,
            max_register: 0,
            locals: LocalEnv::new(),
            registry,
            expander: Some(expander),
            macro_expansion_depth: 0,
            capture_context: CaptureContext::new(),
            upvalues: Vec::new(),
            in_tail_position: false,
        }
    }

    /// Creates a source location from a span.
    #[inline]
    #[must_use]
    const fn location(&self, span: Span) -> SourceLocation {
        SourceLocation::new(self.source_id, span)
    }

    /// Adds a constant to the chunk, converting any error to have the correct source location.
    fn add_constant(
        &mut self,
        constant: lona_core::chunk::Constant,
        span: Span,
    ) -> Result<u16, Error> {
        self.chunk
            .add_constant_at(constant, span)
            .map_err(|_err| Error::new(ErrorKind::TooManyConstants, self.location(span)))
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
    /// # Tail Position
    ///
    /// Top-level expressions are compiled with `in_tail_position = false`.
    /// This is intentional: tail call optimization only applies within function
    /// bodies where recursive calls can replace the current frame. At the top
    /// level (e.g., REPL), there's no function frame to replace, so TCO doesn't
    /// apply. The final `Return` instruction returns from the program, not from
    /// a function that could be tail-called.
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
            Ast::Keyword(ref name) => self.compile_keyword(name, expr.span),
            Ast::Vector(ref elements) => self.compile_vector(elements, expr.span),
            Ast::Map(ref elements) => self.compile_map(elements, expr.span),
            Ast::Set(ref elements) => self.compile_set(elements, expr.span),

            // Metadata (stub for Task 1.1.7: compile inner value without metadata)
            Ast::WithMeta { ref value, .. } => {
                // TODO(Task 1.1.7): Implement metadata attachment at runtime.
                // For now, compile the inner value without metadata.
                self.compile_expr(value)
            }

            // Handle future Ast variants (Ast is #[non_exhaustive])
            _ => Err(Error::new(
                ErrorKind::NotImplemented {
                    feature: "unknown AST node",
                },
                self.location(expr.span),
            )),
        }
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
            return Err(Error::new(ErrorKind::TooManyRegisters, self.location(span)));
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

    // =========================================================================
    // Tail Position Tracking
    // =========================================================================

    /// Executes a closure with a specific tail position setting, then restores
    /// the previous value.
    ///
    /// This is used to propagate tail position through the AST during compilation.
    /// Expressions in tail position can have their calls optimized to tail calls.
    ///
    /// # Example
    ///
    /// ```text
    /// // Compile the test expression NOT in tail position
    /// self.with_tail_position(false, |compiler| {
    ///     compiler.compile_expr(test_expr)
    /// })?;
    ///
    /// // Compile the then branch in tail position (if we are)
    /// self.with_tail_position(self.in_tail_position, |compiler| {
    ///     compiler.compile_expr(then_expr)
    /// })?;
    /// ```
    #[inline]
    fn with_tail_position<F, R>(&mut self, tail: bool, closure: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let prev = self.in_tail_position;
        self.in_tail_position = tail;
        let result = closure(self);
        self.in_tail_position = prev;
        result
    }
}
