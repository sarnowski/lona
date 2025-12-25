// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Property-based tests for pattern matching.
//!
//! Uses proptest to verify pattern matching invariants with random inputs.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use lona_core::map::Map;
use lona_core::value::Value;
use lona_core::vector::Vector;
use lona_core::{integer::Integer, symbol::Interner};
use proptest::prelude::*;

use super::make_symbol;
use crate::vm::pattern::{Pattern, try_match};

// =========================================================================
// Value generators
// =========================================================================

/// Generate a random simple Value (non-collection).
fn arb_simple_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Nil),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|n| Value::Integer(Integer::from(n))),
        any::<f64>().prop_filter_map("filter NaN", |f| {
            if f.is_nan() {
                None
            } else {
                Some(Value::Float(f))
            }
        }),
        // Note: We can't easily generate keywords in proptest without an interner,
        // so we skip keywords in random value generation
    ]
}

/// Generate a random Value (including collections, but shallow).
fn arb_value() -> impl Strategy<Value = Value> {
    arb_simple_value().prop_recursive(3, 10, 5, |inner| {
        prop_oneof![
            // Vector of values
            prop::collection::vec(inner.clone(), 0..5)
                .prop_map(|v| Value::Vector(Vector::from_vec(v))),
            // Map with integer keys (can't use keywords without interner)
            prop::collection::vec(
                (
                    any::<i64>().prop_map(|n| Value::Integer(Integer::from(n))),
                    inner
                ),
                0..3
            )
            .prop_map(|pairs| {
                let mut map = Map::empty();
                for (key, val) in pairs {
                    map = map.assoc(key, val);
                }
                Value::Map(map)
            }),
        ]
    })
}

// =========================================================================
// Property tests
// =========================================================================

proptest! {
    /// Wildcard pattern always matches any value.
    #[test]
    fn wildcard_always_matches(value in arb_value()) {
        let result = try_match(&Pattern::Wildcard, &value);
        prop_assert!(result.is_some());
        prop_assert!(result.as_ref().is_some_and(Vec::is_empty));
    }

    /// Bind pattern always matches and captures the value.
    #[test]
    fn bind_always_captures(value in arb_value()) {
        let mut interner = Interner::new();
        let x = make_symbol(&mut interner, "x");

        let result = try_match(&Pattern::Bind(x), &value);
        prop_assert!(result.is_some());
        let bindings = result.as_ref().map_or(&[] as &[_], Vec::as_slice);
        prop_assert_eq!(bindings.len(), 1);
        prop_assert_eq!(bindings.first().map(|(sym, _)| *sym), Some(x));
        prop_assert_eq!(bindings.first().map(|(_, v)| v), Some(&value));
    }

    /// Literal pattern only matches equal values.
    #[test]
    fn literal_matches_only_equal(
        lit in arb_simple_value(),
        value in arb_simple_value()
    ) {
        let pattern = Pattern::Literal(lit.clone());
        let result = try_match(&pattern, &value);

        if lit == value {
            prop_assert!(result.is_some());
            prop_assert!(result.as_ref().is_some_and(Vec::is_empty));
        } else {
            prop_assert!(result.is_none());
        }
    }

    /// Empty sequence matches empty vector.
    #[test]
    fn empty_seq_matches_empty_vec(_x in any::<u8>()) {
        let pattern = Pattern::Seq { items: vec![], rest: None };
        let value = Value::Vector(Vector::empty());

        let result = try_match(&pattern, &value);
        prop_assert!(result.is_some());
        prop_assert!(result.as_ref().is_some_and(Vec::is_empty));
    }

    /// Sequence pattern rejects non-sequence types.
    #[test]
    fn seq_rejects_non_sequence(value in arb_simple_value()) {
        let pattern = Pattern::Seq {
            items: vec![Pattern::Wildcard],
            rest: None,
        };

        let result = try_match(&pattern, &value);
        prop_assert!(result.is_none());
    }

    /// Map pattern with empty entries matches any map.
    #[test]
    fn empty_map_pattern_matches_any_map(
        pairs in prop::collection::vec(
            (any::<i64>().prop_map(|n| Value::Integer(Integer::from(n))), arb_simple_value()),
            0..5
        )
    ) {
        let mut map = Map::empty();
        for (key, val) in pairs {
            map = map.assoc(key, val);
        }
        let value = Value::Map(map);

        let pattern = Pattern::Map { entries: vec![] };
        let result = try_match(&pattern, &value);
        prop_assert!(result.is_some());
    }

    /// Map pattern rejects non-map types.
    #[test]
    fn map_pattern_rejects_non_map(value in arb_simple_value()) {
        let mut interner = Interner::new();
        let x = make_symbol(&mut interner, "x");
        let key = Value::Keyword(interner.intern("a"));

        let pattern = Pattern::Map {
            entries: vec![(key, Pattern::Bind(x))],
        };

        let result = try_match(&pattern, &value);
        prop_assert!(result.is_none());
    }

    /// Guarded pattern returns same bindings as inner pattern.
    #[test]
    fn guarded_same_as_inner(value in arb_value()) {
        let mut interner = Interner::new();
        let x = make_symbol(&mut interner, "x");

        let inner = Pattern::Bind(x);
        let guarded = Pattern::Guarded {
            pattern: Box::new(inner.clone()),
            guard_index: 0,
        };

        let inner_result = try_match(&inner, &value);
        let guarded_result = try_match(&guarded, &value);

        prop_assert_eq!(inner_result, guarded_result);
    }

    /// Sequence with rest pattern accepts any length >= items.len().
    #[test]
    fn seq_with_rest_accepts_longer(
        prefix_len in 1_usize..4,
        extra_len in 0_usize..5
    ) {
        let total_len = prefix_len.saturating_add(extra_len);

        // Create pattern with prefix_len wildcards and a rest
        let items: Vec<Pattern> = (0..prefix_len).map(|_| Pattern::Wildcard).collect();
        let pattern = Pattern::Seq {
            items,
            rest: Some(Box::new(Pattern::Wildcard)),
        };

        // Create value with total_len elements
        let elements: Vec<Value> = (0..total_len)
            .map(|i| {
                let int_val = i64::try_from(i).unwrap_or(0);
                Value::Integer(Integer::from(int_val))
            })
            .collect();
        let value = Value::Vector(Vector::from_vec(elements));

        let result = try_match(&pattern, &value);
        prop_assert!(result.is_some());
    }
}
