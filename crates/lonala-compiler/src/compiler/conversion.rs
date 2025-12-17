// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Conversion between AST and runtime Values.
//!
//! This module provides bidirectional conversion between parsed AST nodes
//! and runtime `Value` types. This is essential for macro expansion:
//!
//! 1. Macro arguments (AST) are converted to Values before calling the macro
//! 2. Macro results (Values) are converted back to AST for further compilation

use alloc::string::String;
use alloc::vec::Vec;

use lona_core::integer::Integer;
use lona_core::list::List;
use lona_core::map::Map;
use lona_core::span::Span;
use lona_core::string::HeapStr;
use lona_core::symbol;
use lona_core::value::Value;
use lona_core::vector::Vector;
use lonala_parser::{Ast, Spanned};

use crate::error::Error;

/// Converts an AST node to a runtime Value.
///
/// Used to pass macro arguments to macro transformers. The AST is converted
/// to data values that the macro can inspect and manipulate.
///
/// # Symbol Handling
///
/// Symbols and keywords are interned using the provided interner.
/// Keywords are stored as symbols with a `:` prefix to distinguish them.
#[inline]
pub fn ast_to_value(ast: &Spanned<Ast>, interner: &mut symbol::Interner) -> Value {
    match ast.node {
        Ast::Bool(bool_val) => Value::Bool(bool_val),
        Ast::Integer(num) => Value::Integer(Integer::from_i64(num)),
        Ast::Float(num) => Value::Float(num),
        Ast::String(ref text) => Value::String(HeapStr::new(text)),
        Ast::Symbol(ref name) => {
            let id = interner.intern(name);
            Value::Symbol(id)
        }
        Ast::Keyword(ref name) => {
            // Keywords are stored as symbols with `:` prefix
            let keyword_name = alloc::format!(":{name}");
            let id = interner.intern(&keyword_name);
            Value::Symbol(id)
        }
        Ast::List(ref elements) => {
            let values: Vec<Value> = elements
                .iter()
                .map(|elem| ast_to_value(elem, interner))
                .collect();
            Value::List(List::from_vec(values))
        }
        Ast::Vector(ref elements) => {
            let values: Vec<Value> = elements
                .iter()
                .map(|elem| ast_to_value(elem, interner))
                .collect();
            Value::Vector(Vector::from_vec(values))
        }
        Ast::Map(ref elements) => {
            // Map elements come as a flat list [k1 v1 k2 v2 ...]
            // Convert to pairs
            let values: Vec<Value> = elements
                .iter()
                .map(|elem| ast_to_value(elem, interner))
                .collect();

            // Group into pairs
            let pairs: Vec<(Value, Value)> = values
                .chunks_exact(2_usize)
                .map(|chunk| {
                    // chunks_exact(2) guarantees exactly 2 elements per chunk
                    let key = chunk.first().cloned().unwrap_or(Value::Nil);
                    let val = chunk.get(1_usize).cloned().unwrap_or(Value::Nil);
                    (key, val)
                })
                .collect();

            Value::Map(Map::from_pairs(pairs))
        }
        // Ast::Nil and any future variants (non-exhaustive enum) become Value::Nil
        Ast::Nil | _ => Value::Nil,
    }
}

