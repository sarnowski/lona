// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Kind enum.

use super::*;
use alloc::string::ToString;

#[test]
fn kind_name_primitives() {
    assert_eq!(Kind::Nil.name(), "nil");
    assert_eq!(Kind::Bool.name(), "boolean");
    assert_eq!(Kind::Integer.name(), "integer");
    assert_eq!(Kind::Float.name(), "float");
    assert_eq!(Kind::Symbol.name(), "symbol");
}

#[cfg(feature = "alloc")]
#[test]
fn kind_name_heap_types() {
    assert_eq!(Kind::Ratio.name(), "ratio");
    assert_eq!(Kind::String.name(), "string");
    assert_eq!(Kind::List.name(), "list");
    assert_eq!(Kind::Vector.name(), "vector");
    assert_eq!(Kind::Map.name(), "map");
    assert_eq!(Kind::Function.name(), "function");
}

#[test]
fn kind_is_numeric() {
    assert!(Kind::Integer.is_numeric());
    assert!(Kind::Float.is_numeric());
    #[cfg(feature = "alloc")]
    assert!(Kind::Ratio.is_numeric());

    assert!(!Kind::Nil.is_numeric());
    assert!(!Kind::Bool.is_numeric());
    assert!(!Kind::Symbol.is_numeric());
}

#[cfg(feature = "alloc")]
#[test]
fn kind_is_numeric_heap_types() {
    assert!(!Kind::String.is_numeric());
    assert!(!Kind::List.is_numeric());
    assert!(!Kind::Vector.is_numeric());
    assert!(!Kind::Map.is_numeric());
    assert!(!Kind::Function.is_numeric());
}

#[cfg(feature = "alloc")]
#[test]
fn kind_is_sequence() {
    // Maps are sequences of [key value] pairs (Clojure semantics)
    assert!(Kind::List.is_sequence());
    assert!(Kind::Vector.is_sequence());
    assert!(Kind::String.is_sequence());
    assert!(Kind::Map.is_sequence());

    assert!(!Kind::Nil.is_sequence());
    assert!(!Kind::Integer.is_sequence());
}

#[cfg(feature = "alloc")]
#[test]
fn kind_is_callable() {
    assert!(Kind::Function.is_callable());

    assert!(!Kind::Nil.is_callable());
    assert!(!Kind::List.is_callable());
    assert!(!Kind::Symbol.is_callable());
}

#[test]
fn kind_display() {
    assert_eq!(Kind::Nil.to_string(), "nil");
    assert_eq!(Kind::Bool.to_string(), "boolean");
    assert_eq!(Kind::Integer.to_string(), "integer");
    assert_eq!(Kind::Float.to_string(), "float");
}

#[cfg(feature = "alloc")]
#[test]
fn kind_display_heap_types() {
    assert_eq!(Kind::String.to_string(), "string");
    assert_eq!(Kind::List.to_string(), "list");
    assert_eq!(Kind::Function.to_string(), "function");
}

#[test]
fn value_kind_primitives() {
    assert_eq!(Value::Nil.kind(), Kind::Nil);
    assert_eq!(Value::Bool(true).kind(), Kind::Bool);
    assert_eq!(Value::Bool(false).kind(), Kind::Bool);
    assert_eq!(int(42).kind(), Kind::Integer);
    assert_eq!(Value::Float(3.14).kind(), Kind::Float);
}

#[cfg(feature = "alloc")]
#[test]
fn value_kind_heap_types() {
    use crate::list::List;
    use crate::map::Map;
    use crate::vector::Vector;

    let interner = Interner::new();
    let id = interner.intern("test");

    assert_eq!(Value::from(id).kind(), Kind::Symbol);
    assert_eq!(Value::Ratio(Ratio::from_i64(1, 2)).kind(), Kind::Ratio);
    assert_eq!(Value::String(HeapStr::new("hello")).kind(), Kind::String);
    assert_eq!(Value::List(List::empty()).kind(), Kind::List);
    assert_eq!(Value::Vector(Vector::empty()).kind(), Kind::Vector);
    assert_eq!(Value::Map(Map::empty()).kind(), Kind::Map);
}

#[test]
fn kind_equality() {
    assert_eq!(Kind::Nil, Kind::Nil);
    assert_eq!(Kind::Integer, Kind::Integer);
    assert_ne!(Kind::Nil, Kind::Bool);
    assert_ne!(Kind::Integer, Kind::Float);
}

#[test]
fn kind_copy() {
    let kind1 = Kind::Integer;
    let kind2 = kind1; // Copy
    assert_eq!(kind1, kind2);
}
