// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Abstract Syntax Tree types for Lonala expressions.
//!
//! This module defines the AST representation produced by the parser. AST nodes
//! carry source location information via `Spanned<T>` wrappers, enabling precise
//! error messages throughout compilation.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::error::Span;

/// Abstract Syntax Tree node for Lonala expressions.
///
/// Each variant represents a syntactic element in the Lonala language.
/// AST nodes are separate from runtime `Value` types to maintain clean
/// separation between parsing and evaluation phases.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Ast {
    // Literals
    /// Integer literal (e.g., `42`, `-17`, `0xFF`).
    Integer(i64),
    /// Floating-point literal (e.g., `3.14`, `##NaN`).
    Float(f64),
    /// String literal with escapes processed (e.g., `"hello\nworld"`).
    String(String),
    /// Boolean literal (`true` or `false`).
    Bool(bool),
    /// Nil literal.
    Nil,

    // Identifiers
    /// Symbol (e.g., `foo`, `+`, `ns/name`).
    Symbol(String),
    /// Keyword (e.g., `:foo`, `:ns/name`).
    Keyword(String),

    // Collections
    /// List `(...)` - function calls, special forms.
    List(Vec<Spanned<Self>>),
    /// Vector `[...]` - data structure.
    Vector(Vec<Spanned<Self>>),
    /// Map `{...}` - key-value pairs (must have even number of elements).
    Map(Vec<Spanned<Self>>),
    /// Set `#{...}` - unique elements, no duplicates allowed.
    Set(Vec<Spanned<Self>>),

    /// A form with metadata attached.
    ///
    /// Created by the `^` reader macro. The metadata is always a map
    /// (possibly expanded from shorthand like `^:keyword`).
    WithMeta {
        /// The metadata map.
        meta: Box<Spanned<Self>>,
        /// The form to attach metadata to.
        value: Box<Spanned<Self>>,
    },
}

impl Ast {
    /// Creates an integer AST node.
    #[inline]
    #[must_use]
    pub const fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    /// Creates a float AST node.
    #[inline]
    #[must_use]
    pub const fn float(value: f64) -> Self {
        Self::Float(value)
    }

    /// Creates a string AST node.
    #[inline]
    #[must_use]
    pub fn string<S: Into<String>>(value: S) -> Self {
        Self::String(value.into())
    }

    /// Creates a boolean AST node.
    #[inline]
    #[must_use]
    pub const fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    /// Creates a nil AST node.
    #[inline]
    #[must_use]
    pub const fn nil() -> Self {
        Self::Nil
    }

    /// Creates a symbol AST node.
    #[inline]
    #[must_use]
    pub fn symbol<S: Into<String>>(name: S) -> Self {
        Self::Symbol(name.into())
    }

    /// Creates a keyword AST node.
    #[inline]
    #[must_use]
    pub fn keyword<S: Into<String>>(name: S) -> Self {
        Self::Keyword(name.into())
    }

    /// Creates a list AST node.
    #[inline]
    #[must_use]
    pub const fn list(elements: Vec<Spanned<Self>>) -> Self {
        Self::List(elements)
    }

    /// Creates a vector AST node.
    #[inline]
    #[must_use]
    pub const fn vector(elements: Vec<Spanned<Self>>) -> Self {
        Self::Vector(elements)
    }

    /// Creates a map AST node.
    #[inline]
    #[must_use]
    pub const fn map(elements: Vec<Spanned<Self>>) -> Self {
        Self::Map(elements)
    }

    /// Creates a set AST node.
    #[inline]
    #[must_use]
    pub const fn set(elements: Vec<Spanned<Self>>) -> Self {
        Self::Set(elements)
    }

    /// Creates a `WithMeta` node.
    #[inline]
    #[must_use]
    pub fn with_meta(meta: Spanned<Self>, value: Spanned<Self>) -> Self {
        Self::WithMeta {
            meta: Box::new(meta),
            value: Box::new(value),
        }
    }

