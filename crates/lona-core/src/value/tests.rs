// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Value.

use super::*;
#[cfg(feature = "alloc")]
use crate::symbol::Interner;
#[cfg(feature = "alloc")]
use alloc::string::ToString;

/// Helper to create an integer value.
#[cfg(feature = "alloc")]
fn int(value: i64) -> Value {
    Value::Integer(Integer::from_i64(value))
}

/// Helper to create an integer value (non-alloc).
#[cfg(not(feature = "alloc"))]
fn int(value: i64) -> Value {
    Value::Integer(value)
}

#[test]
fn nil_equality() {
    assert_eq!(Value::Nil, Value::Nil);
}

#[test]
fn bool_equality() {
    assert_eq!(Value::Bool(true), Value::Bool(true));
    assert_eq!(Value::Bool(false), Value::Bool(false));
    assert_ne!(Value::Bool(true), Value::Bool(false));
}

#[test]
fn integer_equality() {
    assert_eq!(int(42), int(42));
    assert_eq!(int(-1), int(-1));
    assert_ne!(int(1), int(2));
}

#[test]
fn float_equality() {
    assert_eq!(Value::Float(3.14), Value::Float(3.14));
    assert_eq!(Value::Float(-0.5), Value::Float(-0.5));
    assert_ne!(Value::Float(1.0), Value::Float(2.0));
}

#[test]
fn float_nan_not_equal_to_itself() {
    // NaN behavior: NaN != NaN
    let nan = Value::Float(f64::NAN);
    assert_ne!(nan, nan);
}

#[test]
fn different_types_not_equal() {
    assert_ne!(Value::Nil, Value::Bool(false));
    assert_ne!(int(0), Value::Float(0.0));
    assert_ne!(Value::Bool(true), int(1));
}

#[cfg(feature = "alloc")]
#[test]
fn display_nil() {
    assert_eq!(Value::Nil.to_string(), "nil");
}

#[cfg(feature = "alloc")]
#[test]
fn display_bool() {
    assert_eq!(Value::Bool(true).to_string(), "true");
    assert_eq!(Value::Bool(false).to_string(), "false");
}

#[cfg(feature = "alloc")]
#[test]
fn display_integer() {
    assert_eq!(int(42).to_string(), "42");
    assert_eq!(int(-17).to_string(), "-17");
    assert_eq!(int(0).to_string(), "0");
    assert_eq!(int(i64::MAX).to_string(), "9223372036854775807");
    assert_eq!(int(i64::MIN).to_string(), "-9223372036854775808");
}

#[cfg(feature = "alloc")]
#[test]
fn display_float() {
    assert_eq!(Value::Float(3.14).to_string(), "3.14");
    assert_eq!(Value::Float(-0.5).to_string(), "-0.5");
    // Whole numbers show decimal point
    assert_eq!(Value::Float(1.0).to_string(), "1.0");
    assert_eq!(Value::Float(-42.0).to_string(), "-42.0");
}

#[cfg(feature = "alloc")]
#[test]
fn display_float_special_values() {
    assert_eq!(Value::Float(f64::NAN).to_string(), "##NaN");
    assert_eq!(Value::Float(f64::INFINITY).to_string(), "##Inf");
    assert_eq!(Value::Float(f64::NEG_INFINITY).to_string(), "##-Inf");
}

#[cfg(feature = "alloc")]
#[test]
fn display_float_scientific() {
    // Very large numbers use scientific notation
    assert_eq!(Value::Float(1e20).to_string(), "100000000000000000000");
}

#[cfg(feature = "alloc")]
#[test]
fn display_symbol_without_interner() {
    let mut interner = Interner::new();
    let id = interner.intern("foo");
    let value = Value::Symbol(id);
    // Without interner, shows raw ID
    assert_eq!(value.to_string(), "#<symbol:0>");
}

#[cfg(feature = "alloc")]
#[test]
fn display_symbol_with_interner() {
    let mut interner = Interner::new();
    let id = interner.intern("my-symbol");
    let value = Value::Symbol(id);
    // With interner, shows symbol name
    assert_eq!(value.display(&interner).to_string(), "my-symbol");
}

