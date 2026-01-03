// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Reader integration tests.
//!
//! Tests the Lonala reader (parser) using the test VM infrastructure.
//! Verifies that source strings are correctly parsed into values.

// Test code prioritizes clarity over defensive programming
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use common::{IsBool, IsInt, IsNil, IsString, IsSymbol, TestVm};

// ============================================================================
// Literals: Nil
// ============================================================================

#[test]
fn read_nil() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "nil", IsNil);
}

#[test]
fn read_empty_list_is_nil() {
    let mut vm = TestVm::new();
    // Empty list () parses to nil
    assert_reads_as!(vm, "()", "nil", IsNil);
}

// ============================================================================
// Literals: Booleans
// ============================================================================

#[test]
fn read_true() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "true", IsBool(true));
}

#[test]
fn read_false() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "false", IsBool(false));
}

// ============================================================================
// Literals: Integers
// ============================================================================

#[test]
fn read_zero() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "0", IsInt(0));
}

#[test]
fn read_positive_integer() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "42", IsInt(42));
}

#[test]
fn read_negative_integer() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "-1", IsInt(-1));
    assert_reads!(vm, "-99", IsInt(-99));
}

#[test]
fn read_large_integer() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "123456789", IsInt(123_456_789));
}

// ============================================================================
// Literals: Strings
// ============================================================================

#[test]
fn read_empty_string() {
    let mut vm = TestVm::new();
    assert_reads!(vm, r#""""#, IsString::new(""));
}

#[test]
fn read_simple_string() {
    let mut vm = TestVm::new();
    assert_reads!(vm, r#""hello""#, IsString::new("hello"));
    assert_reads!(vm, r#""hello world""#, IsString::new("hello world"));
}

#[test]
fn read_string_with_escapes() {
    let mut vm = TestVm::new();
    assert_reads!(vm, r#""line1\nline2""#, IsString::new("line1\nline2"));
    assert_reads!(vm, r#""tab\there""#, IsString::new("tab\there"));
    assert_reads!(vm, r#""quote\"here""#, IsString::new("quote\"here"));
    assert_reads!(vm, r#""back\\slash""#, IsString::new("back\\slash"));
}

// ============================================================================
// Literals: Symbols
// ============================================================================

#[test]
fn read_simple_symbol() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "foo", IsSymbol::new("foo"));
    assert_reads!(vm, "hello-world", IsSymbol::new("hello-world"));
}

#[test]
fn read_operator_symbols() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "+", IsSymbol::new("+"));
    assert_reads!(vm, "*", IsSymbol::new("*"));
    assert_reads!(vm, "-", IsSymbol::new("-"));
    assert_reads!(vm, "/", IsSymbol::new("/"));
}

#[test]
fn read_predicate_symbols() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "foo?", IsSymbol::new("foo?"));
    assert_reads!(vm, "nil?", IsSymbol::new("nil?"));
}

#[test]
fn read_mutator_symbols() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "set!", IsSymbol::new("set!"));
}

// ============================================================================
// Lists: Simple
// ============================================================================

#[test]
fn read_singleton_list() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "(1)", list![IsInt(1)]);
}

#[test]
fn read_pair_list() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "(1 2)", list![IsInt(1), IsInt(2)]);
}

#[test]
fn read_triple_list() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "(1 2 3)", list![IsInt(1), IsInt(2), IsInt(3)]);
}

// ============================================================================
// Lists: Nested
// ============================================================================

#[test]
fn read_nested_singleton() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "((1))", list![list![IsInt(1)]]);
}

#[test]
fn read_nested_pairs() {
    let mut vm = TestVm::new();
    assert_reads!(
        vm,
        "((1 2) (3 4))",
        list![list![IsInt(1), IsInt(2)], list![IsInt(3), IsInt(4)]]
    );
}

#[test]
fn read_deeply_nested() {
    let mut vm = TestVm::new();
    assert_reads!(
        vm,
        "(1 (2 3) 4)",
        list![IsInt(1), list![IsInt(2), IsInt(3)], IsInt(4)]
    );
}

// ============================================================================
// Lists: Mixed Types
// ============================================================================

#[test]
fn read_symbol_and_int() {
    let mut vm = TestVm::new();
    assert_reads!(vm, "(foo 42)", list![sym!("foo"), IsInt(42)]);
}

