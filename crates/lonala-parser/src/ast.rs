// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Abstract Syntax Tree types for Lonala expressions.
//!
//! This module defines the AST representation produced by the parser. AST nodes
//! carry source location information via `Spanned<T>` wrappers, enabling precise
//! error messages throughout compilation.

extern crate alloc;

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
    List(Vec<Spanned<Ast>>),
    /// Vector `[...]` - data structure.
    Vector(Vec<Spanned<Ast>>),
    /// Map `{...}` - key-value pairs (must have even number of elements).
    Map(Vec<Spanned<Ast>>),
}

impl Ast {
    /// Creates an integer AST node.
    pub const fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    /// Creates a float AST node.
    pub const fn float(value: f64) -> Self {
        Self::Float(value)
    }

    /// Creates a string AST node.
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    /// Creates a boolean AST node.
    pub const fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    /// Creates a nil AST node.
    pub const fn nil() -> Self {
        Self::Nil
    }

    /// Creates a symbol AST node.
    pub fn symbol(name: impl Into<String>) -> Self {
        Self::Symbol(name.into())
    }

    /// Creates a keyword AST node.
    pub fn keyword(name: impl Into<String>) -> Self {
        Self::Keyword(name.into())
    }

    /// Creates a list AST node.
    pub fn list(elements: Vec<Spanned<Ast>>) -> Self {
        Self::List(elements)
    }

    /// Creates a vector AST node.
    pub fn vector(elements: Vec<Spanned<Ast>>) -> Self {
        Self::Vector(elements)
    }

    /// Creates a map AST node.
    pub fn map(elements: Vec<Spanned<Ast>>) -> Self {
        Self::Map(elements)
    }

    /// Returns a human-readable type name for this AST node.
    pub const fn type_name(&self) -> &'static str {
        match self {
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
        }
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(n) => write!(f, "{n}"),
            Self::Float(n) => {
                if n.is_nan() {
                    write!(f, "##NaN")
                } else if n.is_infinite() {
                    if n.is_sign_positive() {
                        write!(f, "##Inf")
                    } else {
                        write!(f, "##-Inf")
                    }
                } else {
                    write!(f, "{n}")
                }
            }
            Self::String(s) => write!(f, "\"{s}\""),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Nil => write!(f, "nil"),
            Self::Symbol(s) => write!(f, "{s}"),
            Self::Keyword(k) => write!(f, ":{k}"),
            Self::List(elements) => {
                write!(f, "(")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0_usize {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", elem.node)?;
                }
                write!(f, ")")
            }
            Self::Vector(elements) => {
                write!(f, "[")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0_usize {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", elem.node)?;
                }
                write!(f, "]")
            }
            Self::Map(elements) => {
                write!(f, "{{")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0_usize {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", elem.node)?;
                }
                write!(f, "}}")
            }
        }
    }
}

/// AST node with source location information.
///
/// Wraps any AST node with its source span, enabling precise error messages
/// that can point to the exact location in source code.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    /// The AST node.
    pub node: T,
    /// Source location.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Creates a new spanned node.
    pub const fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    /// Maps the inner node using a function, preserving the span.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }
}

impl<T: fmt::Display> fmt::Display for Spanned<T> {
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
    fn spanned_map() {
        let spanned = Spanned::new(42_i64, Span::new(0_usize, 2_usize));
        let mapped = spanned.map(|n| n.saturating_mul(2_i64));
        assert_eq!(mapped.node, 84_i64);
        assert_eq!(mapped.span, Span::new(0_usize, 2_usize));
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
}