#[cfg(feature = "alloc")]
#[test]
fn display_with_interner_passthrough() {
    let interner = Interner::new();

    // Non-symbol values pass through unchanged
    assert_eq!(Value::Nil.display(&interner).to_string(), "nil");
    assert_eq!(Value::Bool(true).display(&interner).to_string(), "true");
    assert_eq!(int(42).display(&interner).to_string(), "42");
    assert_eq!(Value::Float(3.14).display(&interner).to_string(), "3.14");
}

#[cfg(feature = "alloc")]
#[test]
fn display_list_with_symbols_resolves_names() {
    use crate::list::List;

    let mut interner = Interner::new();
    let plus_id = interner.intern("+");
    let x_id = interner.intern("x");
    let y_id = interner.intern("y");

    // Create list (+ x y)
    let list = List::empty()
        .cons(Value::Symbol(y_id))
        .cons(Value::Symbol(x_id))
        .cons(Value::Symbol(plus_id));

    let value = Value::List(list);

    // With interner, symbols should show their names
    assert_eq!(value.display(&interner).to_string(), "(+ x y)");
}

#[cfg(feature = "alloc")]
#[test]
fn display_vector_with_symbols_resolves_names() {
    use crate::vector::Vector;

    let mut interner = Interner::new();
    let a_id = interner.intern("a");
    let b_id = interner.intern("b");

    // Create vector [a b]
    let vector = Vector::empty()
        .push(Value::Symbol(a_id))
        .push(Value::Symbol(b_id));

    let value = Value::Vector(vector);

    // With interner, symbols should show their names
    assert_eq!(value.display(&interner).to_string(), "[a b]");
}

#[test]
fn is_nil() {
    assert!(Value::Nil.is_nil());
    assert!(!Value::Bool(false).is_nil());
    assert!(!int(0).is_nil());
}

#[test]
fn is_truthy() {
    // Only nil and false are falsy
    assert!(!Value::Nil.is_truthy());
    assert!(!Value::Bool(false).is_truthy());

    // Everything else is truthy
    assert!(Value::Bool(true).is_truthy());
    assert!(int(0).is_truthy()); // 0 is truthy!
    assert!(int(42).is_truthy());
    assert!(Value::Float(0.0).is_truthy()); // 0.0 is truthy!
    assert!(Value::Float(3.14).is_truthy());
}

#[test]
fn as_bool() {
    assert_eq!(Value::Bool(true).as_bool(), Some(true));
    assert_eq!(Value::Bool(false).as_bool(), Some(false));
    assert_eq!(Value::Nil.as_bool(), None);
    assert_eq!(int(1).as_bool(), None);
}

#[cfg(feature = "alloc")]
#[test]
fn as_integer() {
    assert_eq!(int(42).as_integer(), Some(&Integer::from_i64(42)));
    assert_eq!(int(-1).as_integer(), Some(&Integer::from_i64(-1)));
    assert_eq!(Value::Nil.as_integer(), None);
    assert_eq!(Value::Float(42.0).as_integer(), None);
}

#[test]
fn as_float() {
    assert_eq!(Value::Float(3.14).as_float(), Some(3.14));
    assert_eq!(Value::Nil.as_float(), None);
    assert_eq!(int(42).as_float(), None);
}

#[cfg(feature = "alloc")]
#[test]
fn as_symbol() {
    let mut interner = Interner::new();
    let id = interner.intern("test");
    assert_eq!(Value::Symbol(id).as_symbol(), Some(id));
    assert_eq!(Value::Nil.as_symbol(), None);
}

#[test]
fn from_bool() {
    assert_eq!(Value::from(true), Value::Bool(true));
    assert_eq!(Value::from(false), Value::Bool(false));
}

#[test]
fn from_i64() {
    assert_eq!(Value::from(42_i64), int(42));
    assert_eq!(Value::from(-1_i64), int(-1));
}

#[test]
fn from_i32() {
    assert_eq!(Value::from(42_i32), int(42));
}

#[test]
fn from_f64() {
    assert_eq!(Value::from(3.14_f64), Value::Float(3.14));
}