#[test]
fn read_function_call_form() {
    let mut vm = TestVm::new();
    assert_reads!(
        vm,
        "(+ 1 2 3)",
        list![sym!("+"), IsInt(1), IsInt(2), IsInt(3)]
    );
}

#[test]
fn read_if_form() {
    let mut vm = TestVm::new();
    assert_reads!(
        vm,
        "(if true 1 2)",
        list![sym!("if"), IsBool(true), IsInt(1), IsInt(2)]
    );
}

#[test]
fn read_string_in_list() {
    let mut vm = TestVm::new();
    assert_reads!(
        vm,
        r#"(print "hello")"#,
        list![sym!("print"), str!("hello")]
    );
}

// ============================================================================
// Quote Syntax
// ============================================================================

#[test]
fn read_quoted_symbol() {
    let mut vm = TestVm::new();
    // 'foo expands to (quote foo)
    assert_reads_as!(vm, "'foo", "(quote foo)", list![sym!("quote"), sym!("foo")]);
}

#[test]
fn read_quoted_list() {
    let mut vm = TestVm::new();
    // '(1 2 3) expands to (quote (1 2 3))
    assert_reads_as!(
        vm,
        "'(1 2 3)",
        "(quote (1 2 3))",
        list![sym!("quote"), list![IsInt(1), IsInt(2), IsInt(3)]]
    );
}

#[test]
fn read_double_quote() {
    let mut vm = TestVm::new();
    // ''x expands to (quote (quote x))
    assert_reads_as!(
        vm,
        "''x",
        "(quote (quote x))",
        list![sym!("quote"), list![sym!("quote"), sym!("x")]]
    );
}

// ============================================================================
// Whitespace Handling
// ============================================================================

#[test]
fn read_with_leading_whitespace() {
    let mut vm = TestVm::new();
    assert_reads_as!(vm, "  42", "42", IsInt(42));
}

#[test]
fn read_with_trailing_whitespace() {
    let mut vm = TestVm::new();
    assert_reads_as!(vm, "42  ", "42", IsInt(42));
}

#[test]
fn read_list_with_extra_whitespace() {
    let mut vm = TestVm::new();
    assert_reads_as!(
        vm,
        "(  1   2   3  )",
        "(1 2 3)",
        list![IsInt(1), IsInt(2), IsInt(3)]
    );
}

#[test]
fn read_list_with_newlines() {
    let mut vm = TestVm::new();
    assert_reads_as!(vm, "(\n1\n2\n)", "(1 2)", list![IsInt(1), IsInt(2)]);
}

// ============================================================================
// Comments
// ============================================================================

#[test]
fn read_with_trailing_comment() {
    let mut vm = TestVm::new();
    assert_reads_as!(vm, "42 ; this is a comment", "42", IsInt(42));
}

#[test]
fn read_with_leading_comment() {
    let mut vm = TestVm::new();
    assert_reads_as!(vm, "; comment\n42", "42", IsInt(42));
}

#[test]
fn read_list_with_embedded_comment() {
    let mut vm = TestVm::new();
    assert_reads_as!(vm, "(1 ; middle\n 2)", "(1 2)", list![IsInt(1), IsInt(2)]);
}

// ============================================================================
// Error Cases: Unmatched Parentheses
// ============================================================================

#[test]
fn error_unclosed_paren() {
    let mut vm = TestVm::new();
    assert_read_error!(vm, "(");
}

#[test]
fn error_double_unclosed_paren() {
    let mut vm = TestVm::new();
    assert_read_error!(vm, "((");
}

#[test]
fn error_unclosed_list() {
    let mut vm = TestVm::new();
    assert_read_error!(vm, "(1 2");
}

#[test]
fn error_extra_close_paren() {
    let mut vm = TestVm::new();
    assert_read_error!(vm, ")");
}

// ============================================================================
// Error Cases: Unterminated Strings
// ============================================================================

#[test]
fn error_unterminated_empty_string() {
    let mut vm = TestVm::new();
    assert_read_error!(vm, r#"""#);
}

#[test]
fn error_unterminated_string() {
    let mut vm = TestVm::new();
    assert_read_error!(vm, r#""hello"#);
}
