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

// ==================== Discard Reader Macro ====================

use super::parse_asts;
use crate::error::{Kind as ErrorKind, SourceId};
use crate::parser::parse_one;

/// Test source ID for tests that need direct parse calls.
const TEST_SOURCE_ID: SourceId = SourceId::new(0_u32);

#[test]
fn parse_discard_in_vector() {
    // [1 #_2 3] should parse to [1, 3]
    let ast = parse_ast("[1 #_2 3]");
    match ast {
        Ast::Vector(elems) => {
            assert_eq!(elems.len(), 2_usize);
            assert!(matches!(
                elems.first().map(|e| &e.node),
                Some(Ast::Integer(1_i64))
            ));
            assert!(matches!(
                elems.get(1_usize).map(|e| &e.node),
                Some(Ast::Integer(3_i64))
            ));
        }
        _ => panic!("expected Vector"),
    }
}

#[test]
fn parse_discard_chained() {
    // [1 #_#_2 3 4] should parse to [1, 4]
    // - First #_ calls parse_expr()
    // - parse_expr calls skip_discards() which handles second #_
    // - Second #_ discards 2, parse_expr returns 3
    // - First #_ discards 3
    // - Next element is 4
    let ast = parse_ast("[1 #_#_2 3 4]");
    match ast {
        Ast::Vector(elems) => {
            assert_eq!(elems.len(), 2_usize);
            assert!(matches!(
                elems.first().map(|e| &e.node),
                Some(Ast::Integer(1_i64))
            ));
            assert!(matches!(
                elems.get(1_usize).map(|e| &e.node),
                Some(Ast::Integer(4_i64))
            ));
        }
        _ => panic!("expected Vector"),
    }
}

#[test]
fn parse_discard_trailing() {
    // [1 #_2] should parse to [1]
    let ast = parse_ast("[1 #_2]");
    match ast {
        Ast::Vector(elems) => {
            assert_eq!(elems.len(), 1_usize);
            assert!(matches!(
                elems.first().map(|e| &e.node),
                Some(Ast::Integer(1_i64))
            ));
        }
        _ => panic!("expected Vector"),
    }
}

#[test]
fn parse_discard_complex_form() {
    // [1 #_(a b c) 2] should parse to [1, 2]
    let ast = parse_ast("[1 #_(a b c) 2]");
    match ast {
        Ast::Vector(elems) => {
            assert_eq!(elems.len(), 2_usize);
            assert!(matches!(
                elems.first().map(|e| &e.node),
                Some(Ast::Integer(1_i64))
            ));
            assert!(matches!(
                elems.get(1_usize).map(|e| &e.node),
                Some(Ast::Integer(2_i64))
            ));
        }
        _ => panic!("expected Vector"),
    }
}

#[test]
fn parse_discard_in_list() {
    // (+ 1 #_2 3) should parse to (+ 1 3)
    let ast = parse_ast("(+ 1 #_2 3)");
    match ast {
        Ast::List(elems) => {
            assert_eq!(elems.len(), 3_usize);
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn parse_discard_in_map() {
    // {:a 1 #_:b #_2 :c 3} should parse to {:a 1 :c 3}
    let ast = parse_ast("{:a 1 #_:b #_2 :c 3}");
    match ast {
        Ast::Map(elems) => {
            assert_eq!(elems.len(), 4_usize); // 2 key-value pairs = 4 elements
        }
        _ => panic!("expected Map"),
    }
}

#[test]
fn parse_discard_in_set() {
    // #{1 #_2 3} should parse to #{1 3}
    let ast = parse_ast("#{1 #_2 3}");
    match ast {
        Ast::Set(elems) => {
            assert_eq!(elems.len(), 2_usize);
        }
        _ => panic!("expected Set"),
    }
}

#[test]
fn parse_discard_at_top_level() {
    // #_1 2 should parse to just [2]
    let asts = parse_asts("#_1 2");
    assert_eq!(asts.len(), 1_usize);
    assert!(matches!(asts.first(), Some(Ast::Integer(2_i64))));
}

#[test]
fn parse_discard_all_at_top_level() {
    // #_1 should parse to empty
    let asts = parse_asts("#_1");
    assert!(asts.is_empty());
}

#[test]
fn parse_discard_at_eof_errors() {
    // #_ at EOF should error
    let result = parse_one("#_", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.err().expect("should be error");
    assert!(matches!(err.kind, ErrorKind::ReaderMacroMissingExpr));
}

#[test]
fn parse_discard_before_delimiter_errors() {
    // [1 #_] should error - #_ followed by closing delimiter
    let result = parse_one("[1 #_]", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.err().expect("should be error");
    assert!(matches!(err.kind, ErrorKind::ReaderMacroMissingExpr));
}
