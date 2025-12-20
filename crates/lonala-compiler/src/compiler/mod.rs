// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Compiler from Lonala AST to bytecode.
//!
//! Transforms parsed [`Ast`] expressions into executable [`Chunk`] bytecode.
//! This module implements the core compilation logic for the Lonala language.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use lona_core::chunk::Chunk;
use lona_core::opcode::{Opcode, encode_abc};
use lona_core::source;
use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use crate::error::{Error, Kind as ErrorKind, SourceLocation};

// Submodules containing the actual compilation logic
mod calls;
pub mod conversion;
mod expressions;
mod functions;
pub mod macros;
mod quasiquote;
mod special_forms;

pub use macros::{MacroBody, MacroDefinition, MacroExpander, MacroExpansionError, MacroRegistry};

#[cfg(test)]
mod tests;

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
            Ast::Vector(ref _elements) => Err(Error::new(
                ErrorKind::NotImplemented {
                    feature: "vector literals",
                },
                self.location(expr.span),
            )),
            Ast::Map(ref _elements) => Err(Error::new(
                ErrorKind::NotImplemented {
                    feature: "map literals",
                },
                self.location(expr.span),
            )),
            Ast::Set(ref elements) => self.compile_set(elements, expr.span),

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
            // Use err.kind since neither Error type implements Display directly
            Self::Parse(ref err) => write!(f, "parse error: {}", err.kind),
            Self::Compile(ref err) => write!(f, "compile error: {}", err.kind),
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
/// # Arguments
///
/// * `source` - The Lonala source code to compile.
/// * `source_id` - Identifies the source for error reporting.
/// * `interner` - Symbol interner for interning identifiers.
///
/// # Errors
///
/// Returns `CompileError::Parse` if parsing fails, or
/// `CompileError::Compile` if compilation fails.
///
/// # Example
///
/// ```
/// use lona_core::source;
/// use lona_core::symbol::Interner;
/// use lonala_compiler::compile;
///
/// let mut interner = Interner::new();
/// let source_id = source::Id::new(0);
/// let chunk = compile("(+ 1 2)", source_id, &mut interner).unwrap();
/// ```
#[inline]
pub fn compile(
    source: &str,
    source_id: source::Id,
    interner: &mut symbol::Interner,
) -> Result<Chunk, CompileError> {
    let mut registry = MacroRegistry::new();
    compile_with_registry(source, source_id, interner, &mut registry)
}

/// Compiles Lonala source code to bytecode with a persistent macro registry.
///
/// This is the full-featured compilation function that accepts an external
/// macro registry. Use this for REPL sessions where macros should persist
/// across evaluations.
///
/// # Arguments
///
/// * `source` - The Lonala source code to compile.
/// * `source_id` - Identifies the source for error reporting.
/// * `interner` - Symbol interner for interning identifiers.
/// * `registry` - Macro registry for persistent macro definitions.
///
/// # Errors
///
/// Returns `CompileError::Parse` if parsing fails, or
/// `CompileError::Compile` if compilation fails.
///
/// # Example
///
/// ```
/// use lona_core::source;
/// use lona_core::symbol::Interner;
/// use lonala_compiler::{compile_with_registry, MacroRegistry};
///
/// let mut interner = Interner::new();
/// let mut registry = MacroRegistry::new();
/// let source_id = source::Id::new(0);
///
/// // Define a macro
/// let chunk1 = compile_with_registry(
///     "(defmacro double [x] (list '+ x x))",
///     source_id,
///     &mut interner,
///     &mut registry
/// ).unwrap();
///
/// // Use the macro (it persists in the registry)
/// let chunk2 = compile_with_registry(
///     "(double 5)",
///     source_id,
///     &mut interner,
///     &mut registry
/// ).unwrap();
/// ```
#[inline]
pub fn compile_with_registry(
    source: &str,
    source_id: source::Id,
    interner: &mut symbol::Interner,
    registry: &mut MacroRegistry,
) -> Result<Chunk, CompileError> {
    let exprs = lonala_parser::parse(source, source_id)?;
    let mut compiler = Compiler::new(interner, registry, source_id);
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
/// # Arguments
///
/// * `source` - The Lonala source code to compile.
/// * `source_id` - Identifies the source for error reporting.
/// * `interner` - Symbol interner for interning identifiers.
/// * `registry` - Macro registry for persistent macro definitions.
/// * `expander` - Macro expander for compile-time expansion.
///
/// # Errors
///
/// Returns `CompileError::Parse` if parsing fails, or
/// `CompileError::Compile` if compilation or macro expansion fails.
///
/// # Example
///
/// ```ignore
/// use lona_core::source;
/// use lona_core::symbol::Interner;
/// use lonala_compiler::{compile_with_expansion, MacroRegistry, MacroExpander};
///
/// let mut interner = Interner::new();
/// let mut registry = MacroRegistry::new();
/// let mut expander = MyMacroExpander::new(); // Implements MacroExpander
/// let source_id = source::Id::new(0);
///
/// // Define a macro
/// let chunk1 = compile_with_expansion(
///     "(defmacro double [x] `(+ ~x ~x))",
///     source_id,
///     &mut interner,
///     &mut registry,
///     &mut expander
/// ).unwrap();
///
/// // Use the macro - it will be expanded at compile time
/// let chunk2 = compile_with_expansion(
///     "(double 5)",
///     source_id,
///     &mut interner,
///     &mut registry,
///     &mut expander
/// ).unwrap();
/// ```
#[inline]
pub fn compile_with_expansion(
    source: &str,
    source_id: source::Id,
    interner: &mut symbol::Interner,
    registry: &mut MacroRegistry,
    expander: &mut dyn MacroExpander,
) -> Result<Chunk, CompileError> {
    let exprs = lonala_parser::parse(source, source_id)?;
    let mut compiler = Compiler::with_expander(interner, registry, source_id, expander);
    let chunk = compiler.compile_program(&exprs)?;
    Ok(chunk)
}
