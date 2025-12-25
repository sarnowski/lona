// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Nested pattern tests.
//!
//! Tests complex patterns with maps inside sequences, sequences inside maps,
//! and deeply nested structures.

use alloc::boxed::Box;
use alloc::vec;

use lona_core::map::Map;
use lona_core::value::Value;
use lona_core::vector::Vector;
use lona_core::{integer::Integer, symbol::Interner};

use super::make_symbol;
use crate::vm::pattern::{Pattern, try_match};

/// Helper to create a keyword Value from a string.
fn make_keyword(interner: &Interner, name: &str) -> Value {
    Value::Keyword(interner.intern(name))
}

// =========================================================================
// Map inside sequence
// =========================================================================

#[test]
fn seq_containing_map_pattern() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let key_a = make_keyword(&interner, "a");

    // Pattern: [{:a x}]
    let pattern = Pattern::Seq {
        items: vec![Pattern::Map {
            entries: vec![(key_a.clone(), Pattern::Bind(x))],
        }],
        rest: None,
    };

    // Value: [{:a 42}]
    let inner_map = Map::empty().assoc(key_a, Value::Integer(Integer::from(42)));
    let vec = Vector::from_vec(vec![Value::Map(inner_map)]);
    let value = Value::Vector(vec);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, Value::Integer(Integer::from(42)))]));
}

#[test]
fn seq_containing_multiple_maps() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let y = make_symbol(&interner, "y");
    let key_a = make_keyword(&interner, "a");
    let key_b = make_keyword(&interner, "b");

    // Pattern: [{:a x} {:b y}]
    let pattern = Pattern::Seq {
        items: vec![
            Pattern::Map {
                entries: vec![(key_a.clone(), Pattern::Bind(x))],
            },
            Pattern::Map {
                entries: vec![(key_b.clone(), Pattern::Bind(y))],
            },
        ],
        rest: None,
    };

    // Value: [{:a 1} {:b 2}]
    let map1 = Map::empty().assoc(key_a, Value::Integer(Integer::from(1)));
    let map2 = Map::empty().assoc(key_b, Value::Integer(Integer::from(2)));
    let vec = Vector::from_vec(vec![Value::Map(map1), Value::Map(map2)]);
    let value = Value::Vector(vec);

    let result = try_match(&pattern, &value);
    assert_eq!(
        result,
        Some(vec![
            (x, Value::Integer(Integer::from(1))),
            (y, Value::Integer(Integer::from(2))),
        ])
    );
}

// =========================================================================
// Sequence inside map
// =========================================================================

#[test]
fn map_containing_seq_pattern() {
    let interner = Interner::new();
    let a = make_symbol(&interner, "a");
    let b = make_symbol(&interner, "b");
    let key_items = make_keyword(&interner, "items");

    // Pattern: {:items [a b]}
    let pattern = Pattern::Map {
        entries: vec![(
            key_items.clone(),
            Pattern::Seq {
                items: vec![Pattern::Bind(a), Pattern::Bind(b)],
                rest: None,
            },
        )],
    };

    // Value: {:items [1 2]}
    let inner_vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
    ]);
    let map = Map::empty().assoc(key_items, Value::Vector(inner_vec));
    let value = Value::Map(map);

    let result = try_match(&pattern, &value);
    assert_eq!(
        result,
        Some(vec![
            (a, Value::Integer(Integer::from(1))),
            (b, Value::Integer(Integer::from(2))),
        ])
    );
}

#[test]
fn map_with_seq_and_rest() {
    let interner = Interner::new();
    let first = make_symbol(&interner, "first");
    let rest = make_symbol(&interner, "rest");
    let key_data = make_keyword(&interner, "data");

    // Pattern: {:data [first & rest]}
    let pattern = Pattern::Map {
        entries: vec![(
            key_data.clone(),
            Pattern::Seq {
                items: vec![Pattern::Bind(first)],
                rest: Some(Box::new(Pattern::Bind(rest))),
            },
        )],
    };

    // Value: {:data [1 2 3]}
    let inner_vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
        Value::Integer(Integer::from(3)),
    ]);
    let map = Map::empty().assoc(key_data, Value::Vector(inner_vec));
    let value = Value::Map(map);

    let result = try_match(&pattern, &value);
    let expected_rest = Value::Vector(Vector::from_vec(vec![
        Value::Integer(Integer::from(2)),
        Value::Integer(Integer::from(3)),
    ]));
    assert_eq!(
        result,
        Some(vec![
            (first, Value::Integer(Integer::from(1))),
            (rest, expected_rest),
        ])
    );
}

// =========================================================================
// Map inside map
// =========================================================================

