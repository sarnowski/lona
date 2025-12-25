// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Guarded pattern tests.
//!
//! Note: The pattern engine only matches the inner pattern and returns bindings.
//! Guard evaluation is the VM's responsibility. These tests verify that
//! guarded patterns correctly delegate to their inner pattern.

use alloc::boxed::Box;
use alloc::vec;

use lona_core::value::Value;
use lona_core::vector::Vector;
use lona_core::{integer::Integer, symbol::Interner};

use super::make_symbol;
use crate::vm::pattern::{Pattern, try_match};

#[test]
fn guarded_pattern_matches_inner_pattern() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");

    let pattern = Pattern::Guarded {
        pattern: Box::new(Pattern::Bind(x)),
        guard_index: 0, // Guard index is ignored by pattern engine
    };

    let value = Value::Integer(Integer::from(42));
    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, value)]));
}

#[test]
fn guarded_pattern_fails_when_inner_fails() {
    let pattern = Pattern::Guarded {
        pattern: Box::new(Pattern::Literal(Value::Integer(Integer::from(42)))),
        guard_index: 0,
    };

    let value = Value::Integer(Integer::from(99));
    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn guarded_wildcard_always_matches() {
    let pattern = Pattern::Guarded {
        pattern: Box::new(Pattern::Wildcard),
        guard_index: 5, // Any guard index
    };

    assert_eq!(try_match(&pattern, &Value::Nil), Some(vec![]));
    assert_eq!(
        try_match(&pattern, &Value::Integer(Integer::from(42))),
        Some(vec![])
    );
}

#[test]
fn guarded_seq_pattern() {
    let mut interner = Interner::new();
    let a = make_symbol(&mut interner, "a");
    let b = make_symbol(&mut interner, "b");

    let pattern = Pattern::Guarded {
        pattern: Box::new(Pattern::Seq {
            items: vec![Pattern::Bind(a), Pattern::Bind(b)],
            rest: None,
        }),
        guard_index: 42,
    };

    let vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
    ]);
    let value = Value::Vector(vec);

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
fn nested_guarded_patterns() {
    let mut interner = Interner::new();
    let x = make_symbol(&mut interner, "x");

    // Outer guarded wrapping inner guarded
    let pattern = Pattern::Guarded {
        pattern: Box::new(Pattern::Guarded {
            pattern: Box::new(Pattern::Bind(x)),
            guard_index: 1,
        }),
        guard_index: 0,
    };

    let value = Value::Integer(Integer::from(42));
    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, value)]));
}
