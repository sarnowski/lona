// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for map destructuring pattern parsing.

use alloc::vec;

use lona_core::symbol;
use lonala_parser::Ast;

use super::{map_elements, source_id, spanned, spanned_at};
use crate::compiler::destructure::{Binding, MAX_PATTERN_DEPTH, parse_map_pattern};
use crate::error::Kind as ErrorKind;

// ==================== Basic Map Pattern Parsing ====================

#[test]
fn parse_map_keys() {
    // {:keys [a b]} -> keys=[a, b]
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("keys".into()),
        Ast::Vector(vec![
            spanned(Ast::Symbol("a".into())),
            spanned(Ast::Symbol("b".into())),
        ]),
    )])));

    let pattern = parse_map_pattern(&interner, &ast, source_id(), 0).unwrap();

    assert_eq!(pattern.keys.len(), 2);
    assert!(pattern.strs.is_empty());
    assert!(pattern.syms.is_empty());
    assert!(pattern.explicit.is_empty());
    assert!(pattern.defaults.is_empty());
    assert!(pattern.as_binding.is_none());
}

#[test]
fn parse_map_strs() {
    // {:strs [name]} -> strs=[name]
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("strs".into()),
        Ast::Vector(vec![spanned(Ast::Symbol("name".into()))]),
    )])));

    let pattern = parse_map_pattern(&interner, &ast, source_id(), 0).unwrap();

    assert!(pattern.keys.is_empty());
    assert_eq!(pattern.strs.len(), 1);
    assert!(pattern.syms.is_empty());
}

#[test]
fn parse_map_syms() {
    // {:syms [x]} -> syms=[x]
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("syms".into()),
        Ast::Vector(vec![spanned(Ast::Symbol("x".into()))]),
    )])));

    let pattern = parse_map_pattern(&interner, &ast, source_id(), 0).unwrap();

    assert!(pattern.keys.is_empty());
    assert!(pattern.strs.is_empty());
    assert_eq!(pattern.syms.len(), 1);
}

#[test]
fn parse_map_or() {
    // {:or {a 0}} -> defaults=[(a, 0)]
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("or".into()),
        Ast::Map(map_elements(vec![(
            Ast::Symbol("a".into()),
            Ast::Integer(0),
        )])),
    )])));

    let pattern = parse_map_pattern(&interner, &ast, source_id(), 0).unwrap();

    assert_eq!(pattern.defaults.len(), 1);
}

#[test]
fn parse_map_as() {
    // {:as m} -> as_binding=Some(m)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("as".into()),
        Ast::Symbol("m".into()),
    )])));

    let pattern = parse_map_pattern(&interner, &ast, source_id(), 0).unwrap();

    assert!(pattern.as_binding.is_some());
}

#[test]
fn parse_map_explicit() {
    // {a :key-a} -> explicit=[(Binding::Symbol(a), :key-a)]
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Symbol("a".into()),
        Ast::Keyword("key-a".into()),
    )])));

    let pattern = parse_map_pattern(&interner, &ast, source_id(), 0).unwrap();

    assert_eq!(pattern.explicit.len(), 1);
    let Some((binding, key_ast)) = pattern.explicit.get(0) else {
        panic!("expected explicit binding");
    };

    // Verify the binding is a symbol with the correct name
    let Binding::Symbol(sym_id) = binding else {
        panic!("expected Binding::Symbol");
    };
    let name = interner.resolve(*sym_id);
    assert_eq!(name, "a");

    // Verify the key is a keyword
    assert!(matches!(key_ast.node, Ast::Keyword(ref k) if k == "key-a"));
}