    /// Returns a human-readable type name for this AST node.
    #[inline]
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match *self {
            Self::Integer(_) => "integer",
            Self::Float(_) => "float",
            Self::String(_) => "string",
            Self::Bool(_) => "boolean",
            Self::Nil => "nil",
            Self::Symbol(_) => "symbol",
            Self::Keyword(_) => "keyword",
            Self::List(_) => "list",
            Self::Vector(_) => "vector",
            Self::Map(_) => "map",
            Self::Set(_) => "set",
            Self::WithMeta { .. } => "with-meta",
        }
    }
}

impl fmt::Display for Ast {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Integer(num) => write!(f, "{num}"),
            Self::Float(num) => {
                if num.is_nan() {
                    write!(f, "##NaN")
                } else if num.is_infinite() {
                    if num.is_sign_positive() {
                        write!(f, "##Inf")
                    } else {
                        write!(f, "##-Inf")
                    }
                } else {
                    write!(f, "{num}")
                }
            }
            Self::String(ref text) => write!(f, "\"{text}\""),
            Self::Bool(val) => write!(f, "{val}"),
            Self::Nil => write!(f, "nil"),
            Self::Symbol(ref name) => write!(f, "{name}"),
            Self::Keyword(ref name) => write!(f, ":{name}"),
            Self::List(ref elements) => {
                write!(f, "(")?;
                for (idx, elem) in elements.iter().enumerate() {
                    if idx > 0_usize {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", elem.node)?;
                }
                write!(f, ")")
            }
            Self::Vector(ref elements) => {
                write!(f, "[")?;
                for (idx, elem) in elements.iter().enumerate() {
                    if idx > 0_usize {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", elem.node)?;
                }
                write!(f, "]")
            }
            Self::Map(ref elements) => {
                write!(f, "{{")?;
                for (idx, elem) in elements.iter().enumerate() {
                    if idx > 0_usize {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", elem.node)?;
                }
                write!(f, "}}")
            }
            Self::Set(ref elements) => {
                write!(f, "#{{")?;
                for (idx, elem) in elements.iter().enumerate() {
                    if idx > 0_usize {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", elem.node)?;
                }
                write!(f, "}}")
            }
            Self::WithMeta {
                ref meta,
                ref value,
            } => {
                write!(f, "^")?;
                meta.node.fmt(f)?;
                write!(f, " ")?;
                value.node.fmt(f)
            }
        }
    }
}

/// AST node with source location information.
///
/// Wraps any AST node with its source span, enabling precise error messages
/// that can point to the exact location in source code.
///
/// The `full_span` field includes leading comments and whitespace, useful
/// for source lookup and documentation display.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Spanned<T> {
    /// The AST node.
    pub node: T,
    /// Source location of just the expression.
    pub span: Span,
    /// Source location including leading comments/whitespace.
    pub full_span: Span,
}

impl<T> Spanned<T> {
    /// Creates a new spanned node where `full_span` equals `span`.
    ///
    /// Use [`with_full_span`](Self::with_full_span) when leading trivia
    /// should be included in the full span.
    #[inline]
    #[must_use]
    pub const fn new(node: T, span: Span) -> Self {
        Self {
            node,
            span,
            full_span: span,
        }
    }

    /// Creates a new spanned node with an explicit full span.
    ///
    /// The `full_span` should start at leading comments/whitespace and
    /// end at the same position as `span`.
    #[inline]
    #[must_use]
    pub const fn with_full_span(node: T, span: Span, full_span: Span) -> Self {
        Self {
            node,
            span,
            full_span,
        }
    }

    /// Maps the inner node using a function, preserving both spans.
    #[inline]
    #[must_use]
    pub fn map<U, F: FnOnce(T) -> U>(self, func: F) -> Spanned<U> {
        Spanned {
            node: func(self.node),
            span: self.span,
            full_span: self.full_span,
        }
    }
}

impl<T: fmt::Display> fmt::Display for Spanned<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.node)
    }
}

#[cfg(test)]
#[path = "ast_tests.rs"]
mod tests;
