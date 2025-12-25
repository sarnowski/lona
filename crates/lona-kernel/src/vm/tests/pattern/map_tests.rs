// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Map pattern tests.

use alloc::vec;
use alloc::vec::Vec;

use lona_core::map::Map;
use lona_core::value::Value;
use lona_core::{integer::Integer, string::HeapStr, symbol::Interner};

use super::make_symbol;
use crate::vm::pattern::{Pattern, try_match};

/// Helper to create a keyword Value from a string.
fn make_keyword(interner: &mut Interner, name: &str) -> Value {
    Value::Keyword(interner.intern(name))
}

// =========================================================================
// Empty map pattern tests
// =========================================================================

#[test]
fn empty_map_pattern_matches_empty_map() {
    let pattern = Pattern::Map { entries: vec![] };
    let value = Value::Map(Map::empty());
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn empty_map_pattern_matches_any_map() {
    let mut interner = Interner::new();
    let pattern = Pattern::Map { entries: vec![] };
    let map = Map::empty()
        .assoc(
            make_keyword(&mut interner, "a"),
            Value::Integer(Integer::from(1)),
        )
        .assoc(
            make_keyword(&mut interner, "b"),
            Value::Integer(Integer::from(2)),
        );
    let value = Value::Map(map);
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

// =========================================================================
// Single entry tests
// =========================================================================

#[test]
fn map_pattern_binds_single_value() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");
    let key_a = make_keyword(&mut interner, "a");

    let pattern = Pattern::Map {
        entries: vec![(key_a.clone(), Pattern::Bind(x))],
    };
    let map = Map::empty().assoc(key_a, Value::Integer(Integer::from(42)));
    let value = Value::Map(map);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, Value::Integer(Integer::from(42)))]));
}

#[test]
fn map_pattern_fails_on_missing_key() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");
    let key_missing = make_keyword(&mut interner, "missing");
    let key_a = make_keyword(&mut interner, "a");

    let pattern = Pattern::Map {
        entries: vec![(key_missing, Pattern::Bind(x))],
    };
    let map = Map::empty().assoc(key_a, Value::Integer(Integer::from(42)));
    let value = Value::Map(map);

    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn map_pattern_with_literal_matches() {
    let mut interner = Interner::new();
    let key_a = make_keyword(&mut interner, "a");

    let pattern = Pattern::Map {
        entries: vec![(
            key_a.clone(),
            Pattern::Literal(Value::Integer(Integer::from(42))),
        )],
    };
    let map = Map::empty().assoc(key_a, Value::Integer(Integer::from(42)));
    let value = Value::Map(map);

    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn map_pattern_with_literal_rejects_mismatch() {
    let mut interner = Interner::new();
    let key_a = make_keyword(&mut interner, "a");

    let pattern = Pattern::Map {
        entries: vec![(
            key_a.clone(),
            Pattern::Literal(Value::Integer(Integer::from(42))),
        )],
    };
    let map = Map::empty().assoc(key_a, Value::Integer(Integer::from(99)));
    let value = Value::Map(map);

    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn map_pattern_with_wildcard() {
    let mut interner = Interner::new();
    let key_a = make_keyword(&mut interner, "a");

    let pattern = Pattern::Map {
        entries: vec![(key_a.clone(), Pattern::Wildcard)],
    };
    let map = Map::empty().assoc(key_a, Value::Integer(Integer::from(42)));
    let value = Value::Map(map);

    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

// =========================================================================
// Multiple entry tests
// =========================================================================

#[test]
fn map_pattern_binds_multiple_values() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");
    let y = make_symbol(&mut interner, "y");
    let key_a = make_keyword(&mut interner, "a");
    let key_b = make_keyword(&mut interner, "b");

    let pattern = Pattern::Map {
        entries: vec![
            (key_a.clone(), Pattern::Bind(x)),
            (key_b.clone(), Pattern::Bind(y)),
        ],
    };
    let map = Map::empty()
        .assoc(key_a, Value::Integer(Integer::from(1)))
        .assoc(key_b, Value::Integer(Integer::from(2)));
    let value = Value::Map(map);

    let result = try_match(&pattern, &value);
    assert_eq!(
        result,
        Some(vec![
            (x, Value::Integer(Integer::from(1))),
            (y, Value::Integer(Integer::from(2))),
        ])
    );
}

#[test]
fn map_pattern_ignores_extra_keys() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");
    let key_a = make_keyword(&mut interner, "a");
    let key_b = make_keyword(&mut interner, "b");
    let key_c = make_keyword(&mut interner, "c");

    let pattern = Pattern::Map {
        entries: vec![(key_a.clone(), Pattern::Bind(x))],
    };
    // Map has extra keys :b and :c that are not in the pattern
    let map = Map::empty()
        .assoc(key_a, Value::Integer(Integer::from(1)))
        .assoc(key_b, Value::Integer(Integer::from(2)))
        .assoc(key_c, Value::Integer(Integer::from(3)));
    let value = Value::Map(map);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, Value::Integer(Integer::from(1)))]));
}

#[test]
fn map_pattern_fails_if_any_key_missing() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");
    let y = make_symbol(&mut interner, "y");
    let key_a = make_keyword(&mut interner, "a");
    let key_missing = make_keyword(&mut interner, "missing");

    let pattern = Pattern::Map {
        entries: vec![
            (key_a.clone(), Pattern::Bind(x)),
            (key_missing, Pattern::Bind(y)),
        ],
    };
    let map = Map::empty().assoc(key_a, Value::Integer(Integer::from(1)));
    let value = Value::Map(map);

    assert_eq!(try_match(&pattern, &value), None);
}

// =========================================================================
// Different key types
// =========================================================================

#[test]
fn map_pattern_with_string_keys() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");

    let pattern = Pattern::Map {
        entries: vec![(Value::String(HeapStr::from("name")), Pattern::Bind(x))],
    };
    let map = Map::empty().assoc(
        Value::String(HeapStr::from("name")),
        Value::String(HeapStr::from("Alice")),
    );
    let value = Value::Map(map);

    let result = try_match(&pattern, &value);
    assert_eq!(
        result,
        Some(vec![(x, Value::String(HeapStr::from("Alice")))])
    );
}

#[test]
fn map_pattern_with_integer_keys() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");

    let pattern = Pattern::Map {
        entries: vec![(Value::Integer(Integer::from(0)), Pattern::Bind(x))],
    };
    let map = Map::empty().assoc(
        Value::Integer(Integer::from(0)),
        Value::String(HeapStr::from("first")),
    );
    let value = Value::Map(map);

    let result = try_match(&pattern, &value);
    assert_eq!(
        result,
        Some(vec![(x, Value::String(HeapStr::from("first")))])
    );
}

// =========================================================================
// Non-map type rejection tests
// =========================================================================

#[test]
fn map_pattern_rejects_non_map_types() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");
    let key_a = make_keyword(&mut interner, "a");

    let pattern = Pattern::Map {
        entries: vec![(key_a, Pattern::Bind(x))],
    };

    assert_eq!(try_match(&pattern, &Value::Nil), None);
    assert_eq!(try_match(&pattern, &Value::Bool(true)), None);
    assert_eq!(
        try_match(&pattern, &Value::Integer(Integer::from(42))),
        None
    );
    assert_eq!(try_match(&pattern, &Value::Float(3.14)), None);
    assert_eq!(
        try_match(&pattern, &Value::String(HeapStr::from("hello"))),
        None
    );
}