#[test]
fn parse_map_combined() {
    // {:keys [a b] :or {a 0} :as m} -> combined pattern
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![
        (
            Ast::Keyword("keys".into()),
            Ast::Vector(vec![
                spanned(Ast::Symbol("a".into())),
                spanned(Ast::Symbol("b".into())),
            ]),
        ),
        (
            Ast::Keyword("or".into()),
            Ast::Map(map_elements(vec![(
                Ast::Symbol("a".into()),
                Ast::Integer(0),
            )])),
        ),
        (Ast::Keyword("as".into()), Ast::Symbol("m".into())),
    ])));

    let pattern = parse_map_pattern(&interner, &ast, source_id(), 0).unwrap();

    assert_eq!(pattern.keys.len(), 2);
    assert_eq!(pattern.defaults.len(), 1);
    assert!(pattern.as_binding.is_some());
}

#[test]
fn parse_map_empty() {
    // {} -> empty pattern
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(vec![]));

    let pattern = parse_map_pattern(&interner, &ast, source_id(), 0).unwrap();

    assert!(pattern.keys.is_empty());
    assert!(pattern.strs.is_empty());
    assert!(pattern.syms.is_empty());
    assert!(pattern.explicit.is_empty());
    assert!(pattern.defaults.is_empty());
    assert!(pattern.as_binding.is_none());
}

// ==================== Map Pattern Error Cases ====================

#[test]
fn error_map_duplicate_keys() {
    // {:keys [a] :keys [b]} -> error
    let interner = symbol::Interner::new();
    // Build manually to have specific span for second :keys
    let ast = spanned(Ast::Map(vec![
        spanned(Ast::Keyword("keys".into())),
        spanned(Ast::Vector(vec![spanned(Ast::Symbol("a".into()))])),
        spanned_at(Ast::Keyword("keys".into()), 10, 14),
        spanned(Ast::Vector(vec![spanned(Ast::Symbol("b".into()))])),
    ]));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "duplicate :keys in map pattern"
        }
    ));
}

#[test]
fn error_map_duplicate_strs() {
    // {:strs [a] :strs [b]} -> error
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(vec![
        spanned(Ast::Keyword("strs".into())),
        spanned(Ast::Vector(vec![spanned(Ast::Symbol("a".into()))])),
        spanned_at(Ast::Keyword("strs".into()), 10, 14),
        spanned(Ast::Vector(vec![spanned(Ast::Symbol("b".into()))])),
    ]));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "duplicate :strs in map pattern"
        }
    ));
}

#[test]
fn error_map_duplicate_syms() {
    // {:syms [a] :syms [b]} -> error
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(vec![
        spanned(Ast::Keyword("syms".into())),
        spanned(Ast::Vector(vec![spanned(Ast::Symbol("a".into()))])),
        spanned_at(Ast::Keyword("syms".into()), 10, 14),
        spanned(Ast::Vector(vec![spanned(Ast::Symbol("b".into()))])),
    ]));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "duplicate :syms in map pattern"
        }
    ));
}

#[test]
fn error_map_duplicate_or() {
    // {:or {a 0} :or {b 1}} -> error
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(vec![
        spanned(Ast::Keyword("or".into())),
        spanned(Ast::Map(map_elements(vec![(
            Ast::Symbol("a".into()),
            Ast::Integer(0),
        )]))),
        spanned_at(Ast::Keyword("or".into()), 15, 17),
        spanned(Ast::Map(map_elements(vec![(
            Ast::Symbol("b".into()),
            Ast::Integer(1),
        )]))),
    ]));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "duplicate :or in map pattern"
        }
    ));
}

#[test]
fn error_map_duplicate_as() {
    // {:as m :as n} -> error
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(vec![
        spanned(Ast::Keyword("as".into())),
        spanned(Ast::Symbol("m".into())),
        spanned_at(Ast::Keyword("as".into()), 10, 12),
        spanned(Ast::Symbol("n".into())),
    ]));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "duplicate :as in map pattern"
        }
    ));
}

#[test]
fn error_map_keys_not_vector() {
    // {:keys :a} -> error (value must be vector)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("keys".into()),
        Ast::Keyword("a".into()),
    )])));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: ":keys/:strs/:syms value must be a vector"
        }
    ));
}

