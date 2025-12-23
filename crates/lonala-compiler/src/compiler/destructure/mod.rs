// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Destructuring pattern parsing and compilation.
//!
//! This module provides pattern IR types, parsing logic, and bytecode emission
//! for destructuring in binding forms (`let`, `fn`, `loop`).
//!
//! # Sequential Destructuring
//!
//! Sequential patterns destructure vectors and lists:
//!
//! ```clojure
//! (let [[a b c] [1 2 3]] a)           ; Fixed elements
//! (let [[a & rest] [1 2 3]] rest)     ; Rest binding
//! (let [[a _ c] [1 2 3]] c)           ; Ignore with _
//! (let [[a :as all] [1 2]] all)       ; Whole binding
//! (let [[[x y] z] [[1 2] 3]] x)       ; Nested patterns
//! ```
//!
//! # Associative Destructuring
//!
//! Map patterns destructure maps:
//!
//! ```clojure
//! (let [{:keys [a b]} {:a 1 :b 2}] a)        ; Keyword keys
//! (let [{:strs [name]} {"name" "Alice"}] name) ; String keys
//! (let [{x :foo} {:foo 42}] x)               ; Explicit binding
//! (let [{:keys [a] :or {a 0}} {}] a)         ; Default value
//! ```
//!
//! # Compilation Strategy
//!
//! Patterns are compiled to bytecode using primitives:
//! - Sequential: `first`/`rest` for element extraction
//! - Associative: `get` for map lookup
//!
//! The compiler uses `GetGlobal` to resolve primitives, preserving late
//! binding (hot-patching affects destructuring behavior).

mod compile;
mod parse;

use alloc::boxed::Box;
use alloc::vec::Vec;

use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::Spanned;

pub use parse::{parse_map_pattern, parse_sequential_pattern};

use super::Ast;

/// A binding target in a destructuring pattern.
///
/// Each variant represents a different way to bind a value from the source
/// collection during destructuring.
#[derive(Debug, Clone, PartialEq)]
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

    /// Nested associative (map) pattern.
    ///
    /// Example: `{:keys [x y]}` in function parameters destructures a map
    /// argument to extract keyword keys.
    Map(Box<MapPattern>),
}

/// A sequential destructuring pattern.
///
/// Represents a vector pattern like `[a b & rest :as all]` with:
/// - Fixed positional bindings (`items`)
/// - Optional rest binding for remaining elements (`rest`)
/// - Optional `:as` binding for the whole collection (`as_binding`)
#[derive(Debug, Clone, PartialEq)]
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

/// An associative (map) destructuring pattern.
///
/// Represents a map pattern like `{:keys [a b] :or {b 0} :as m}` with:
/// - Shorthand bindings via `:keys`, `:strs`, `:syms`
/// - Explicit bindings like `{a :key-a}` (symbol bound to key lookup)
/// - Nested pattern bindings like `{[a b] :point}` or `{{:keys [x]} :inner}`
/// - Default values via `:or`
/// - Whole-map binding via `:as`
///
/// # Compilation Strategy
///
/// Map patterns are compiled using the `get` primitive:
/// 1. Optionally bind `:as` first (binds original map)
/// 2. For each binding, emit `(get map key)` via `GetGlobal`
/// 3. For each symbol with `:or` default, emit nil check and conditional
/// 4. For nested patterns, recurse into sequential or map compilation
///
/// # Example Patterns
///
/// ```text
/// {:keys [a b]}           ; extract :a and :b as keywords
/// {:strs [name]}          ; extract "name" as string key
/// {:syms [x]}             ; extract 'x as symbol key
/// {a :key-a}              ; bind a to (get m :key-a)
/// {[a b] :point}          ; nested sequential destructuring
/// {{:keys [x]} :inner}    ; nested map destructuring
/// {:keys [a] :or {a 0}}   ; default a to 0 if nil
/// {:keys [a] :as m}       ; also bind entire map to m
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MapPattern {
    /// Explicit bindings: pattern-to-key mappings.
    ///
    /// - `{a :key-a}` becomes `(Binding::Symbol(a), :key-a)`
    /// - `{[a b] :point}` becomes `(Binding::Seq(...), :point)`
    /// - `{{:keys [x]} :inner}` becomes `(Binding::Map(...), :inner)`
    ///
    /// The key expression is an arbitrary AST that will be compiled to produce
    /// the lookup key.
    pub explicit: Vec<(Binding, Spanned<Ast>)>,

    /// `:keys [a b]` - symbols to extract using keyword keys.
    ///
    /// Each symbol `foo` is extracted via `(get map :foo)`.
    pub keys: Vec<symbol::Id>,

    /// `:strs [a b]` - symbols to extract using string keys.
    ///
    /// Each symbol `foo` is extracted via `(get map "foo")`.
    pub strs: Vec<symbol::Id>,

    /// `:syms [a b]` - symbols to extract using symbol keys.
    ///
    /// Each symbol `foo` is extracted via `(get map 'foo)`.
    pub syms: Vec<symbol::Id>,

    /// `:or {a default}` - default expressions for symbols when value is nil.
    ///
    /// Applied after all extractions. Defaults trigger only on nil (not false).
    pub defaults: Vec<(symbol::Id, Spanned<Ast>)>,

    /// `:as name` - bind entire map to symbol.
    pub as_binding: Option<symbol::Id>,

    /// Span of the pattern for error reporting.
    pub span: Span,
}

impl MapPattern {
    /// Creates a new empty map pattern.
    #[inline]
    #[must_use]
    pub const fn new(span: Span) -> Self {
        Self {
            explicit: Vec::new(),
            keys: Vec::new(),
            strs: Vec::new(),
            syms: Vec::new(),
            defaults: Vec::new(),
            as_binding: None,
            span,
        }
    }
}
