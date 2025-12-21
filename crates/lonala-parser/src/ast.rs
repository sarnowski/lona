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
mod tests {
    extern crate alloc;

    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec;

    use super::*;

    // ==================== AST Node Construction ====================

    #[test]
    fn integer_construction() {
        let ast = Ast::integer(42_i64);
        assert_eq!(ast, Ast::Integer(42_i64));
    }

    #[test]
    fn float_construction() {
        let ast = Ast::float(3.14_f64);
        assert_eq!(ast, Ast::Float(3.14_f64));
    }

    #[test]
    fn string_construction() {
        let ast = Ast::string("hello");
        assert_eq!(ast, Ast::String("hello".to_string()));
    }

    #[test]
    fn bool_construction() {
        assert_eq!(Ast::bool(true), Ast::Bool(true));
        assert_eq!(Ast::bool(false), Ast::Bool(false));
    }

    #[test]
    fn nil_construction() {
        assert_eq!(Ast::nil(), Ast::Nil);
    }

    #[test]
    fn symbol_construction() {
        let ast = Ast::symbol("foo");
        assert_eq!(ast, Ast::Symbol("foo".to_string()));
    }

    #[test]
    fn keyword_construction() {
        let ast = Ast::keyword("key");
        assert_eq!(ast, Ast::Keyword("key".to_string()));
    }

    #[test]
    fn list_construction() {
        let elements = vec![
            Spanned::new(Ast::symbol("+"), Span::new(1_usize, 2_usize)),
            Spanned::new(Ast::integer(1_i64), Span::new(3_usize, 4_usize)),
            Spanned::new(Ast::integer(2_i64), Span::new(5_usize, 6_usize)),
        ];
        let ast = Ast::list(elements.clone());
        assert_eq!(ast, Ast::List(elements));
    }

    #[test]
    fn vector_construction() {
        let elements = vec![
            Spanned::new(Ast::integer(1_i64), Span::new(1_usize, 2_usize)),
            Spanned::new(Ast::integer(2_i64), Span::new(3_usize, 4_usize)),
        ];
        let ast = Ast::vector(elements.clone());
        assert_eq!(ast, Ast::Vector(elements));
    }

    #[test]
    fn map_construction() {
        let elements = vec![
            Spanned::new(Ast::keyword("a"), Span::new(1_usize, 3_usize)),
            Spanned::new(Ast::integer(1_i64), Span::new(4_usize, 5_usize)),
        ];
        let ast = Ast::map(elements.clone());
        assert_eq!(ast, Ast::Map(elements));
    }

    // ==================== Type Names ====================

    #[test]
    fn type_names() {
        assert_eq!(Ast::integer(0_i64).type_name(), "integer");
        assert_eq!(Ast::float(0.0_f64).type_name(), "float");
        assert_eq!(Ast::string("").type_name(), "string");
        assert_eq!(Ast::bool(true).type_name(), "boolean");
        assert_eq!(Ast::nil().type_name(), "nil");
        assert_eq!(Ast::symbol("x").type_name(), "symbol");
        assert_eq!(Ast::keyword("k").type_name(), "keyword");
        assert_eq!(Ast::list(vec![]).type_name(), "list");
        assert_eq!(Ast::vector(vec![]).type_name(), "vector");
        assert_eq!(Ast::map(vec![]).type_name(), "map");
    }

    // ==================== Display ====================

    #[test]
    fn display_integer() {
        assert_eq!(format!("{}", Ast::integer(42_i64)), "42");
        assert_eq!(format!("{}", Ast::integer(-17_i64)), "-17");
    }

    #[test]
    fn display_float() {
        assert_eq!(format!("{}", Ast::float(3.14_f64)), "3.14");
    }

    #[test]
    fn display_float_nan() {
        assert_eq!(format!("{}", Ast::float(f64::NAN)), "##NaN");
    }

    #[test]
    fn display_float_infinity() {
        assert_eq!(format!("{}", Ast::float(f64::INFINITY)), "##Inf");
        assert_eq!(format!("{}", Ast::float(f64::NEG_INFINITY)), "##-Inf");
    }

    #[test]
    fn display_string() {
        assert_eq!(format!("{}", Ast::string("hello")), "\"hello\"");
    }

    #[test]
    fn display_bool() {
        assert_eq!(format!("{}", Ast::bool(true)), "true");
        assert_eq!(format!("{}", Ast::bool(false)), "false");
    }

    #[test]
    fn display_nil() {
        assert_eq!(format!("{}", Ast::nil()), "nil");
    }

    #[test]
    fn display_symbol() {
        assert_eq!(format!("{}", Ast::symbol("foo")), "foo");
        assert_eq!(format!("{}", Ast::symbol("+")), "+");
    }

    #[test]
    fn display_keyword() {
        assert_eq!(format!("{}", Ast::keyword("key")), ":key");
    }

    #[test]
    fn display_empty_list() {
        assert_eq!(format!("{}", Ast::list(vec![])), "()");
    }