#[test]
fn error_map_keys_non_symbol() {
    // {:keys [42]} -> error (vector must contain symbols)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("keys".into()),
        Ast::Vector(vec![spanned(Ast::Integer(42))]),
    )])));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: ":keys/:strs/:syms vector must contain only symbols"
        }
    ));
}

#[test]
fn error_map_or_not_map() {
    // {:or [a 0]} -> error (value must be map)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("or".into()),
        Ast::Vector(vec![
            spanned(Ast::Symbol("a".into())),
            spanned(Ast::Integer(0)),
        ]),
    )])));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: ":or value must be a map"
        }
    ));
}

#[test]
fn error_map_or_non_symbol_key() {
    // {:or {42 0}} -> error (keys must be symbols)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("or".into()),
        Ast::Map(map_elements(vec![(Ast::Integer(42), Ast::Integer(0))])),
    )])));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: ":or map keys must be symbols"
        }
    ));
}

#[test]
fn error_map_as_not_symbol() {
    // {:as 42} -> error (value must be symbol)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("as".into()),
        Ast::Integer(42),
    )])));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: ":as must be followed by a symbol"
        }
    ));
}

#[test]
fn error_map_not_map() {
    // [a b] -> error (must be map, not vector)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("b".into())),
    ]));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "expected map pattern"
        }
    ));
}

#[test]
fn error_map_invalid_key() {
    // {42 :foo} -> error (key must be symbol or special keyword)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Integer(42),
        Ast::Keyword("foo".into()),
    )])));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern { message } if message.contains("binding target")
    ));
}

// ==================== Map Pattern Symbol Interning ====================

#[test]
fn map_symbols_are_interned() {
    // Verify that symbols are properly interned
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("keys".into()),
        Ast::Vector(vec![
            spanned(Ast::Symbol("foo".into())),
            spanned(Ast::Symbol("foo".into())), // Same name
        ]),
    )])));

    let pattern = parse_map_pattern(&interner, &ast, source_id(), 0).unwrap();

    // Both should have the same symbol ID
    assert_eq!(pattern.keys.len(), 2);
    let Some(id1) = pattern.keys.get(0) else {
        panic!("expected first key");
    };
    let Some(id2) = pattern.keys.get(1) else {
        panic!("expected second key");
    };
    assert_eq!(id1, id2, "same symbol name should have same ID");
}

#[test]
fn error_map_odd_element_count() {
    // {:a} -> error (map pattern must have even number of elements)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(vec![spanned(Ast::Keyword("a".into()))]));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "map pattern must have even number of elements"
        }
    ));
}

#[test]
fn error_map_or_odd_element_count() {
    // {:or {:a}} -> error (:or map must have even number of elements)
    let interner = symbol::Interner::new();
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("or".into()),
        Ast::Map(vec![spanned(Ast::Symbol("a".into()))]),
    )])));

    let result = parse_map_pattern(&interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: ":or map must have even number of elements"
        }
    ));
}

// ==================== Recursion Depth Limit ====================

#[test]
fn map_recursion_limit_exceeded_returns_error() {
    // Test that exceeding the limit returns an error by starting at a depth
    // close to the limit. We use depth=MAX_PATTERN_DEPTH to ensure the check
    // triggers on the very first nested call.
    let interner = symbol::Interner::new();
    // Create a nested map pattern: {{:keys [x]} :inner}
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Map(map_elements(vec![(
            Ast::Keyword("keys".into()),
            Ast::Vector(vec![spanned(Ast::Symbol("x".into()))]),
        )])),
        Ast::Keyword("inner".into()),
    )])));

    // Start at MAX_PATTERN_DEPTH - the inner map will exceed the limit
    let result = parse_map_pattern(&interner, &ast, source_id(), MAX_PATTERN_DEPTH);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            ErrorKind::RecursionDepthExceeded {
                max_depth: MAX_PATTERN_DEPTH
            }
        ),
        "expected RecursionDepthExceeded error, got {:?}",
        err.kind
    );
}
