// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for collection parsing: lists, vectors, maps.

extern crate alloc;

use alloc::string::ToString;
use alloc::vec;

use crate::ast::Ast;
use crate::error::{Kind as ErrorKind, SourceId};
use crate::parser::parse_one;

use super::parse_ast;

/// Test source ID for error tests.
const TEST_SOURCE_ID: SourceId = SourceId::new(0_u32);

// ==================== Collections: Lists ====================

#[test]
fn parse_empty_list() {
    assert_eq!(parse_ast("()"), Ast::List(vec![]));
}

#[test]
fn parse_list_with_elements() {
    let ast = parse_ast("(+ 1 2)");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 3_usize);
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Symbol("+".to_string()))
            );
            assert_eq!(
                elements.get(1_usize).map(|spanned| &spanned.node),
                Some(&Ast::Integer(1_i64))
            );
            assert_eq!(
                elements.get(2_usize).map(|spanned| &spanned.node),
                Some(&Ast::Integer(2_i64))
            );
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn parse_nested_lists() {
    let ast = parse_ast("((a) (b))");
    match ast {
        Ast::List(outer) => {
            assert_eq!(outer.len(), 2_usize);
            match &outer.first().map(|spanned| &spanned.node) {
                Some(Ast::List(inner)) => {
                    assert_eq!(inner.len(), 1_usize);
                }
                _ => panic!("expected inner List"),
            }
        }
        _ => panic!("expected List"),
    }
}

// ==================== Collections: Vectors ====================

#[test]
fn parse_empty_vector() {
    assert_eq!(parse_ast("[]"), Ast::Vector(vec![]));
}

#[test]
fn parse_vector_with_elements() {
    let ast = parse_ast("[1 2 3]");
    match ast {
        Ast::Vector(elements) => {
            assert_eq!(elements.len(), 3_usize);
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Integer(1_i64))
            );
            assert_eq!(
                elements.get(1_usize).map(|spanned| &spanned.node),
                Some(&Ast::Integer(2_i64))
            );
            assert_eq!(
                elements.get(2_usize).map(|spanned| &spanned.node),
                Some(&Ast::Integer(3_i64))
            );
        }
        _ => panic!("expected Vector"),
    }
}

// ==================== Collections: Maps ====================

#[test]
fn parse_empty_map() {
    assert_eq!(parse_ast("{}"), Ast::Map(vec![]));
}

#[test]
fn parse_map_with_entries() {
    let ast = parse_ast("{:a 1 :b 2}");
    match ast {
        Ast::Map(elements) => {
            assert_eq!(elements.len(), 4_usize);
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Keyword("a".to_string()))
            );
            assert_eq!(
                elements.get(1_usize).map(|spanned| &spanned.node),
                Some(&Ast::Integer(1_i64))
            );
            assert_eq!(
                elements.get(2_usize).map(|spanned| &spanned.node),
                Some(&Ast::Keyword("b".to_string()))
            );
            assert_eq!(
                elements.get(3_usize).map(|spanned| &spanned.node),
                Some(&Ast::Integer(2_i64))
            );
        }
        _ => panic!("expected Map"),
    }
}

#[test]
fn parse_nested_collections() {
    let ast = parse_ast("{:list (1 2) :vec [3 4]}");
    match ast {
        Ast::Map(elements) => {
            assert_eq!(elements.len(), 4_usize);
            assert!(matches!(
                elements.get(1_usize).map(|spanned| &spanned.node),
                Some(Ast::List(_))
            ));
            assert!(matches!(
                elements.get(3_usize).map(|spanned| &spanned.node),
                Some(Ast::Vector(_))
            ));
        }
        _ => panic!("expected Map"),
    }
}

// ==================== Complex Expressions ====================

#[test]
fn parse_function_definition() {
    let ast = parse_ast("(defn foo [x] x)");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 4_usize);
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Symbol("defn".to_string()))
            );
            assert_eq!(
                elements.get(1_usize).map(|spanned| &spanned.node),
                Some(&Ast::Symbol("foo".to_string()))
            );
            assert!(matches!(
                elements.get(2_usize).map(|spanned| &spanned.node),
                Some(Ast::Vector(_))
            ));
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn parse_let_binding() {
    let ast = parse_ast("(let [x 1] x)");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 3_usize);
        }
        _ => panic!("expected List"),
    }
}

// ==================== Map Duplicate Key Detection ====================

#[test]
fn map_duplicate_keyword_key_error() {
    let result = parse_one("{:a 1 :a 2}", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ErrorKind::DuplicateMapKey));
}

#[test]
fn map_duplicate_integer_key_error() {
    let result = parse_one("{1 :a 1 :b}", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ErrorKind::DuplicateMapKey));
}

#[test]
fn map_duplicate_string_key_error() {
    let result = parse_one("{\"key\" 1 \"key\" 2}", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ErrorKind::DuplicateMapKey));
}

#[test]
fn map_duplicate_symbol_key_error() {
    let result = parse_one("{foo 1 foo 2}", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ErrorKind::DuplicateMapKey));
}

#[test]
fn map_different_keys_ok() {
    let result = parse_one("{:a 1 :b 2 :c 3}", TEST_SOURCE_ID);
    assert!(result.is_ok());
}

#[test]
fn map_same_value_different_key_ok() {
    // Same values but different keys should be OK
    let result = parse_one("{:a 1 :b 1 :c 1}", TEST_SOURCE_ID);
    assert!(result.is_ok());
}

