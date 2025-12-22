// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Compiler from Lonala AST to bytecode.
//!
//! Transforms parsed [`Ast`] expressions into executable [`Chunk`] bytecode.
//! This module implements the core compilation logic for the Lonala language.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use lona_core::chunk::{Chunk, UpvalueSource};
use lona_core::opcode::{Opcode, encode_abc};
use lona_core::source;
use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use crate::error::{Error, Kind as ErrorKind, SourceLocation};

// Submodules containing the actual compilation logic
mod calls;
pub mod conversion;
pub mod destructure;
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

// =============================================================================
// Upvalue Tracking for Closures
// =============================================================================

/// Tracks an upvalue captured by the function being compiled.
#[derive(Debug, Clone)]
pub(crate) struct UpvalueInfo {
    /// The symbol being captured.
    pub symbol: symbol::Id,
    /// How to capture this upvalue at runtime.
    pub source: UpvalueSource,
}

/// Information about a variable available for capture from a parent scope.
#[derive(Debug, Clone)]
pub(crate) struct ParentLocal {
    /// The register in the parent where this variable is stored.
    pub register: u8,
}

/// Context for capturing variables from enclosing scopes.
///
/// When compiling a nested function, this describes what variables are
/// available for capture from the immediately enclosing function.
#[derive(Debug, Clone, Default)]
pub(crate) struct CaptureContext {
    /// Variables defined as locals in the parent function.
    /// Maps symbol ID to register index.
    pub parent_locals: BTreeMap<symbol::Id, ParentLocal>,
    /// Upvalues available in the parent function.
    /// Maps symbol ID to upvalue index in parent's upvalue array.
    /// Used for nested closures capturing from grandparent+ scopes.
    pub parent_upvalues: BTreeMap<symbol::Id, u8>,
}

impl CaptureContext {
    /// Creates an empty capture context (for top-level or non-closure functions).
    pub const fn new() -> Self {
        Self {
            parent_locals: BTreeMap::new(),
            parent_upvalues: BTreeMap::new(),
        }
    }
}