#[cfg(feature = "alloc")]
#[test]
fn from_symbol_id() {
    let mut interner = Interner::new();
    let id = interner.intern("test");
    assert_eq!(Value::from(id), Value::Symbol(id));
}

#[test]
fn value_is_clone() {
    let v1 = int(42);
    let v2 = v1.clone();
    assert_eq!(v1, v2);
}

// =========================================================================
// Ratio Tests
// =========================================================================

#[cfg(feature = "alloc")]
#[test]
fn ratio_equality() {
    let r1 = Value::Ratio(Ratio::from_i64(1, 2));
    let r2 = Value::Ratio(Ratio::from_i64(1, 2));
    let r3 = Value::Ratio(Ratio::from_i64(1, 3));

    assert_eq!(r1, r2);
    assert_ne!(r1, r3);
}

#[cfg(feature = "alloc")]
#[test]
fn ratio_equality_normalized() {
    // 2/4 should equal 1/2 after normalization
    let r1 = Value::Ratio(Ratio::from_i64(2, 4));
    let r2 = Value::Ratio(Ratio::from_i64(1, 2));
    assert_eq!(r1, r2);
}

#[cfg(feature = "alloc")]
#[test]
fn display_ratio() {
    let ratio = Value::Ratio(Ratio::from_i64(1, 3));
    assert_eq!(ratio.to_string(), "1/3");
}

#[cfg(feature = "alloc")]
#[test]
fn display_ratio_integer() {
    // Ratio that equals an integer displays as integer
    let ratio = Value::Ratio(Ratio::from_i64(4, 2));
    assert_eq!(ratio.to_string(), "2");
}

#[cfg(feature = "alloc")]
#[test]
fn is_ratio() {
    assert!(Value::Ratio(Ratio::from_i64(1, 2)).is_ratio());
    assert!(!Value::Nil.is_ratio());
    assert!(!int(42).is_ratio());
}

#[cfg(feature = "alloc")]
#[test]
fn as_ratio() {
    let ratio = Ratio::from_i64(1, 2);
    let value = Value::Ratio(ratio.clone());
    assert_eq!(value.as_ratio(), Some(&ratio));
    assert_eq!(Value::Nil.as_ratio(), None);
}

#[cfg(feature = "alloc")]
#[test]
fn from_ratio() {
    let ratio = Ratio::from_i64(1, 2);
    let value = Value::from(ratio.clone());
    assert_eq!(value, Value::Ratio(ratio));
}

#[cfg(feature = "alloc")]
#[test]
fn ratio_is_truthy() {
    // All ratios are truthy, including zero
    assert!(Value::Ratio(Ratio::from_i64(0, 1)).is_truthy());
    assert!(Value::Ratio(Ratio::from_i64(1, 2)).is_truthy());
}

#[cfg(feature = "alloc")]
#[test]
fn ratio_not_equal_to_integer() {
    // Even though 2/1 = 2, they are different types
    let ratio = Value::Ratio(Ratio::from_i64(2, 1));
    let integer = int(2);
    assert_ne!(ratio, integer);
}

#[cfg(feature = "alloc")]
#[test]
fn display_ratio_with_interner() {
    let interner = Interner::new();
    let ratio = Value::Ratio(Ratio::from_i64(1, 3));
    assert_eq!(ratio.display(&interner).to_string(), "1/3");
}

// =========================================================================
// String Tests
// =========================================================================

#[cfg(feature = "alloc")]
#[test]
fn string_equality() {
    let s1 = Value::String(HeapStr::new("hello"));
    let s2 = Value::String(HeapStr::new("hello"));
    let s3 = Value::String(HeapStr::new("world"));

    assert_eq!(s1, s2);
    assert_ne!(s1, s3);
}

#[cfg(feature = "alloc")]
#[test]
fn display_string() {
    let string = Value::String(HeapStr::new("hello world"));
    assert_eq!(string.to_string(), "\"hello world\"");
}

#[cfg(feature = "alloc")]
#[test]
fn display_string_empty() {
    let string = Value::String(HeapStr::new(""));
    assert_eq!(string.to_string(), "\"\"");
}