#[test]
fn map_different_type_same_text_ok() {
    // :abc (keyword) and abc (symbol) have the same text but are different types
    let result = parse_one("{:abc 1 abc 2}", TEST_SOURCE_ID);
    assert!(result.is_ok());
}

#[test]
fn map_string_vs_integer_key_ok() {
    // "1" (string) and 1 (integer) are different types
    let result = parse_one("{\"1\" :a 1 :b}", TEST_SOURCE_ID);
    assert!(result.is_ok());
}

// ==================== Anonymous Functions #() ====================

#[test]
fn anon_fn_basic() {
    // #(+ % 1) should expand to (fn [p1] (+ p1 1))
    let ast = parse_ast("#(+ % 1)");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 3_usize);
            // First element: fn symbol
            assert_eq!(
                elements.first().map(|spanned| &spanned.node),
                Some(&Ast::Symbol("fn".to_string()))
            );
            // Second element: [p1] vector
            match elements.get(1_usize).map(|spanned| &spanned.node) {
                Some(Ast::Vector(params)) => {
                    assert_eq!(params.len(), 1_usize);
                    assert_eq!(
                        params.first().map(|spanned| &spanned.node),
                        Some(&Ast::Symbol("p1".to_string()))
                    );
                }
                _ => panic!("expected Vector for params"),
            }
            // Third element: (+ p1 1) body
            match elements.get(2_usize).map(|spanned| &spanned.node) {
                Some(Ast::List(body)) => {
                    assert_eq!(body.len(), 3_usize);
                    assert_eq!(
                        body.get(1_usize).map(|spanned| &spanned.node),
                        Some(&Ast::Symbol("p1".to_string()))
                    );
                }
                _ => panic!("expected List for body"),
            }
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn anon_fn_multiple_args() {
    // #(+ %1 %2) should expand to (fn [p1 p2] (+ p1 p2))
    let ast = parse_ast("#(+ %1 %2)");
    match ast {
        Ast::List(elements) => {
            assert_eq!(elements.len(), 3_usize);
            match elements.get(1_usize).map(|spanned| &spanned.node) {
                Some(Ast::Vector(params)) => {
                    assert_eq!(params.len(), 2_usize);
                    assert_eq!(
                        params.first().map(|spanned| &spanned.node),
                        Some(&Ast::Symbol("p1".to_string()))
                    );
                    assert_eq!(
                        params.get(1_usize).map(|spanned| &spanned.node),
                        Some(&Ast::Symbol("p2".to_string()))
                    );
                }
                _ => panic!("expected Vector"),
            }
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn anon_fn_gap_in_args() {
    // #(%3) should expand to (fn [p1 p2 p3] (p3)) - fills in gaps
    let ast = parse_ast("#(%3)");
    match ast {
        Ast::List(elements) => match elements.get(1_usize).map(|spanned| &spanned.node) {
            Some(Ast::Vector(params)) => {
                assert_eq!(params.len(), 3_usize);
                assert_eq!(
                    params.first().map(|spanned| &spanned.node),
                    Some(&Ast::Symbol("p1".to_string()))
                );
                assert_eq!(
                    params.get(2_usize).map(|spanned| &spanned.node),
                    Some(&Ast::Symbol("p3".to_string()))
                );
            }
            _ => panic!("expected Vector"),
        },
        _ => panic!("expected List"),
    }
}

#[test]
fn anon_fn_rest_args() {
    // #(first %&) should expand to (fn [& rest] (first rest))
    let ast = parse_ast("#(first %&)");
    match ast {
        Ast::List(elements) => match elements.get(1_usize).map(|spanned| &spanned.node) {
            Some(Ast::Vector(params)) => {
                assert_eq!(params.len(), 2_usize);
                assert_eq!(
                    params.first().map(|spanned| &spanned.node),
                    Some(&Ast::Symbol("&".to_string()))
                );
                assert_eq!(
                    params.get(1_usize).map(|spanned| &spanned.node),
                    Some(&Ast::Symbol("rest".to_string()))
                );
            }
            _ => panic!("expected Vector"),
        },
        _ => panic!("expected List"),
    }
}

#[test]
fn anon_fn_no_args() {
    // #(+ 1 2) should expand to (fn [] (+ 1 2))
    let ast = parse_ast("#(+ 1 2)");
    match ast {
        Ast::List(elements) => match elements.get(1_usize).map(|spanned| &spanned.node) {
            Some(Ast::Vector(params)) => {
                assert_eq!(params.len(), 0_usize);
            }
            _ => panic!("expected Vector"),
        },
        _ => panic!("expected List"),
    }
}

#[test]
fn anon_fn_nested_error() {
    // Nested #() should be an error
    let result = parse_one("#(#(%)))", TEST_SOURCE_ID);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ErrorKind::NestedAnonFn));
}

#[test]
fn anon_fn_with_metadata_placeholder() {
    // #(^:tag %) should work - placeholder with metadata
    let ast = parse_ast("#(^:tag %)");
    match ast {
        Ast::List(elements) => {
            // Should have fn, [p1], and body with metadata-wrapped p1
            assert_eq!(elements.len(), 3_usize);
            match elements.get(1_usize).map(|spanned| &spanned.node) {
                Some(Ast::Vector(params)) => {
                    assert_eq!(params.len(), 1_usize);
                }
                _ => panic!("expected Vector"),
            }
        }
        _ => panic!("expected List"),
    }
}