#[test]
fn nested_map_patterns() {
    let interner = Interner::new();
    let name = make_symbol(&interner, "name");
    let key_user = make_keyword(&interner, "user");
    let key_name = make_keyword(&interner, "name");

    // Pattern: {:user {:name name}}
    let pattern = Pattern::Map {
        entries: vec![(
            key_user.clone(),
            Pattern::Map {
                entries: vec![(key_name.clone(), Pattern::Bind(name))],
            },
        )],
    };

    // Value: {:user {:name 123}}
    let inner_map = Map::empty().assoc(key_name, Value::Integer(Integer::from(123)));
    let outer_map = Map::empty().assoc(key_user, Value::Map(inner_map));
    let value = Value::Map(outer_map);

    let result = try_match(&pattern, &value);
    assert_eq!(
        result,
        Some(vec![(name, Value::Integer(Integer::from(123)))])
    );
}

// =========================================================================
// Deep nesting
// =========================================================================

#[test]
fn deeply_nested_seq_in_map_in_seq() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let key_x = make_keyword(&interner, "x");

    // Pattern: [[{:x x}]]
    let pattern = Pattern::Seq {
        items: vec![Pattern::Seq {
            items: vec![Pattern::Map {
                entries: vec![(key_x.clone(), Pattern::Bind(x))],
            }],
            rest: None,
        }],
        rest: None,
    };

    // Value: [[{:x 42}]]
    let inner_map = Map::empty().assoc(key_x, Value::Integer(Integer::from(42)));
    let inner_vec = Vector::from_vec(vec![Value::Map(inner_map)]);
    let outer_vec = Vector::from_vec(vec![Value::Vector(inner_vec)]);
    let value = Value::Vector(outer_vec);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, Value::Integer(Integer::from(42)))]));
}

#[test]
fn deeply_nested_map_in_seq_in_map() {
    let interner = Interner::new();
    let val = make_symbol(&interner, "val");
    let key_outer = make_keyword(&interner, "outer");
    let key_inner = make_keyword(&interner, "inner");

    // Pattern: {:outer [{:inner val}]}
    let pattern = Pattern::Map {
        entries: vec![(
            key_outer.clone(),
            Pattern::Seq {
                items: vec![Pattern::Map {
                    entries: vec![(key_inner.clone(), Pattern::Bind(val))],
                }],
                rest: None,
            },
        )],
    };

    // Value: {:outer [{:inner 99}]}
    let inner_map = Map::empty().assoc(key_inner, Value::Integer(Integer::from(99)));
    let vec = Vector::from_vec(vec![Value::Map(inner_map)]);
    let outer_map = Map::empty().assoc(key_outer, Value::Vector(vec));
    let value = Value::Map(outer_map);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(val, Value::Integer(Integer::from(99)))]));
}

// =========================================================================
// Mixed pattern types
// =========================================================================

#[test]
fn mixed_wildcards_binds_literals_in_nested() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let key_type = make_keyword(&interner, "type");
    let key_data = make_keyword(&interner, "data");
    let kw_message = make_keyword(&interner, "message");

    // Pattern: {:type :message :data [_ x]}
    // Matches if :type is :message, ignores first data element, binds second
    let pattern = Pattern::Map {
        entries: vec![
            (key_type.clone(), Pattern::Literal(kw_message.clone())),
            (
                key_data.clone(),
                Pattern::Seq {
                    items: vec![Pattern::Wildcard, Pattern::Bind(x)],
                    rest: None,
                },
            ),
        ],
    };

    // Value: {:type :message :data [ignored 42]}
    let data_vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(999)),
        Value::Integer(Integer::from(42)),
    ]);
    let map = Map::empty()
        .assoc(key_type, kw_message)
        .assoc(key_data, Value::Vector(data_vec));
    let value = Value::Map(map);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, Value::Integer(Integer::from(42)))]));
}

#[test]
fn nested_fails_on_inner_mismatch() {
    let interner = Interner::new();
    let key_items = make_keyword(&interner, "items");

    // Pattern: {:items [1 2]}
    let pattern = Pattern::Map {
        entries: vec![(
            key_items.clone(),
            Pattern::Seq {
                items: vec![
                    Pattern::Literal(Value::Integer(Integer::from(1))),
                    Pattern::Literal(Value::Integer(Integer::from(2))),
                ],
                rest: None,
            },
        )],
    };

    // Value: {:items [1 3]} - second element doesn't match
    let inner_vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(3)),
    ]);
    let map = Map::empty().assoc(key_items, Value::Vector(inner_vec));
    let value = Value::Map(map);

    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn nested_fails_on_type_mismatch() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let key_items = make_keyword(&interner, "items");

    // Pattern: {:items [x]} - expects vector
    let pattern = Pattern::Map {
        entries: vec![(
            key_items.clone(),
            Pattern::Seq {
                items: vec![Pattern::Bind(x)],
                rest: None,
            },
        )],
    };

    // Value: {:items 42} - :items is an integer, not a vector
    let map = Map::empty().assoc(key_items, Value::Integer(Integer::from(42)));
    let value = Value::Map(map);

    assert_eq!(try_match(&pattern, &value), None);
}
