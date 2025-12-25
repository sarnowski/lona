// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for String values.

use super::*;
use alloc::string::ToString;

#[test]
fn string_equality() {
    let s1 = Value::String(HeapStr::new("hello"));
    let s2 = Value::String(HeapStr::new("hello"));
    let s3 = Value::String(HeapStr::new("world"));

    assert_eq!(s1, s2);
    assert_ne!(s1, s3);
}

#[test]
fn display_string() {
    let string = Value::String(HeapStr::new("hello world"));
    assert_eq!(string.to_string(), "\"hello world\"");
}

#[test]
fn display_string_empty() {
    let string = Value::String(HeapStr::new(""));
    assert_eq!(string.to_string(), "\"\"");
}

#[test]
fn display_string_with_interner() {
    let interner = Interner::new();
    let string = Value::String(HeapStr::new("hello"));
    // Strings should be quoted for readable output
    assert_eq!(string.display(&interner).to_string(), "\"hello\"");
}

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

#[test]
fn display_string_with_interner_escapes_newline() {
    let interner = Interner::new();
    // String containing a newline
    let string = Value::String(HeapStr::new("line1\nline2"));
    // Newlines should be escaped
    assert_eq!(string.display(&interner).to_string(), "\"line1\\nline2\"");
}

#[test]
fn display_string_with_interner_escapes_tab() {
    let interner = Interner::new();
    // String containing a tab
    let string = Value::String(HeapStr::new("col1\tcol2"));
    // Tabs should be escaped
    assert_eq!(string.display(&interner).to_string(), "\"col1\\tcol2\"");
}

#[test]
fn display_string_with_interner_escapes_carriage_return() {
    let interner = Interner::new();
    // String containing a carriage return
    let string = Value::String(HeapStr::new("line1\rline2"));
    // Carriage returns should be escaped
    assert_eq!(string.display(&interner).to_string(), "\"line1\\rline2\"");
}

#[test]
fn is_string() {
    assert!(Value::String(HeapStr::new("test")).is_string());
    assert!(!Value::Nil.is_string());
    assert!(!Value::Integer(Integer::from_i64(42)).is_string());
}

#[test]
fn as_string() {
    let string = HeapStr::new("test");
    let value = Value::String(string.clone());
    assert_eq!(value.as_string(), Some(&string));
    assert_eq!(Value::Nil.as_string(), None);
}

#[test]
fn from_heap_str() {
    let string = HeapStr::new("test");
    let value = Value::from(string.clone());
    assert_eq!(value, Value::String(string));
}

#[test]
fn from_str_slice() {
    let value = Value::from("hello");
    assert_eq!(value, Value::String(HeapStr::new("hello")));
}

#[test]
fn string_is_truthy() {
    // All strings are truthy, even empty ones
    assert!(Value::String(HeapStr::new("")).is_truthy());
    assert!(Value::String(HeapStr::new("hello")).is_truthy());
}

#[test]
fn string_not_equal_to_other_types() {
    let string = Value::String(HeapStr::new("42"));
    assert_ne!(string, Value::Integer(Integer::from_i64(42)));
    assert_ne!(string, Value::Nil);
    assert_ne!(string, Value::Bool(true));
}

#[test]
fn string_clone_shares_data() {
    let s1 = Value::String(HeapStr::new("hello"));
    let s2 = s1.clone();
    assert_eq!(s1, s2);
    // Both are still valid after clone
    assert_eq!(s1.as_string().unwrap().as_str(), "hello");
    assert_eq!(s2.as_string().unwrap().as_str(), "hello");
}
