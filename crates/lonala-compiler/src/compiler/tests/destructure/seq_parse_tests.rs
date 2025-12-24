// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for sequential destructuring pattern parsing.

use alloc::vec;

use lona_core::symbol;
use lonala_parser::Ast;

use super::{source_id, spanned, spanned_at};
use crate::compiler::destructure::{Binding, MAX_PATTERN_DEPTH, parse_sequential_pattern};
use crate::error::Kind as ErrorKind;

// ==================== Basic Pattern Parsing ====================

#[test]
fn parse_simple_symbols() {
    // [a b c] -> 3 symbol bindings
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("b".into())),
        spanned(Ast::Symbol("c".into())),
    ]));

    let pattern = parse_sequential_pattern(&mut interner, &ast, source_id(), 0).unwrap();

    assert_eq!(pattern.items.len(), 3);
    assert!(pattern.rest.is_none());
    assert!(pattern.as_binding.is_none());

    // Verify each binding is a symbol
    for binding in &pattern.items {
        assert!(matches!(binding, Binding::Symbol(_)));
    }
}

#[test]
fn parse_rest_binding() {
    // [a & rest] -> 1 symbol + rest binding
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("&".into())),
        spanned(Ast::Symbol("rest".into())),
    ]));

    let pattern = parse_sequential_pattern(&mut interner, &ast, source_id(), 0).unwrap();

    assert_eq!(pattern.items.len(), 1);
    let Some(ref rest) = pattern.rest else {
        panic!("expected rest binding");
    };
    assert!(matches!(**rest, Binding::Symbol(_)));
}

#[test]
fn parse_ignore_binding() {
    // [a _ c] -> symbol, ignore, symbol
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("_".into())),
        spanned(Ast::Symbol("c".into())),
    ]));

    let pattern = parse_sequential_pattern(&mut interner, &ast, source_id(), 0).unwrap();

    assert_eq!(pattern.items.len(), 3);
    let Some(item0) = pattern.items.get(0) else {
        panic!("expected item at index 0");
    };
    let Some(item1) = pattern.items.get(1) else {
        panic!("expected item at index 1");
    };
    let Some(item2) = pattern.items.get(2) else {
        panic!("expected item at index 2");
    };
    assert!(matches!(item0, Binding::Symbol(_)));
    assert!(matches!(item1, Binding::Ignore));
    assert!(matches!(item2, Binding::Symbol(_)));
}

#[test]
fn parse_as_binding() {
    // [a :as all] -> 1 symbol + as binding
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Keyword("as".into())),
        spanned(Ast::Symbol("all".into())),
    ]));

    let pattern = parse_sequential_pattern(&mut interner, &ast, source_id(), 0).unwrap();

    assert_eq!(pattern.items.len(), 1);
    assert!(pattern.as_binding.is_some());
}

#[test]
fn parse_nested_pattern() {
    // [[a b] c] -> nested pattern + symbol
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Vector(vec![
            spanned(Ast::Symbol("a".into())),
            spanned(Ast::Symbol("b".into())),
        ])),
        spanned(Ast::Symbol("c".into())),
    ]));

    let pattern = parse_sequential_pattern(&mut interner, &ast, source_id(), 0)
        .expect("pattern should parse");

    assert_eq!(pattern.items.len(), 2);
    let Some(item0) = pattern.items.get(0) else {
        panic!("expected item at index 0");
    };
    let Some(item1) = pattern.items.get(1) else {
        panic!("expected item at index 1");
    };
    assert!(matches!(item0, Binding::Seq(_)));
    assert!(matches!(item1, Binding::Symbol(_)));

    // Verify nested pattern
    let Binding::Seq(nested) = item0 else {
        panic!("expected Seq at index 0");
    };
    assert_eq!(nested.items.len(), 2);
}

#[test]
fn parse_rest_with_as() {
    // [a & r :as all] -> 1 symbol + rest + as binding
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("&".into())),
        spanned(Ast::Symbol("r".into())),
        spanned(Ast::Keyword("as".into())),
        spanned(Ast::Symbol("all".into())),
    ]));

    let pattern = parse_sequential_pattern(&mut interner, &ast, source_id(), 0).unwrap();

    assert_eq!(pattern.items.len(), 1);
    assert!(pattern.rest.is_some());
    assert!(pattern.as_binding.is_some());
}

#[test]
fn parse_empty_pattern() {
    // [] -> empty pattern
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![]));

    let pattern = parse_sequential_pattern(&mut interner, &ast, source_id(), 0).unwrap();

    assert!(pattern.items.is_empty());
    assert!(pattern.rest.is_none());
    assert!(pattern.as_binding.is_none());
}

// ==================== Error Cases ====================

