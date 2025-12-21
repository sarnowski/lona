// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Conversion between AST and runtime Values.
//!
//! This module provides bidirectional conversion between parsed AST nodes
//! and runtime `Value` types. This is essential for macro expansion:
//!
//! 1. Macro arguments (AST) are converted to Values before calling the macro
//! 2. Macro results (Values) are converted back to AST for further compilation

#[cfg(test)]
#[path = "conversion_tests.rs"]
mod tests;

use alloc::string::String;
use alloc::vec::Vec;

use lona_core::integer::Integer;
use lona_core::list::List;
use lona_core::map::Map;
use lona_core::source;
use lona_core::span::Span;
use lona_core::string::HeapStr;
use lona_core::symbol;
use lona_core::value::Value;
use lona_core::vector::Vector;
use lonala_parser::{Ast, Spanned};

use crate::error::{Error, Kind as ErrorKind, SourceLocation};

/// Converts an AST node to a runtime Value.
///
/// Used to pass macro arguments to macro transformers. The AST is converted
/// to data values that the macro can inspect and manipulate.
///
/// # Symbol and Keyword Handling
///
/// Symbols are interned as `Value::Symbol` and keywords as `Value::Keyword`.
/// Keywords are stored without the colon prefix (the colon is syntax only).
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
            let id = interner.intern(name);
            Value::Keyword(id)
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
        Ast::Set(ref elements) => {
            let values: Vec<Value> = elements
                .iter()
                .map(|elem| ast_to_value(elem, interner))
                .collect();
            Value::Set(lona_core::set::Set::from_values(values))
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
    source_id: source::Id,
    span: Span,
) -> Result<Spanned<Ast>, Error> {
    let location = SourceLocation::new(source_id, span);
    let ast = match *value {
        Value::Nil => Ast::Nil,
        Value::Bool(bool_val) => Ast::Bool(bool_val),
        Value::Integer(ref int_val) => {
            // Convert arbitrary-precision integer to i64
            // If it doesn't fit, we have a problem - but for now most macros
            // work with small integers
            let i64_val = int_val.to_i64().ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidMacroResult {
                        message: String::from("integer too large for AST representation"),
                    },
                    location,
                )
            })?;
            Ast::Integer(i64_val)
        }
        Value::Float(num) => Ast::Float(num),
        Value::Ratio(ref _ratio) => {
            // Ratios don't have direct AST representation
            // Convert to a list form: (/ numerator denominator)
            return Err(Error::new(
                ErrorKind::InvalidMacroResult {
                    message: String::from("ratio values cannot be converted to AST"),
                },
                location,
            ));
        }
        Value::Symbol(id) => {
            let name = interner.resolve(id);
            Ast::Symbol(String::from(name))
        }
        Value::Keyword(id) => {
            let name = interner.resolve(id);
            Ast::Keyword(String::from(name))
        }
        Value::String(ref text) => Ast::String(String::from(text.as_str())),
        Value::List(ref list) => {
            let elements: Result<Vec<Spanned<Ast>>, Error> = list
                .iter()
                .map(|elem| value_to_ast(elem, interner, source_id, span))
                .collect();
            Ast::List(elements?)
        }
        Value::Vector(ref vector) => {
            let elements: Result<Vec<Spanned<Ast>>, Error> = vector
                .iter()
                .map(|elem| value_to_ast(elem, interner, source_id, span))
                .collect();
            Ast::Vector(elements?)
        }
        Value::Map(ref map) => {
            // Convert map back to flat list [k1 v1 k2 v2 ...]
            let mut elements = Vec::new();
            for (key, value) in map.iter() {
                elements.push(value_to_ast(key.value(), interner, source_id, span)?);
                elements.push(value_to_ast(value, interner, source_id, span)?);
            }
            Ast::Map(elements)
        }
        Value::Set(ref set) => {
            let elements: Result<Vec<Spanned<Ast>>, Error> = set
                .iter()
                .map(|key| value_to_ast(key.value(), interner, source_id, span))
                .collect();
            Ast::Set(elements?)
        }
        Value::Function(ref _func) => {
            return Err(Error::new(
                ErrorKind::InvalidMacroResult {
                    message: String::from("function values cannot be converted to AST"),
                },
                location,
            ));
        }
        Value::NativeFunction(_) => {
            return Err(Error::new(
                ErrorKind::InvalidMacroResult {
                    message: String::from("native function values cannot be converted to AST"),
                },
                location,
            ));
        }
        Value::Binary(_) => {
            return Err(Error::new(
                ErrorKind::InvalidMacroResult {
                    message: String::from("binary values cannot be converted to AST"),
                },
                location,
            ));
        }
        // Handle future Value variants (Value is non-exhaustive)
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidMacroResult {
                    message: String::from("unknown value type cannot be converted to AST"),
                },
                location,
            ));
        }
    };

    Ok(Spanned::new(ast, span))
}
