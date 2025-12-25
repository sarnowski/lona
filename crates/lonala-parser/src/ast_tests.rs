// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for AST types.

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