/// Result of resolving a symbol during compilation.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SymbolResolution {
    /// Symbol is a local variable in the current function.
    Local(u8),
    /// Symbol is captured as an upvalue.
    Upvalue(u8),
    /// Symbol is a global variable.
    Global,
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
    // Upvalue Resolution
    // =========================================================================

    /// Resolves a symbol, returning how to access it.
    ///
    /// Resolution order:
    /// 1. Local variables in the current function
    /// 2. Already-captured upvalues
    /// 3. Attempt to capture from parent scope
    /// 4. Fall back to global lookup
    pub(crate) fn resolve_symbol(&mut self, symbol: symbol::Id) -> SymbolResolution {
        // 1. Check locals in current function
        if let Some(reg) = self.locals.lookup(symbol) {
            return SymbolResolution::Local(reg);
        }

        // 2. Check if already captured as upvalue
        if let Some(idx) = self.lookup_upvalue(symbol) {
            return SymbolResolution::Upvalue(idx);
        }

        // 3. Try to capture from enclosing scope
        if let Some(idx) = self.try_capture_upvalue(symbol) {
            return SymbolResolution::Upvalue(idx);
        }

        // 4. Must be a global
        SymbolResolution::Global
    }

    /// Checks if a symbol is already captured as an upvalue.
    ///
    /// Returns the upvalue index if found.
    fn lookup_upvalue(&self, symbol: symbol::Id) -> Option<u8> {
        for (idx, upvalue) in self.upvalues.iter().enumerate() {
            if upvalue.symbol == symbol {
                return u8::try_from(idx).ok();
            }
        }
        None
    }

    /// Attempts to capture a variable from an enclosing scope.
    ///
    /// Returns the upvalue index if the variable was successfully captured.
    fn try_capture_upvalue(&mut self, symbol: symbol::Id) -> Option<u8> {
        // First check if it's a local in the parent scope
        if let Some(parent_local) = self.capture_context.parent_locals.get(&symbol) {
            // Capture from parent's local register
            return self.add_upvalue(symbol, UpvalueSource::Local(parent_local.register));
        }

        // Then check if it's an upvalue in the parent scope (for nested closures)
        if let Some(&parent_upvalue_idx) = self.capture_context.parent_upvalues.get(&symbol) {
            // Capture from parent's upvalue array
            return self.add_upvalue(symbol, UpvalueSource::ParentUpvalue(parent_upvalue_idx));
        }

        None
    }

    /// Adds a new upvalue to the current function.
    ///
    /// Returns the upvalue index, or `None` if the upvalue array is full.
    fn add_upvalue(&mut self, symbol: symbol::Id, source: UpvalueSource) -> Option<u8> {
        // Check if already captured (shouldn't happen if called correctly, but be safe)
        if let Some(idx) = self.lookup_upvalue(symbol) {
            return Some(idx);
        }

        // Check limit (max 256 upvalues)
        let idx = u8::try_from(self.upvalues.len()).ok()?;
        self.upvalues.push(UpvalueInfo { symbol, source });
        Some(idx)
    }

    /// Returns the collected upvalue sources for the current function.
    ///
    /// Called after compilation to get the upvalue descriptors for `FunctionBodyData`.
    pub(crate) fn take_upvalue_sources(&self) -> Vec<UpvalueSource> {
        self.upvalues.iter().map(|info| info.source).collect()
    }

    /// Builds a capture context from the current compiler's state.
    ///
    /// Used when creating a child compiler for a nested function.
    /// The context includes all locals in the current function and any upvalues
    /// that the current function has captured.
    ///
    /// For transitive closure support, this also preemptively captures any
    /// variables available through our own capture context. This ensures that
    /// deeply nested functions can access variables from grandparent+ scopes.
    pub(crate) fn build_capture_context(&mut self) -> CaptureContext {
        let mut parent_locals = BTreeMap::new();

        // Collect all locals from all scopes in the current function
        for scope in &self.locals.scopes {
            for (&symbol, &register) in scope {
                let _existing = parent_locals.insert(symbol, ParentLocal { register });
            }
        }

        // Preemptively capture variables from our capture_context that we haven't
        // captured yet. This enables transitive capture for nested closures.
        // For example, in (fn [a] (fn [] (fn [] a))):
        //   - The middle function (fn [] ...) has access to `a` via its capture_context
        //   - The inner function (fn [] a) needs to capture `a` from the middle function
        //   - So the middle function must first capture `a` to make it available
        for (&symbol, parent_local) in &self.capture_context.parent_locals.clone() {
            if !parent_locals.contains_key(&symbol) {
                // Capture from our parent's local register
                let _idx = self.add_upvalue(symbol, UpvalueSource::Local(parent_local.register));
            }
        }
        for (&symbol, &upvalue_idx) in &self.capture_context.parent_upvalues.clone() {
            if !parent_locals.contains_key(&symbol) && self.lookup_upvalue(symbol).is_none() {
                // Capture from our parent's upvalue array
                let _idx = self.add_upvalue(symbol, UpvalueSource::ParentUpvalue(upvalue_idx));
            }
        }

        // Now collect all upvalues (including newly captured ones)
        let mut parent_upvalues = BTreeMap::new();
        for (idx, upvalue) in self.upvalues.iter().enumerate() {
            if let Ok(idx_u8) = u8::try_from(idx) {
                let _existing = parent_upvalues.insert(upvalue.symbol, idx_u8);
            }
        }

        CaptureContext {
            parent_locals,
            parent_upvalues,
        }
    }

    /// Sets the capture context for this compiler.
    ///
    /// Called when creating a child compiler to enable upvalue capture.
    pub(crate) fn set_capture_context(&mut self, context: CaptureContext) {
        self.capture_context = context;
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
