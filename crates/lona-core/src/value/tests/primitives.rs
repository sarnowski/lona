// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for primitive Value types: nil, bool, integer, float, symbol.

use super::*;
#[cfg(feature = "alloc")]
use alloc::string::ToString;

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
    let interner = Interner::new();
    let id = interner.intern("foo");
    let value = Value::from(id);
    // Without interner, shows raw ID
    assert_eq!(value.to_string(), "#<symbol:0>");
}

#[cfg(feature = "alloc")]
#[test]
fn display_symbol_with_interner() {
    let interner = Interner::new();
    let id = interner.intern("my-symbol");
    let value = Value::from(id);
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

    let interner = Interner::new();
    let plus_id = interner.intern("+");
    let x_id = interner.intern("x");
    let y_id = interner.intern("y");

    // Create list (+ x y)
    let list = List::empty()
        .cons(Value::from(y_id))
        .cons(Value::from(x_id))
        .cons(Value::from(plus_id));

    let value = Value::List(list);

    // With interner, symbols should show their names
    assert_eq!(value.display(&interner).to_string(), "(+ x y)");
}

#[cfg(feature = "alloc")]
#[test]
fn display_vector_with_symbols_resolves_names() {
    use crate::vector::Vector;

    let interner = Interner::new();
    let a_id = interner.intern("a");
    let b_id = interner.intern("b");

    // Create vector [a b]
    let vector = Vector::empty()
        .push(Value::from(a_id))
        .push(Value::from(b_id));

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
    let interner = Interner::new();
    let id = interner.intern("test");
    assert_eq!(Value::from(id).as_symbol(), Some(id));
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
    use crate::value::Symbol;
    let interner = Interner::new();
    let id = interner.intern("test");
    assert_eq!(Value::from(id), Value::Symbol(Symbol::new(id)));
}

#[test]
fn value_is_clone() {
    let v1 = int(42);
    let v2 = v1.clone();
    assert_eq!(v1, v2);
}