#[test]
fn error_duplicate_ampersand() {
    // [& a & b] -> error
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("&".into())),
        spanned(Ast::Symbol("a".into())),
        spanned_at(Ast::Symbol("&".into()), 5, 6),
        spanned(Ast::Symbol("b".into())),
    ]));

    let result = parse_sequential_pattern(&mut interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "duplicate & in pattern"
        }
    ));
}

#[test]
fn error_duplicate_as() {
    // [a :as x :as y] -> error
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Keyword("as".into())),
        spanned(Ast::Symbol("x".into())),
        spanned_at(Ast::Keyword("as".into()), 10, 12),
        spanned(Ast::Symbol("y".into())),
    ]));

    let result = parse_sequential_pattern(&mut interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "duplicate :as in pattern"
        }
    ));
}

#[test]
fn error_as_without_symbol() {
    // [:as] -> error
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![spanned(Ast::Keyword("as".into()))]));

    let result = parse_sequential_pattern(&mut interner, &ast, source_id(), 0);
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
fn error_as_followed_by_non_symbol() {
    // [:as 42] -> error
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Keyword("as".into())),
        spanned(Ast::Integer(42)),
    ]));

    let result = parse_sequential_pattern(&mut interner, &ast, source_id(), 0);
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
fn error_ampersand_without_binding() {
    // [a &] -> error
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("&".into())),
    ]));

    let result = parse_sequential_pattern(&mut interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "& must be followed by a binding"
        }
    ));
}

#[test]
fn error_not_vector() {
    // (a b c) -> error (list, not vector)
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::List(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("b".into())),
        spanned(Ast::Symbol("c".into())),
    ]));

    let result = parse_sequential_pattern(&mut interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "expected vector pattern"
        }
    ));
}

#[test]
fn error_invalid_binding() {
    // [a 42 c] -> error (integer not valid)
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Integer(42)),
        spanned(Ast::Symbol("c".into())),
    ]));

    let result = parse_sequential_pattern(&mut interner, &ast, source_id(), 0);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ErrorKind::InvalidDestructuringPattern {
            message: "binding must be a symbol, _, vector, or map pattern"
        }
    ));
}

// ==================== Symbol ID Verification ====================

#[test]
fn symbols_are_interned() {
    // Verify that symbols are properly interned
    let mut interner = symbol::Interner::new();
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("foo".into())),
        spanned(Ast::Symbol("foo".into())), // Same name
    ]));

    let pattern = parse_sequential_pattern(&mut interner, &ast, source_id(), 0)
        .expect("pattern should parse");

    // Both should be Symbol bindings with the same ID
    let Some(item0) = pattern.items.get(0) else {
        panic!("expected item at index 0");
    };
    let Some(item1) = pattern.items.get(1) else {
        panic!("expected item at index 1");
    };
    let (Binding::Symbol(id1), Binding::Symbol(id2)) = (item0, item1) else {
        panic!("expected both items to be Symbol bindings");
    };
    assert_eq!(id1, id2, "same symbol name should have same ID");
}

// ==================== Recursion Depth Limit ====================

/// Helper to generate a deeply nested vector pattern.
///
/// Creates a pattern like `[[[[...x...]...]...]` with `depth` levels of nesting.
fn generate_nested_vectors(depth: usize) -> lonala_parser::Spanned<Ast> {
    let mut result = spanned(Ast::Symbol("x".into()));

    for _ in 0..depth {
        result = spanned(Ast::Vector(vec![result]));
    }

    result
}

#[test]
fn deep_nesting_within_limit_succeeds() {
    // 500 levels is well within the 1024 limit
    let mut interner = symbol::Interner::new();
    let deep = generate_nested_vectors(500_usize);

    let result = parse_sequential_pattern(&mut interner, &deep, source_id(), 0);
    assert!(
        result.is_ok(),
        "500 levels of nesting should succeed (limit is {MAX_PATTERN_DEPTH})"
    );
}

#[test]
fn recursion_limit_exceeded_returns_error() {
    // Test that exceeding the limit returns an error by starting at a depth
    // close to the limit. We use depth=MAX_PATTERN_DEPTH to ensure the check
    // triggers on the very first nested call.
    let mut interner = symbol::Interner::new();
    // Create just 2 levels of nesting, but start at depth MAX_PATTERN_DEPTH
    let ast = spanned(Ast::Vector(vec![spanned(Ast::Vector(vec![spanned(
        Ast::Symbol("x".into()),
    )]))]));

    // Start at MAX_PATTERN_DEPTH - the inner vector will exceed the limit
    let result = parse_sequential_pattern(&mut interner, &ast, source_id(), MAX_PATTERN_DEPTH);
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
