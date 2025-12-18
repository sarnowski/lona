// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for reader macros: quote, syntax-quote, unquote, unquote-splice.

extern crate alloc;

use alloc::string::ToString;

use crate::ast::Ast;

use super::parse_ast;

// ==================== Reader Macros ====================

#[test]
fn parse_quote() {
    let ast = parse_ast("'x");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 2_usize);
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Symbol("quote".to_string()))
            );
            assert_eq!(
                elements.get(1_usize).map(|spanned| &spanned.node),
                Some(&Ast::Symbol("x".to_string()))
            );
        }
        _ => panic!("expected List for quote"),
    }
}

#[test]
fn parse_quote_list() {
    let ast = parse_ast("'(1 2 3)");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 2_usize);
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Symbol("quote".to_string()))
            );
            assert!(matches!(
                elements.get(1_usize).map(|spanned| &spanned.node),
                Some(Ast::List(_))
            ));
        }
        _ => panic!("expected List for quote"),
    }
}

#[test]
fn parse_syntax_quote() {
    let ast = parse_ast("`x");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 2_usize);
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Symbol("syntax-quote".to_string()))
            );
        }
        _ => panic!("expected List for syntax-quote"),
    }
}

#[test]
fn parse_unquote() {
    let ast = parse_ast("~x");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 2_usize);
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Symbol("unquote".to_string()))
            );
        }
        _ => panic!("expected List for unquote"),
    }
}

#[test]
fn parse_unquote_splice() {
    let ast = parse_ast("~@xs");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 2_usize);
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Symbol("unquote-splicing".to_string()))
            );
        }
        _ => panic!("expected List for unquote-splicing"),
    }
}

#[test]
fn parse_nested_reader_macros() {
    let ast = parse_ast("''x");
    // Should be (quote (quote x))
    match ast {
        Ast::List(outer) => {
            assert_eq!(outer.len(), 2_usize);
            assert_eq!(
                outer.first().map(|spanned| &spanned.node),
                Some(&Ast::Symbol("quote".to_string()))
            );
            match outer.get(1_usize).map(|spanned| &spanned.node) {
                Some(Ast::List(inner)) => {
                    assert_eq!(inner.len(), 2_usize);
                    assert_eq!(
                        inner.first().map(|spanned| &spanned.node),
                        Some(&Ast::Symbol("quote".to_string()))
                    );
                }
                _ => panic!("expected inner List"),
            }
        }
        _ => panic!("expected List"),
    }
}
