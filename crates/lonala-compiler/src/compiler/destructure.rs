// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Sequential destructuring pattern parsing and compilation.
//!
//! This module provides pattern IR types, parsing logic, and bytecode emission
//! for sequential destructuring in binding forms (`let`, `fn`, `loop`).
//! Destructuring allows binding parts of a collection to individual names:
//!
//! ```clojure
//! (let [[a b c] [1 2 3]] a)           ; Fixed elements
//! (let [[a & rest] [1 2 3]] rest)     ; Rest binding
//! (let [[a _ c] [1 2 3]] c)           ; Ignore with _
//! (let [[a :as all] [1 2]] all)       ; Whole binding
//! (let [[[x y] z] [[1 2] 3]] x)       ; Nested patterns
//! ```
//!
//! # Compilation Strategy
//!
//! Patterns are compiled to bytecode using `first` and `rest` primitives:
//! - `(first coll)` extracts the first element
//! - `(rest coll)` returns remaining elements as a list
//!
//! The compiler uses `GetGlobal` to resolve `first`/`rest`, avoiding local
//! capture issues while preserving late binding (hot-patching affects
//! destructuring behavior).
//!
//! # Algorithm
//!
//! 1. Optionally bind `:as` first (binds original collection)
//! 2. Initialize cursor register to source collection
//! 3. For each fixed item:
//!    - `head = (first cursor)` - extract element
//!    - Bind head via symbol/ignore/nested pattern
//!    - `cursor = (rest cursor)` - advance cursor
//! 4. For `& rest_binding`: bind cursor directly (already a list)

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abc, encode_abx};
use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use super::Compiler;
use crate::error::{Error, Kind as ErrorKind, SourceLocation};

/// A binding target in a destructuring pattern.
///
/// Each variant represents a different way to bind a value from the source
/// collection during destructuring.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Binding {
    /// Bind to a named symbol.
    ///
    /// Example: `a` in `[a b c]` binds the first element to symbol `a`.
    Symbol(symbol::Id),

    /// Ignore this position.
    ///
    /// Example: `_` in `[a _ c]` discards the second element.
    Ignore,

    /// Nested sequential pattern.
    ///
    /// Example: `[x y]` in `[[x y] z]` destructures the first element
    /// as another sequence.
    Seq(Box<SeqPattern>),
}

/// A sequential destructuring pattern.
///
/// Represents a vector pattern like `[a b & rest :as all]` with:
/// - Fixed positional bindings (`items`)
/// - Optional rest binding for remaining elements (`rest`)
/// - Optional `:as` binding for the whole collection (`as_binding`)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SeqPattern {
    /// Fixed positional bindings, matched left to right.
    pub items: Vec<Binding>,

    /// Optional rest binding (after `&`), receives remaining elements as a list.
    pub rest: Option<Box<Binding>>,

    /// Optional `:as` binding, receives the original collection unchanged.
    pub as_binding: Option<symbol::Id>,

    /// Span of the pattern for error reporting.
    pub span: Span,
}

impl SeqPattern {
    /// Creates a new empty sequential pattern.
    #[inline]
    #[must_use]
    pub const fn new(span: Span) -> Self {
        Self {
            items: Vec::new(),
            rest: None,
            as_binding: None,
            span,
        }
    }
}

