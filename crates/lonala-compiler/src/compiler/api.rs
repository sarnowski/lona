// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Public API for the Lonala compiler.
//!
//! This module provides the high-level compilation functions that combine
//! parsing and compilation into a single step.

use lona_core::chunk::Chunk;
use lona_core::source;
use lona_core::symbol;

use super::{Compiler, MacroExpander, MacroRegistry};
use crate::error::Error;

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
        // Use variant_name() for basic display. Rich formatting with context
        // and help text is provided by the Diagnostic trait in lonala-human.
        match *self {
            Self::Parse(ref err) => write!(f, "parse error: {}", err.kind.variant_name()),
            Self::Compile(ref err) => write!(f, "compile error: {}", err.kind.variant_name()),
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