/// Converts a runtime Value back to an AST node.
///
/// Used to convert macro expansion results back to AST for further compilation.
/// The span parameter is used for error reporting on the converted AST.
///
/// # Errors
///
/// Returns an error if the Value cannot be converted to AST:
/// - Functions cannot be represented as AST
/// - Ratios are not supported in AST (future enhancement)
#[inline]
pub fn value_to_ast(
    value: &Value,
    interner: &symbol::Interner,
    span: Span,
) -> Result<Spanned<Ast>, Error> {
    let ast = match *value {
        Value::Nil => Ast::Nil,
        Value::Bool(bool_val) => Ast::Bool(bool_val),
        Value::Integer(ref int_val) => {
            // Convert arbitrary-precision integer to i64
            // If it doesn't fit, we have a problem - but for now most macros
            // work with small integers
            let i64_val = int_val.to_i64().ok_or_else(|| Error::InvalidMacroResult {
                message: String::from("integer too large for AST representation"),
                span,
            })?;
            Ast::Integer(i64_val)
        }
        Value::Float(num) => Ast::Float(num),
        Value::Ratio(ref _ratio) => {
            // Ratios don't have direct AST representation
            // Convert to a list form: (/ numerator denominator)
            return Err(Error::InvalidMacroResult {
                message: String::from("ratio values cannot be converted to AST"),
                span,
            });
        }
        Value::Symbol(id) => {
            let name = interner.resolve(id);
            // Check if it's a keyword (starts with ':')
            name.strip_prefix(':').map_or_else(
                || Ast::Symbol(String::from(name)),
                |stripped| Ast::Keyword(String::from(stripped)),
            )
        }
        Value::String(ref text) => Ast::String(String::from(text.as_str())),
        Value::List(ref list) => {
            let elements: Result<Vec<Spanned<Ast>>, Error> = list
                .iter()
                .map(|elem| value_to_ast(elem, interner, span))
                .collect();
            Ast::List(elements?)
        }
        Value::Vector(ref vector) => {
            let elements: Result<Vec<Spanned<Ast>>, Error> = vector
                .iter()
                .map(|elem| value_to_ast(elem, interner, span))
                .collect();
            Ast::Vector(elements?)
        }
        Value::Map(ref map) => {
            // Convert map back to flat list [k1 v1 k2 v2 ...]
            let mut elements = Vec::new();
            for (key, value) in map.iter() {
                elements.push(value_to_ast(key.value(), interner, span)?);
                elements.push(value_to_ast(value, interner, span)?);
            }
            Ast::Map(elements)
        }
        Value::Function(ref _func) => {
            return Err(Error::InvalidMacroResult {
                message: String::from("function values cannot be converted to AST"),
                span,
            });
        }
        // Handle future Value variants
        _ => {
            return Err(Error::InvalidMacroResult {
                message: String::from("unknown value type cannot be converted to AST"),
                span,
            });
        }
    };

    Ok(Spanned::new(ast, span))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// Helper to create a span for testing.
    fn test_span() -> Span {
        Span::new(0_usize, 10_usize)
    }

    /// Helper to wrap AST in Spanned.
    fn spanned(ast: Ast) -> Spanned<Ast> {
        Spanned::new(ast, test_span())
    }

    // =========================================================================
    // ast_to_value tests
    // =========================================================================

    #[test]
    fn ast_to_value_nil() {
        let mut interner = symbol::Interner::new();
        let ast = spanned(Ast::Nil);
        let value = ast_to_value(&ast, &mut interner);
        assert_eq!(value, Value::Nil);
    }

    #[test]
    fn ast_to_value_bool() {
        let mut interner = symbol::Interner::new();

        let ast_true = spanned(Ast::Bool(true));
        assert_eq!(ast_to_value(&ast_true, &mut interner), Value::Bool(true));

        let ast_false = spanned(Ast::Bool(false));
        assert_eq!(ast_to_value(&ast_false, &mut interner), Value::Bool(false));
    }

    #[test]
    fn ast_to_value_integer() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::Integer(42_i64));
        let value = ast_to_value(&ast, &mut interner);

        if let Value::Integer(int_val) = value {
            assert_eq!(int_val.to_i64(), Some(42_i64));
        } else {
            panic!("Expected Integer");
        }
    }

    #[test]
    fn ast_to_value_float() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::Float(3.14_f64));
        let value = ast_to_value(&ast, &mut interner);

        assert_eq!(value, Value::Float(3.14_f64));
    }

    #[test]
    fn ast_to_value_string() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::String(String::from("hello")));
        let value = ast_to_value(&ast, &mut interner);

        if let Value::String(text) = value {
            assert_eq!(text.as_str(), "hello");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn ast_to_value_symbol() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::Symbol(String::from("foo")));
        let value = ast_to_value(&ast, &mut interner);

        if let Value::Symbol(id) = value {
            assert_eq!(interner.resolve(id), "foo");
        } else {
            panic!("Expected Symbol");
        }
    }

    #[test]
    fn ast_to_value_keyword() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::Keyword(String::from("key")));
        let value = ast_to_value(&ast, &mut interner);

        if let Value::Symbol(id) = value {
            // Keywords are stored with ':' prefix
            assert_eq!(interner.resolve(id), ":key");
        } else {
            panic!("Expected Symbol");
        }
    }

    #[test]
    fn ast_to_value_list() {
        let mut interner = symbol::Interner::new();

        let elements = vec![
            spanned(Ast::Integer(1_i64)),
            spanned(Ast::Integer(2_i64)),
            spanned(Ast::Integer(3_i64)),
        ];
        let ast = spanned(Ast::List(elements));
        let value = ast_to_value(&ast, &mut interner);

        if let Value::List(list) = value {
            assert_eq!(list.len(), 3_usize);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn ast_to_value_vector() {
        let mut interner = symbol::Interner::new();

        let elements = vec![spanned(Ast::Integer(1_i64)), spanned(Ast::Integer(2_i64))];
        let ast = spanned(Ast::Vector(elements));
        let value = ast_to_value(&ast, &mut interner);

        if let Value::Vector(vec) = value {
            assert_eq!(vec.len(), 2_usize);
        } else {
            panic!("Expected Vector");
        }
    }

    #[test]
    fn ast_to_value_map() {
        let mut interner = symbol::Interner::new();

        // Map elements are flat: [k1 v1 k2 v2]
        let elements = vec![
            spanned(Ast::Keyword(String::from("a"))),
            spanned(Ast::Integer(1_i64)),
            spanned(Ast::Keyword(String::from("b"))),
            spanned(Ast::Integer(2_i64)),
        ];
        let ast = spanned(Ast::Map(elements));
        let value = ast_to_value(&ast, &mut interner);

        if let Value::Map(map) = value {
            assert_eq!(map.len(), 2_usize);
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn ast_to_value_nested_list() {
        let mut interner = symbol::Interner::new();

        let inner = vec![
            spanned(Ast::Symbol(String::from("+"))),
            spanned(Ast::Integer(1_i64)),
            spanned(Ast::Integer(2_i64)),
        ];
        let outer = vec![
            spanned(Ast::Symbol(String::from("do"))),
            spanned(Ast::List(inner)),
        ];
        let ast = spanned(Ast::List(outer));
        let value = ast_to_value(&ast, &mut interner);

        if let Value::List(list) = value {
            assert_eq!(list.len(), 2_usize);
            // Second element should be a nested list
            if let Some(Value::List(inner_list)) = list.iter().nth(1_usize) {
                assert_eq!(inner_list.len(), 3_usize);
            } else {
                panic!("Expected nested List");
            }
        } else {
            panic!("Expected List");
        }
    }

    // =========================================================================
    // value_to_ast tests
    // =========================================================================

    #[test]
    fn value_to_ast_nil() {
        let interner = symbol::Interner::new();
        let result = value_to_ast(&Value::Nil, &interner, test_span()).unwrap();
        assert_eq!(result.node, Ast::Nil);
    }

    #[test]
    fn value_to_ast_bool() {
        let interner = symbol::Interner::new();

        let result = value_to_ast(&Value::Bool(true), &interner, test_span()).unwrap();
        assert_eq!(result.node, Ast::Bool(true));

        let result = value_to_ast(&Value::Bool(false), &interner, test_span()).unwrap();
        assert_eq!(result.node, Ast::Bool(false));
    }

    #[test]
    fn value_to_ast_integer() {
        let interner = symbol::Interner::new();

        let value = Value::Integer(Integer::from_i64(42_i64));
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        assert_eq!(result.node, Ast::Integer(42_i64));
    }

    #[test]
    fn value_to_ast_float() {
        let interner = symbol::Interner::new();

        let value = Value::Float(3.14_f64);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        assert_eq!(result.node, Ast::Float(3.14_f64));
    }

    #[test]
    fn value_to_ast_string() {
        let interner = symbol::Interner::new();

        let value = Value::String(HeapStr::new("hello"));
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        assert_eq!(result.node, Ast::String(String::from("hello")));
    }

    #[test]
    fn value_to_ast_symbol() {
        let mut interner = symbol::Interner::new();
        let id = interner.intern("foo");

        let value = Value::Symbol(id);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        assert_eq!(result.node, Ast::Symbol(String::from("foo")));
    }

    #[test]
    fn value_to_ast_keyword() {
        let mut interner = symbol::Interner::new();
        // Keywords are stored with ':' prefix
        let id = interner.intern(":key");

        let value = Value::Symbol(id);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        assert_eq!(result.node, Ast::Keyword(String::from("key")));
    }

    #[test]
    fn value_to_ast_list() {
        let interner = symbol::Interner::new();

        let list = List::from_vec(vec![
            Value::Integer(Integer::from_i64(1_i64)),
            Value::Integer(Integer::from_i64(2_i64)),
        ]);
        let value = Value::List(list);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        if let Ast::List(elements) = result.node {
            assert_eq!(elements.len(), 2_usize);
            assert_eq!(
                elements.first().map(|e| &e.node),
                Some(&Ast::Integer(1_i64))
            );
        } else {
            panic!("Expected List AST");
        }
    }

    #[test]
    fn value_to_ast_vector() {
        let interner = symbol::Interner::new();

        let vector = Vector::from_vec(vec![
            Value::Integer(Integer::from_i64(1_i64)),
            Value::Integer(Integer::from_i64(2_i64)),
        ]);
        let value = Value::Vector(vector);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        if let Ast::Vector(elements) = result.node {
            assert_eq!(elements.len(), 2_usize);
        } else {
            panic!("Expected Vector AST");
        }
    }

    #[test]
    fn value_to_ast_map() {
        let mut interner = symbol::Interner::new();
        let key_id = interner.intern(":a");

        let map = Map::from_pairs(vec![(
            Value::Symbol(key_id),
            Value::Integer(Integer::from_i64(1_i64)),
        )]);
        let value = Value::Map(map);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        if let Ast::Map(elements) = result.node {
            // Map has 1 entry = 2 elements (key + value)
            assert_eq!(elements.len(), 2_usize);
        } else {
            panic!("Expected Map AST");
        }
    }

    // =========================================================================
    // Round-trip tests
    // =========================================================================

    #[test]
    fn roundtrip_nil() {
        let mut interner = symbol::Interner::new();
        let ast = spanned(Ast::Nil);
        let value = ast_to_value(&ast, &mut interner);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();
        assert_eq!(result.node, ast.node);
    }

    #[test]
    fn roundtrip_bool() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::Bool(true));
        let value = ast_to_value(&ast, &mut interner);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();
        assert_eq!(result.node, ast.node);
    }

    #[test]
    fn roundtrip_integer() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::Integer(42_i64));
        let value = ast_to_value(&ast, &mut interner);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();
        assert_eq!(result.node, ast.node);
    }

    #[test]
    fn roundtrip_float() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::Float(3.14_f64));
        let value = ast_to_value(&ast, &mut interner);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();
        assert_eq!(result.node, ast.node);
    }

    #[test]
    fn roundtrip_string() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::String(String::from("hello")));
        let value = ast_to_value(&ast, &mut interner);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();
        assert_eq!(result.node, ast.node);
    }

    #[test]
    fn roundtrip_symbol() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::Symbol(String::from("foo")));
        let value = ast_to_value(&ast, &mut interner);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();
        assert_eq!(result.node, ast.node);
    }

    #[test]
    fn roundtrip_keyword() {
        let mut interner = symbol::Interner::new();

        let ast = spanned(Ast::Keyword(String::from("key")));
        let value = ast_to_value(&ast, &mut interner);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();
        assert_eq!(result.node, ast.node);
    }

    #[test]
    fn roundtrip_list() {
        let mut interner = symbol::Interner::new();

        let elements = vec![
            spanned(Ast::Symbol(String::from("+"))),
            spanned(Ast::Integer(1_i64)),
            spanned(Ast::Integer(2_i64)),
        ];
        let ast = spanned(Ast::List(elements));
        let value = ast_to_value(&ast, &mut interner);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        // Compare structure (spans may differ)
        if let (Ast::List(orig), Ast::List(res)) = (&ast.node, &result.node) {
            assert_eq!(orig.len(), res.len());
            for (orig_elem, res_elem) in orig.iter().zip(res.iter()) {
                assert_eq!(orig_elem.node, res_elem.node);
            }
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn roundtrip_vector() {
        let mut interner = symbol::Interner::new();

        let elements = vec![spanned(Ast::Integer(1_i64)), spanned(Ast::Integer(2_i64))];
        let ast = spanned(Ast::Vector(elements));
        let value = ast_to_value(&ast, &mut interner);
        let result = value_to_ast(&value, &interner, test_span()).unwrap();

        if let (Ast::Vector(orig), Ast::Vector(res)) = (&ast.node, &result.node) {
            assert_eq!(orig.len(), res.len());
        } else {
            panic!("Expected Vector");
        }
    }

    // =========================================================================
    // Error cases
    // =========================================================================

    #[test]
    fn value_to_ast_function_error() {
        use lona_core::chunk::Chunk;
        use lona_core::value::Function;

        let interner = symbol::Interner::new();
        let chunk = alloc::sync::Arc::new(Chunk::new());
        let func = Function::new(chunk, 0_u8, None);
        let value = Value::Function(func);

        let result = value_to_ast(&value, &interner, test_span());
        assert!(result.is_err());
        if let Err(Error::InvalidMacroResult { message, .. }) = result {
            assert!(message.contains("function"));
        }
    }

    #[test]
    fn value_to_ast_ratio_error() {
        use lona_core::ratio::Ratio;

        let interner = symbol::Interner::new();
        let ratio = Ratio::from_i64(1_i64, 3_i64);
        let value = Value::Ratio(ratio);

        let result = value_to_ast(&value, &interner, test_span());
        assert!(result.is_err());
        if let Err(Error::InvalidMacroResult { message, .. }) = result {
            assert!(message.contains("ratio"));
        }
    }
}