#[cfg(feature = "alloc")]
#[test]
fn display_string_with_interner() {
    let interner = Interner::new();
    let string = Value::String(HeapStr::new("hello"));
    // Strings should be quoted for readable output
    assert_eq!(string.display(&interner).to_string(), "\"hello\"");
}

#[cfg(feature = "alloc")]
#[test]
fn display_string_with_interner_escapes_quotes() {
    let interner = Interner::new();
    // String containing a quote character
    let string = Value::String(HeapStr::new("say \"hello\""));
    // Quotes inside the string should be escaped
    assert_eq!(
        string.display(&interner).to_string(),
        "\"say \\\"hello\\\"\""
    );
}

#[cfg(feature = "alloc")]
#[test]
fn display_string_with_interner_escapes_backslash() {
    let interner = Interner::new();
    // String containing a backslash
    let string = Value::String(HeapStr::new("path\\to\\file"));
    // Backslashes should be escaped
    assert_eq!(
        string.display(&interner).to_string(),
        "\"path\\\\to\\\\file\""
    );
}

#[cfg(feature = "alloc")]
#[test]
fn display_string_with_interner_escapes_newline() {
    let interner = Interner::new();
    // String containing a newline
    let string = Value::String(HeapStr::new("line1\nline2"));
    // Newlines should be escaped
    assert_eq!(string.display(&interner).to_string(), "\"line1\\nline2\"");
}

#[cfg(feature = "alloc")]
#[test]
fn display_string_with_interner_escapes_tab() {
    let interner = Interner::new();
    // String containing a tab
    let string = Value::String(HeapStr::new("col1\tcol2"));
    // Tabs should be escaped
    assert_eq!(string.display(&interner).to_string(), "\"col1\\tcol2\"");
}

#[cfg(feature = "alloc")]
#[test]
fn display_string_with_interner_escapes_carriage_return() {
    let interner = Interner::new();
    // String containing a carriage return
    let string = Value::String(HeapStr::new("line1\rline2"));
    // Carriage returns should be escaped
    assert_eq!(string.display(&interner).to_string(), "\"line1\\rline2\"");
}

#[cfg(feature = "alloc")]
#[test]
fn is_string() {
    assert!(Value::String(HeapStr::new("test")).is_string());
    assert!(!Value::Nil.is_string());
    assert!(!Value::Integer(Integer::from_i64(42)).is_string());
}

#[cfg(feature = "alloc")]
#[test]
fn as_string() {
    let string = HeapStr::new("test");
    let value = Value::String(string.clone());
    assert_eq!(value.as_string(), Some(&string));
    assert_eq!(Value::Nil.as_string(), None);
}

#[cfg(feature = "alloc")]
#[test]
fn from_heap_str() {
    let string = HeapStr::new("test");
    let value = Value::from(string.clone());
    assert_eq!(value, Value::String(string));
}

#[cfg(feature = "alloc")]
#[test]
fn from_str_slice() {
    let value = Value::from("hello");
    assert_eq!(value, Value::String(HeapStr::new("hello")));
}

#[cfg(feature = "alloc")]
#[test]
fn string_is_truthy() {
    // All strings are truthy, even empty ones
    assert!(Value::String(HeapStr::new("")).is_truthy());
    assert!(Value::String(HeapStr::new("hello")).is_truthy());
}

#[cfg(feature = "alloc")]
#[test]
fn string_not_equal_to_other_types() {
    let string = Value::String(HeapStr::new("42"));
    assert_ne!(string, Value::Integer(Integer::from_i64(42)));
    assert_ne!(string, Value::Nil);
    assert_ne!(string, Value::Bool(true));
}

#[cfg(feature = "alloc")]
#[test]
fn string_clone_shares_data() {
    let s1 = Value::String(HeapStr::new("hello"));
    let s2 = s1.clone();
    assert_eq!(s1, s2);
    // Both are still valid after clone
    assert_eq!(s1.as_string().unwrap().as_str(), "hello");
    assert_eq!(s2.as_string().unwrap().as_str(), "hello");
}

// =========================================================================
// Kind Tests
// =========================================================================

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
    use alloc::string::ToString;
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

    let mut interner = Interner::new();
    let id = interner.intern("test");

    assert_eq!(Value::Symbol(id).kind(), Kind::Symbol);
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