/// Parses a sequential destructuring pattern from a vector AST.
///
/// # Arguments
///
/// * `interner` - Symbol interner for interning symbol names
/// * `ast` - The vector AST to parse as a pattern
/// * `source_id` - Source ID for error reporting
///
/// # Returns
///
/// A `SeqPattern` representing the parsed destructuring pattern.
///
/// # Errors
///
/// Returns an error if:
/// - Input is not a vector
/// - Duplicate `&` in pattern
/// - `:as` not followed by a symbol
/// - `&` not followed by a binding
/// - Invalid binding target (not symbol, `_`, or nested vector)
///
/// # Examples
///
/// ```text
/// [a b c]     -> 3 symbol bindings
/// [a & rest]  -> 1 symbol binding + rest binding
/// [a _ c]     -> symbol, ignore, symbol
/// [a :as all] -> 1 symbol + as binding
/// [[a b] c]   -> nested pattern + symbol
/// ```
#[inline]
pub fn parse_sequential_pattern(
    interner: &mut symbol::Interner,
    ast: &Spanned<Ast>,
    source_id: lona_core::source::Id,
) -> Result<SeqPattern, Error> {
    let Ast::Vector(ref elements) = ast.node else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: "expected vector pattern",
            },
            SourceLocation::new(source_id, ast.span),
        ));
    };

    let mut pattern = SeqPattern::new(ast.span);
    let mut idx = 0_usize;
    let mut seen_ampersand = false;
    let mut seen_as = false;
    let mut ctx = ParseContext {
        interner,
        elements,
        source_id,
    };

    while idx < elements.len() {
        let Some(elem) = elements.get(idx) else { break };

        match elem.node {
            // `&` introduces rest binding
            Ast::Symbol(ref name) if name == "&" => {
                if seen_ampersand {
                    return Err(Error::new(
                        ErrorKind::InvalidDestructuringPattern {
                            message: "duplicate & in pattern",
                        },
                        SourceLocation::new(source_id, elem.span),
                    ));
                }
                seen_ampersand = true;
                let rest_binding = parse_rest_binding(&mut ctx, elem, idx)?;
                pattern.rest = Some(Box::new(rest_binding));
                idx = idx.saturating_add(1); // Skip past the rest binding
            }

            // `:as` introduces whole-collection binding
            Ast::Keyword(ref name) if name == "as" => {
                if seen_as {
                    return Err(Error::new(
                        ErrorKind::InvalidDestructuringPattern {
                            message: "duplicate :as in pattern",
                        },
                        SourceLocation::new(source_id, elem.span),
                    ));
                }
                seen_as = true;
                pattern.as_binding = Some(parse_as_binding(&mut ctx, elem, idx)?);
                idx = idx.saturating_add(1); // Skip past the symbol
            }

            // Regular binding (symbol, _, or nested vector)
            // All other AST types fall through to error in parse_binding
            Ast::Symbol(_)
            | Ast::Vector(_)
            | Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Map(_)
            | Ast::Set(_)
            | Ast::WithMeta { .. }
            | _ => {
                if seen_ampersand {
                    return Err(Error::new(
                        ErrorKind::InvalidDestructuringPattern {
                            message: "unexpected binding after rest binding",
                        },
                        SourceLocation::new(source_id, elem.span),
                    ));
                }
                let binding = parse_binding(ctx.interner, elem, source_id)?;
                pattern.items.push(binding);
            }
        }

        idx = idx.saturating_add(1);
    }

    Ok(pattern)
}

/// Context for parsing within a pattern.
struct ParseContext<'elements> {
    interner: &'elements mut symbol::Interner,
    elements: &'elements [Spanned<Ast>],
    source_id: lona_core::source::Id,
}

/// Parses a rest binding after `&`.
///
/// Returns the index increment and the parsed binding.
fn parse_rest_binding(
    ctx: &mut ParseContext<'_>,
    ampersand_elem: &Spanned<Ast>,
    idx: usize,
) -> Result<Binding, Error> {
    let rest_idx = idx.saturating_add(1);
    let Some(rest_elem) = ctx.elements.get(rest_idx) else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: "& must be followed by a binding",
            },
            SourceLocation::new(ctx.source_id, ampersand_elem.span),
        ));
    };
    parse_binding(ctx.interner, rest_elem, ctx.source_id)
}

/// Parses an `:as` binding.
///
/// Returns the symbol ID for the binding.
fn parse_as_binding(
    ctx: &mut ParseContext<'_>,
    as_elem: &Spanned<Ast>,
    idx: usize,
) -> Result<symbol::Id, Error> {
    let sym_idx = idx.saturating_add(1);
    let Some(sym_elem) = ctx.elements.get(sym_idx) else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: ":as must be followed by a symbol",
            },
            SourceLocation::new(ctx.source_id, as_elem.span),
        ));
    };

    let Ast::Symbol(ref sym_name) = sym_elem.node else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: ":as must be followed by a symbol",
            },
            SourceLocation::new(ctx.source_id, sym_elem.span),
        ));
    };

    Ok(ctx.interner.intern(sym_name))
}

/// Parses a single binding target.
///
/// A binding can be:
/// - A symbol name (binds value to that name)
/// - `_` (ignores the value)
/// - A nested vector (recursive destructuring)
fn parse_binding(
    interner: &mut symbol::Interner,
    ast: &Spanned<Ast>,
    source_id: lona_core::source::Id,
) -> Result<Binding, Error> {
    match ast.node {
        Ast::Symbol(ref name) if name == "_" => Ok(Binding::Ignore),

        Ast::Symbol(ref name) => {
            let sym_id = interner.intern(name);
            Ok(Binding::Symbol(sym_id))
        }

        Ast::Vector(_) => {
            let nested = parse_sequential_pattern(interner, ast, source_id)?;
            Ok(Binding::Seq(Box::new(nested)))
        }

        // All other AST types are invalid binding targets
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
            ErrorKind::InvalidDestructuringPattern {
                message: "binding must be a symbol, _, or vector pattern",
            },
            SourceLocation::new(source_id, ast.span),
        )),
    }
}

// =============================================================================
// Compilation
// =============================================================================

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
            }
        } else {
            // No rest binding - cursor is temporary, free it
            self.free_registers_to(cursor_reg);
        }

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
}
