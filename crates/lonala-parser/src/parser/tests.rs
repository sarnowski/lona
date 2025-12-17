// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Lonala parser.

extern crate alloc;

use alloc::format;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use crate::ast::Ast;
use crate::error::{Kind as ErrorKind, Span};
use crate::parser::{parse, parse_one};

/// Helper to parse and return the AST node, ignoring spans.
fn parse_ast(source: &str) -> Ast {
    parse_one(source).expect("parse should succeed").node
}

/// Helper to parse and return all AST nodes.
fn parse_asts(source: &str) -> Vec<Ast> {
    parse(source)
        .expect("parse should succeed")
        .into_iter()
        .map(|spanned| spanned.node)
        .collect()
}

// ==================== Atoms: Integers ====================

#[test]
fn parse_integer_decimal() {
    assert_eq!(parse_ast("42"), Ast::Integer(42_i64));
    assert_eq!(parse_ast("0"), Ast::Integer(0_i64));
    assert_eq!(parse_ast("123456789"), Ast::Integer(123456789_i64));
}

#[test]
fn parse_integer_negative() {
    assert_eq!(parse_ast("-42"), Ast::Integer(-42_i64));
    assert_eq!(parse_ast("-1"), Ast::Integer(-1_i64));
}

#[test]
fn parse_integer_hex() {
    assert_eq!(parse_ast("0xFF"), Ast::Integer(255_i64));
    assert_eq!(parse_ast("0x1a2B"), Ast::Integer(0x1a2B_i64));
    assert_eq!(parse_ast("0X10"), Ast::Integer(16_i64));
}

#[test]
fn parse_integer_binary() {
    assert_eq!(parse_ast("0b1010"), Ast::Integer(10_i64));
    assert_eq!(parse_ast("0B11"), Ast::Integer(3_i64));
}

#[test]
fn parse_integer_octal() {
    assert_eq!(parse_ast("0o755"), Ast::Integer(493_i64));
    assert_eq!(parse_ast("0O17"), Ast::Integer(15_i64));
}

// ==================== Atoms: Floats ====================

#[test]
fn parse_float_simple() {
    assert_eq!(parse_ast("3.14"), Ast::Float(3.14_f64));
    assert_eq!(parse_ast("0.5"), Ast::Float(0.5_f64));
}

#[test]
fn parse_float_negative() {
    assert_eq!(parse_ast("-3.14"), Ast::Float(-3.14_f64));
}

#[test]
fn parse_float_scientific() {
    assert_eq!(parse_ast("1e10"), Ast::Float(1e10_f64));
    assert_eq!(parse_ast("2.5e-3"), Ast::Float(2.5e-3_f64));
    assert_eq!(parse_ast("1E+5"), Ast::Float(1e5_f64));
}

#[test]
fn parse_float_nan() {
    let ast = parse_ast("##NaN");
    if let Ast::Float(float_val) = ast {
        assert!(float_val.is_nan());
    } else {
        panic!("expected Float");
    }
}

#[test]
fn parse_float_infinity() {
    assert_eq!(parse_ast("##Inf"), Ast::Float(f64::INFINITY));
    assert_eq!(parse_ast("##-Inf"), Ast::Float(f64::NEG_INFINITY));
}

// ==================== Atoms: Strings ====================