    #[test]
    fn display_list_with_elements() {
        let elements = vec![
            Spanned::new(Ast::symbol("+"), Span::new(1_usize, 2_usize)),
            Spanned::new(Ast::integer(1_i64), Span::new(3_usize, 4_usize)),
            Spanned::new(Ast::integer(2_i64), Span::new(5_usize, 6_usize)),
        ];
        assert_eq!(format!("{}", Ast::list(elements)), "(+ 1 2)");
    }

    #[test]
    fn display_empty_vector() {
        assert_eq!(format!("{}", Ast::vector(vec![])), "[]");
    }

    #[test]
    fn display_vector_with_elements() {
        let elements = vec![
            Spanned::new(Ast::integer(1_i64), Span::new(1_usize, 2_usize)),
            Spanned::new(Ast::integer(2_i64), Span::new(3_usize, 4_usize)),
            Spanned::new(Ast::integer(3_i64), Span::new(5_usize, 6_usize)),
        ];
        assert_eq!(format!("{}", Ast::vector(elements)), "[1 2 3]");
    }

    #[test]
    fn display_empty_map() {
        assert_eq!(format!("{}", Ast::map(vec![])), "{}");
    }

    #[test]
    fn display_map_with_elements() {
        let elements = vec![
            Spanned::new(Ast::keyword("a"), Span::new(1_usize, 3_usize)),
            Spanned::new(Ast::integer(1_i64), Span::new(4_usize, 5_usize)),
        ];
        assert_eq!(format!("{}", Ast::map(elements)), "{:a 1}");
    }

    // ==================== Spanned ====================

    #[test]
    fn spanned_construction() {
        let spanned = Spanned::new(Ast::integer(42_i64), Span::new(0_usize, 2_usize));
        assert_eq!(spanned.node, Ast::Integer(42_i64));
        assert_eq!(spanned.span, Span::new(0_usize, 2_usize));
    }

    #[test]
    fn spanned_new_defaults_full_span_to_span() {
        let spanned = Spanned::new(Ast::integer(42_i64), Span::new(5_usize, 7_usize));
        assert_eq!(spanned.span, Span::new(5_usize, 7_usize));
        assert_eq!(spanned.full_span, Span::new(5_usize, 7_usize));
    }

    #[test]
    fn spanned_with_full_span_sets_both() {
        let spanned = Spanned::with_full_span(
            Ast::integer(42_i64),
            Span::new(10_usize, 12_usize),
            Span::new(0_usize, 12_usize),
        );
        assert_eq!(spanned.span, Span::new(10_usize, 12_usize));
        assert_eq!(spanned.full_span, Span::new(0_usize, 12_usize));
    }

    #[test]
    fn spanned_map() {
        let spanned = Spanned::new(42_i64, Span::new(0_usize, 2_usize));
        let mapped = spanned.map(|n| n.saturating_mul(2_i64));
        assert_eq!(mapped.node, 84_i64);
        assert_eq!(mapped.span, Span::new(0_usize, 2_usize));
    }

    #[test]
    fn spanned_map_preserves_full_span() {
        let spanned = Spanned::with_full_span(
            42_i64,
            Span::new(10_usize, 12_usize),
            Span::new(0_usize, 12_usize),
        );
        let mapped = spanned.map(|n| n.saturating_mul(2_i64));
        assert_eq!(mapped.node, 84_i64);
        assert_eq!(mapped.span, Span::new(10_usize, 12_usize));
        assert_eq!(mapped.full_span, Span::new(0_usize, 12_usize));
    }

    #[test]
    fn spanned_display() {
        let spanned = Spanned::new(Ast::integer(42_i64), Span::new(0_usize, 2_usize));
        assert_eq!(format!("{spanned}"), "42");
    }

    // ==================== Equality ====================

    #[test]
    fn ast_equality() {
        assert_eq!(Ast::integer(42_i64), Ast::integer(42_i64));
        assert_ne!(Ast::integer(42_i64), Ast::integer(43_i64));
        assert_ne!(Ast::integer(42_i64), Ast::float(42.0_f64));
    }

    #[test]
    fn spanned_equality() {
        let a = Spanned::new(Ast::integer(42_i64), Span::new(0_usize, 2_usize));
        let b = Spanned::new(Ast::integer(42_i64), Span::new(0_usize, 2_usize));
        let c = Spanned::new(Ast::integer(42_i64), Span::new(1_usize, 3_usize));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn spanned_equality_includes_full_span() {
        let a = Spanned::with_full_span(
            Ast::integer(42_i64),
            Span::new(10_usize, 12_usize),
            Span::new(0_usize, 12_usize),
        );
        let b = Spanned::with_full_span(
            Ast::integer(42_i64),
            Span::new(10_usize, 12_usize),
            Span::new(0_usize, 12_usize),
        );
        let c = Spanned::with_full_span(
            Ast::integer(42_i64),
            Span::new(10_usize, 12_usize),
            Span::new(5_usize, 12_usize),
        );
        assert_eq!(a, b);
        assert_ne!(a, c); // Different full_span
    }
}
