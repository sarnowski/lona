// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for `full_span` tracking in the parser.
//!
//! These tests verify that leading comments and whitespace are correctly
//! captured in the `full_span` field of parsed expressions.

extern crate alloc;

use crate::ast::Ast;
use crate::error::{SourceId, Span};
use crate::parser::parse;

/// Test source ID for all parser tests.
const TEST_SOURCE_ID: SourceId = SourceId::new(0_u32);

// ==================== Single Expression Tests ====================

#[test]
fn full_span_includes_leading_comment() {
    let source = "; doc comment\n(foo)";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    // span: just "(foo)"
    assert_eq!(
        exprs.get(0_usize).unwrap().span,
        Span::new(14_usize, 19_usize)
    );
    // full_span: includes "; doc comment\n"
    assert_eq!(
        exprs.get(0_usize).unwrap().full_span,
        Span::new(0_usize, 19_usize)
    );
}

#[test]
fn full_span_no_leading_trivia() {
    let source = "(foo)";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    // No trivia, so full_span equals span
    assert_eq!(
        exprs.get(0_usize).unwrap().span,
        exprs.get(0_usize).unwrap().full_span
    );
}

#[test]
fn full_span_only_whitespace_trivia() {
    let source = "   (foo)";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    // span: just "(foo)"
    assert_eq!(
        exprs.get(0_usize).unwrap().span,
        Span::new(3_usize, 8_usize)
    );
    // full_span: includes leading spaces
    assert_eq!(
        exprs.get(0_usize).unwrap().full_span,
        Span::new(0_usize, 8_usize)
    );
}

// ==================== Multiple Expression Tests ====================

#[test]
fn full_span_multiple_expressions() {
    let source = "; comment 1\n(foo)\n\n; comment 2\n(bar)";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 2_usize);

    // First expression includes its comment
    // "; comment 1\n" is 12 bytes, "(foo)" is positions 12-16, span.end = 17
    let first = exprs.get(0_usize).unwrap();
    assert_eq!(first.full_span.start, 0_usize);
    assert_eq!(first.span, Span::new(12_usize, 17_usize));

    // Second expression includes blank line and its comment
    // Trivia starts right after (foo) at position 17
    // "\n\n; comment 2\n" is 14 bytes, so (bar) starts at position 31
    let second = exprs.get(1_usize).unwrap();
    assert_eq!(second.full_span.start, first.span.end);
    assert_eq!(second.span, Span::new(31_usize, 36_usize));
}

#[test]
fn full_span_multiline_comments() {
    let source = "; Line 1\n; Line 2\n; Line 3\n(foo)";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    // All three comment lines included
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

// ==================== Collection Tests ====================

#[test]
fn full_span_vector() {
    let source = "; vector\n[1 2 3]";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

#[test]
fn full_span_map() {
    let source = "; map\n{:a 1}";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

// ==================== Nested Expression Tests ====================

#[test]
fn full_span_nested_expressions() {
    let source = "; outer\n(list ; inner\n  42)";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);

    // Outer list includes "; outer\n"
    let outer = exprs.get(0_usize).unwrap();
    assert_eq!(outer.full_span.start, 0_usize);

    // Check inner elements have their own full_span
    if let Ast::List(ref elements) = outer.node {
        assert_eq!(elements.len(), 2_usize);

        // First element is "list" symbol (minimal trivia - just space after the open paren)
        let list_sym = elements.get(0_usize).unwrap();
        assert_eq!(list_sym.node, Ast::symbol("list"));

        // Second element (42) has its own comment "; inner\n  "
        let inner = elements.get(1_usize).unwrap();
        assert!(inner.full_span.start < inner.span.start);
    } else {
        panic!("expected list");
    }
}

// ==================== Reader Macro Tests ====================

#[test]
fn full_span_reader_macro_with_comment() {
    let source = "; quoted\n'foo";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);

    // The expanded (quote foo) includes leading comment
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

#[test]
fn full_span_syntax_quote() {
    let source = "; syntax\n`(x y)";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

#[test]
fn full_span_unquote() {
    let source = "; unquoted\n~x";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

#[test]
fn full_span_unquote_splicing() {
    let source = "; spliced\n~@xs";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

// ==================== Atom Tests ====================

#[test]
fn full_span_integer_with_comment() {
    let source = "; number\n42";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

#[test]
fn full_span_string_with_comment() {
    let source = "; greeting\n\"hello\"";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

#[test]
fn full_span_keyword_with_comment() {
    let source = "; key\n:foo";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

#[test]
fn full_span_symbol_with_comment() {
    let source = "; identifier\nfoo";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    assert_eq!(exprs.get(0_usize).unwrap().full_span.start, 0_usize);
}

// ==================== Edge Cases ====================

#[test]
fn full_span_empty_source() {
    let source = "";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 0_usize);
}

#[test]
fn full_span_only_comments() {
    let source = "; just a comment";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 0_usize);
}

#[test]
fn full_span_comma_as_whitespace() {
    let source = ",,,42";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);
    // span: just "42"
    assert_eq!(
        exprs.get(0_usize).unwrap().span,
        Span::new(3_usize, 5_usize)
    );
    // full_span: includes commas
    assert_eq!(
        exprs.get(0_usize).unwrap().full_span,
        Span::new(0_usize, 5_usize)
    );
}

// ==================== Real-World Example Tests ====================

#[test]
fn full_span_function_definition() {
    // Simulates how a defn would look with doc comments
    let source =
        "; Adds two numbers together.\n; Returns the sum of a and b.\n(defn add [a b]\n  (+ a b))";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    assert_eq!(exprs.len(), 1_usize);

    let defn = exprs.get(0_usize).unwrap();

    // full_span should start at the beginning (including both comment lines)
    assert_eq!(defn.full_span.start, 0_usize);

    // span should start at the actual (defn ...)
    let defn_start = source.find("(defn").unwrap();
    assert_eq!(defn.span.start, defn_start);
}

#[test]
fn full_span_preserves_exact_boundaries() {
    let source = "; comment\n(foo)";
    let exprs = parse(source, TEST_SOURCE_ID).unwrap();

    let expr = exprs.get(0_usize).unwrap();

    // full_span should slice to get the full source including comment
    let full_source = source
        .get(expr.full_span.start..expr.full_span.end)
        .unwrap();
    assert_eq!(full_source, "; comment\n(foo)");

    // span should slice to just the expression
    let expr_source = source.get(expr.span.start..expr.span.end).unwrap();
    assert_eq!(expr_source, "(foo)");
}