#[test]
fn parse_string_empty() {
    assert_eq!(
        parse_ast(r#""""#),
        Ast::String(alloc::string::String::new())
    );
}

#[test]
fn parse_string_simple() {
    assert_eq!(parse_ast(r#""hello""#), Ast::String("hello".to_string()));
}

#[test]
fn parse_string_with_escapes() {
    assert_eq!(
        parse_ast(r#""hello\nworld""#),
        Ast::String("hello\nworld".to_string())
    );
    assert_eq!(
        parse_ast(r#""tab\there""#),
        Ast::String("tab\there".to_string())
    );
    assert_eq!(
        parse_ast(r#""back\\slash""#),
        Ast::String("back\\slash".to_string())
    );
    assert_eq!(
        parse_ast(r#""say \"hi\"""#),
        Ast::String("say \"hi\"".to_string())
    );
    assert_eq!(
        parse_ast(r#""return\r""#),
        Ast::String("return\r".to_string())
    );
    assert_eq!(parse_ast(r#""null\0""#), Ast::String("null\0".to_string()));
}

#[test]
fn parse_string_unicode_escape() {
    assert_eq!(parse_ast(r#""\u0041""#), Ast::String("A".to_string()));
    assert_eq!(
        parse_ast(r#""\u03B1""#),
        Ast::String("\u{03B1}".to_string())
    ); // Greek alpha
}

// ==================== Atoms: Booleans and Nil ====================

#[test]
fn parse_boolean_true() {
    assert_eq!(parse_ast("true"), Ast::Bool(true));
}

#[test]
fn parse_boolean_false() {
    assert_eq!(parse_ast("false"), Ast::Bool(false));
}

#[test]
fn parse_nil() {
    assert_eq!(parse_ast("nil"), Ast::Nil);
}

// ==================== Atoms: Symbols ====================

#[test]
fn parse_symbol_simple() {
    assert_eq!(parse_ast("foo"), Ast::Symbol("foo".to_string()));
    assert_eq!(parse_ast("bar"), Ast::Symbol("bar".to_string()));
}

#[test]
fn parse_symbol_operators() {
    assert_eq!(parse_ast("+"), Ast::Symbol("+".to_string()));
    assert_eq!(parse_ast("-"), Ast::Symbol("-".to_string()));
    assert_eq!(parse_ast("*"), Ast::Symbol("*".to_string()));
    assert_eq!(parse_ast("/"), Ast::Symbol("/".to_string()));
    assert_eq!(parse_ast("<="), Ast::Symbol("<=".to_string()));
    assert_eq!(parse_ast(">="), Ast::Symbol(">=".to_string()));
}

#[test]
fn parse_symbol_with_special_chars() {
    assert_eq!(parse_ast("update!"), Ast::Symbol("update!".to_string()));
    assert_eq!(parse_ast("empty?"), Ast::Symbol("empty?".to_string()));
    assert_eq!(parse_ast("->arrow"), Ast::Symbol("->arrow".to_string()));
    assert_eq!(parse_ast("*special*"), Ast::Symbol("*special*".to_string()));
}

#[test]
fn parse_symbol_namespaced() {
    assert_eq!(parse_ast("ns/name"), Ast::Symbol("ns/name".to_string()));
    assert_eq!(
        parse_ast("foo.bar/baz"),
        Ast::Symbol("foo.bar/baz".to_string())
    );
}

// ==================== Atoms: Keywords ====================

#[test]
fn parse_keyword_simple() {
    assert_eq!(parse_ast(":foo"), Ast::Keyword("foo".to_string()));
    assert_eq!(parse_ast(":bar"), Ast::Keyword("bar".to_string()));
}

#[test]
fn parse_keyword_namespaced() {
    assert_eq!(parse_ast(":ns/name"), Ast::Keyword("ns/name".to_string()));
}

#[test]
fn parse_keyword_kebab_case() {
    assert_eq!(
        parse_ast(":kebab-case"),
        Ast::Keyword("kebab-case".to_string())
    );
}

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

// ==================== Multiple Expressions ====================

#[test]
fn parse_multiple_expressions() {
    let asts = parse_asts("1 2 3");
    assert_eq!(asts.len(), 3_usize);
    assert_eq!(asts.first(), Some(&Ast::Integer(1_i64)));
    assert_eq!(asts.get(1_usize), Some(&Ast::Integer(2_i64)));
    assert_eq!(asts.get(2_usize), Some(&Ast::Integer(3_i64)));
}

#[test]
fn parse_empty_source() {
    let asts = parse_asts("");
    assert!(asts.is_empty());
}

#[test]
fn parse_whitespace_only() {
    let asts = parse_asts("   \n\t  ");
    assert!(asts.is_empty());
}

// ==================== Span Tracking ====================

#[test]
fn span_single_token() {
    let spanned = parse_one("foo").expect("parse should succeed");
    assert_eq!(spanned.span, Span::new(0_usize, 3_usize));
}

#[test]
fn span_collection() {
    let spanned = parse_one("(+ 1 2)").expect("parse should succeed");
    assert_eq!(spanned.span, Span::new(0_usize, 7_usize));
}

#[test]
fn span_nested() {
    let spanned = parse_one("((a))").expect("parse should succeed");
    assert_eq!(spanned.span, Span::new(0_usize, 5_usize));
}

#[test]
fn span_reader_macro() {
    let spanned = parse_one("'x").expect("parse should succeed");
    // Spans from quote (0) to end of x (2)
    assert_eq!(spanned.span, Span::new(0_usize, 2_usize));
}

// ==================== Error Cases ====================

#[test]
fn error_unexpected_eof_in_list() {
    let result = parse_one("(+ 1");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ErrorKind::UnexpectedEof { .. }));
}

#[test]
fn error_unexpected_eof_in_vector() {
    let result = parse_one("[1 2");
    assert!(result.is_err());
}

#[test]
fn error_unexpected_eof_in_map() {
    let result = parse_one("{:a 1");
    assert!(result.is_err());
}

#[test]
fn error_mismatched_delimiter() {
    let result = parse_one("(]");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ErrorKind::UnmatchedDelimiter { .. }));
}

#[test]
fn error_odd_map_entries() {
    let result = parse_one("{:a 1 :b}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::OddMapEntries);
}

#[test]
fn error_reader_macro_at_eof() {
    let result = parse_one("'");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::ReaderMacroMissingExpr);
}

#[test]
fn error_reader_macro_before_closer() {
    let result = parse_one("(')");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::ReaderMacroMissingExpr);
}

#[test]
fn error_unexpected_closing_delimiter() {
    let result = parse_one(")");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ErrorKind::UnexpectedToken { .. }));
}

#[test]
fn error_parse_one_empty() {
    let result = parse_one("");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ErrorKind::UnexpectedEof { .. }));
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

// ==================== Display Round-trip ====================

#[test]
fn display_roundtrip_simple() {
    let ast = parse_ast("42");
    assert_eq!(format!("{ast}"), "42");
}

#[test]
fn display_roundtrip_list() {
    let ast = parse_ast("(+ 1 2)");
    assert_eq!(format!("{ast}"), "(+ 1 2)");
}

#[test]
fn display_roundtrip_vector() {
    let ast = parse_ast("[1 2 3]");
    assert_eq!(format!("{ast}"), "[1 2 3]");
}

#[test]
fn display_roundtrip_map() {
    let ast = parse_ast("{:a 1}");
    assert_eq!(format!("{ast}"), "{:a 1}");
}
