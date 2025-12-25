// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Sequence pattern tests.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use lona_core::list::List;
use lona_core::value::Value;
use lona_core::vector::Vector;
use lona_core::{integer::Integer, string::HeapStr, symbol::Interner};

use super::make_symbol;
use crate::vm::pattern::{Pattern, try_match};

// =========================================================================
// Empty sequence tests
// =========================================================================

#[test]
fn empty_seq_matches_empty_vector() {
    let pattern = Pattern::Seq {
        items: vec![],
        rest: None,
    };
    let value = Value::Vector(Vector::empty());
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn empty_seq_matches_empty_list() {
    let pattern = Pattern::Seq {
        items: vec![],
        rest: None,
    };
    let value = Value::List(List::empty());
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn empty_seq_rejects_non_empty_vector() {
    let pattern = Pattern::Seq {
        items: vec![],
        rest: None,
    };
    let vec = Vector::from_vec(vec![Value::Integer(Integer::from(1))]);
    let value = Value::Vector(vec);
    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn empty_seq_rejects_non_empty_list() {
    let pattern = Pattern::Seq {
        items: vec![],
        rest: None,
    };
    let list = List::empty().cons(Value::Integer(Integer::from(1)));
    let value = Value::List(list);
    assert_eq!(try_match(&pattern, &value), None);
}

// =========================================================================
// Single element tests
// =========================================================================

#[test]
fn seq_single_wildcard_matches_single_element_vector() {
    let pattern = Pattern::Seq {
        items: vec![Pattern::Wildcard],
        rest: None,
    };
    let vec = Vector::from_vec(vec![Value::Integer(Integer::from(42))]);
    let value = Value::Vector(vec);
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn seq_single_bind_captures_single_element_vector() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");

    let pattern = Pattern::Seq {
        items: vec![Pattern::Bind(x)],
        rest: None,
    };
    let vec = Vector::from_vec(vec![Value::Integer(Integer::from(42))]);
    let value = Value::Vector(vec);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, Value::Integer(Integer::from(42)))]));
}

#[test]
fn seq_single_literal_matches_matching_element() {
    let pattern = Pattern::Seq {
        items: vec![Pattern::Literal(Value::Integer(Integer::from(42)))],
        rest: None,
    };
    let vec = Vector::from_vec(vec![Value::Integer(Integer::from(42))]);
    let value = Value::Vector(vec);
    assert_eq!(try_match(&pattern, &value), Some(Vec::new()));
}

#[test]
fn seq_single_literal_rejects_non_matching_element() {
    let pattern = Pattern::Seq {
        items: vec![Pattern::Literal(Value::Integer(Integer::from(42)))],
        rest: None,
    };
    let vec = Vector::from_vec(vec![Value::Integer(Integer::from(99))]);
    let value = Value::Vector(vec);
    assert_eq!(try_match(&pattern, &value), None);
}

// =========================================================================
// Multiple element tests
// =========================================================================

#[test]
fn seq_multiple_binds_capture_all_elements() {
    let interner = Interner::new();
    let a = make_symbol(&interner, "a");
    let b = make_symbol(&interner, "b");
    let c = make_symbol(&interner, "c");

    let pattern = Pattern::Seq {
        items: vec![Pattern::Bind(a), Pattern::Bind(b), Pattern::Bind(c)],
        rest: None,
    };
    let vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
        Value::Integer(Integer::from(3)),
    ]);
    let value = Value::Vector(vec);

    let result = try_match(&pattern, &value);
    assert_eq!(
        result,
        Some(vec![
            (a, Value::Integer(Integer::from(1))),
            (b, Value::Integer(Integer::from(2))),
            (c, Value::Integer(Integer::from(3))),
        ])
    );
}

#[test]
fn seq_mixed_patterns() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");

    let pattern = Pattern::Seq {
        items: vec![
            Pattern::Literal(Value::Integer(Integer::from(1))),
            Pattern::Wildcard,
            Pattern::Bind(x),
        ],
        rest: None,
    };
    let vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
        Value::Integer(Integer::from(3)),
    ]);
    let value = Value::Vector(vec);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, Value::Integer(Integer::from(3)))]));
}

#[test]
fn seq_rejects_wrong_length_too_short() {
    let pattern = Pattern::Seq {
        items: vec![Pattern::Wildcard, Pattern::Wildcard, Pattern::Wildcard],
        rest: None,
    };
    let vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
    ]);
    let value = Value::Vector(vec);
    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn seq_rejects_wrong_length_too_long() {
    let pattern = Pattern::Seq {
        items: vec![Pattern::Wildcard],
        rest: None,
    };
    let vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
    ]);
    let value = Value::Vector(vec);
    assert_eq!(try_match(&pattern, &value), None);
}

// =========================================================================
// Rest binding tests
// =========================================================================

#[test]
fn seq_with_rest_captures_remaining_vector() {
    let interner = Interner::new();
    let a = make_symbol(&interner, "a");
    let rest = make_symbol(&interner, "rest");

    let pattern = Pattern::Seq {
        items: vec![Pattern::Bind(a)],
        rest: Some(Box::new(Pattern::Bind(rest))),
    };
    let vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
        Value::Integer(Integer::from(3)),
    ]);
    let value = Value::Vector(vec);

    let result = try_match(&pattern, &value);
    let expected_rest = Value::Vector(Vector::from_vec(vec![
        Value::Integer(Integer::from(2)),
        Value::Integer(Integer::from(3)),
    ]));
    assert_eq!(
        result,
        Some(vec![
            (a, Value::Integer(Integer::from(1))),
            (rest, expected_rest),
        ])
    );
}

#[test]
fn seq_with_rest_captures_empty_when_exact() {
    let interner = Interner::new();
    let a = make_symbol(&interner, "a");
    let rest = make_symbol(&interner, "rest");

    let pattern = Pattern::Seq {
        items: vec![Pattern::Bind(a)],
        rest: Some(Box::new(Pattern::Bind(rest))),
    };
    let vec = Vector::from_vec(vec![Value::Integer(Integer::from(1))]);
    let value = Value::Vector(vec);

    let result = try_match(&pattern, &value);
    let expected_rest = Value::Vector(Vector::empty());
    assert_eq!(
        result,
        Some(vec![
            (a, Value::Integer(Integer::from(1))),
            (rest, expected_rest),
        ])
    );
}

#[test]
fn seq_with_rest_fails_when_not_enough_elements() {
    let interner = Interner::new();
    let a = make_symbol(&interner, "a");
    let b = make_symbol(&interner, "b");
    let rest = make_symbol(&interner, "rest");

    let pattern = Pattern::Seq {
        items: vec![Pattern::Bind(a), Pattern::Bind(b)],
        rest: Some(Box::new(Pattern::Bind(rest))),
    };
    let vec = Vector::from_vec(vec![Value::Integer(Integer::from(1))]);
    let value = Value::Vector(vec);

    assert_eq!(try_match(&pattern, &value), None);
}

#[test]
fn seq_with_wildcard_rest() {
    let interner = Interner::new();
    let a = make_symbol(&interner, "a");

    let pattern = Pattern::Seq {
        items: vec![Pattern::Bind(a)],
        rest: Some(Box::new(Pattern::Wildcard)),
    };
    let vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
        Value::Integer(Integer::from(3)),
    ]);
    let value = Value::Vector(vec);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(a, Value::Integer(Integer::from(1)))]));
}

#[test]
fn seq_rest_only_matches_empty() {
    let interner = Interner::new();
    let rest = make_symbol(&interner, "rest");

    let pattern = Pattern::Seq {
        items: vec![],
        rest: Some(Box::new(Pattern::Bind(rest))),
    };
    let value = Value::Vector(Vector::empty());

    let result = try_match(&pattern, &value);
    let expected_rest = Value::Vector(Vector::empty());
    assert_eq!(result, Some(vec![(rest, expected_rest)]));
}

#[test]
fn seq_rest_only_captures_all() {
    let interner = Interner::new();
    let rest = make_symbol(&interner, "rest");

    let pattern = Pattern::Seq {
        items: vec![],
        rest: Some(Box::new(Pattern::Bind(rest))),
    };
    let vec = Vector::from_vec(vec![
        Value::Integer(Integer::from(1)),
        Value::Integer(Integer::from(2)),
        Value::Integer(Integer::from(3)),
    ]);
    let value = Value::Vector(vec.clone());

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(rest, Value::Vector(vec))]));
}

// =========================================================================
// List tests
// =========================================================================

#[test]
fn seq_binds_work_with_lists() {
    let interner = Interner::new();
    let a = make_symbol(&interner, "a");
    let b = make_symbol(&interner, "b");

    let pattern = Pattern::Seq {
        items: vec![Pattern::Bind(a), Pattern::Bind(b)],
        rest: None,
    };
    let list = List::empty()
        .cons(Value::Integer(Integer::from(2)))
        .cons(Value::Integer(Integer::from(1)));
    let value = Value::List(list);

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
fn seq_with_rest_captures_remaining_list() {
    let interner = Interner::new();
    let a = make_symbol(&interner, "a");
    let rest = make_symbol(&interner, "rest");

    let pattern = Pattern::Seq {
        items: vec![Pattern::Bind(a)],
        rest: Some(Box::new(Pattern::Bind(rest))),
    };
    let list = List::empty()
        .cons(Value::Integer(Integer::from(3)))
        .cons(Value::Integer(Integer::from(2)))
        .cons(Value::Integer(Integer::from(1)));
    let value = Value::List(list);

    let result = try_match(&pattern, &value);
    let expected_rest = Value::List(
        List::empty()
            .cons(Value::Integer(Integer::from(3)))
            .cons(Value::Integer(Integer::from(2))),
    );
    assert_eq!(
        result,
        Some(vec![
            (a, Value::Integer(Integer::from(1))),
            (rest, expected_rest),
        ])
    );
}

// =========================================================================
// Nested pattern tests
// =========================================================================

#[test]
fn nested_seq_patterns() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");
    let y = make_symbol(&interner, "y");

    let pattern = Pattern::Seq {
        items: vec![
            Pattern::Seq {
                items: vec![Pattern::Bind(x)],
                rest: None,
            },
            Pattern::Seq {
                items: vec![Pattern::Bind(y)],
                rest: None,
            },
        ],
        rest: None,
    };

    let inner1 = Vector::from_vec(vec![Value::Integer(Integer::from(1))]);
    let inner2 = Vector::from_vec(vec![Value::Integer(Integer::from(2))]);
    let outer = Vector::from_vec(vec![Value::Vector(inner1), Value::Vector(inner2)]);
    let value = Value::Vector(outer);

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
fn deeply_nested_patterns() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");

    let pattern = Pattern::Seq {
        items: vec![Pattern::Seq {
            items: vec![Pattern::Seq {
                items: vec![Pattern::Bind(x)],
                rest: None,
            }],
            rest: None,
        }],
        rest: None,
    };

    let inner = Vector::from_vec(vec![Value::Integer(Integer::from(42))]);
    let middle = Vector::from_vec(vec![Value::Vector(inner)]);
    let outer = Vector::from_vec(vec![Value::Vector(middle)]);
    let value = Value::Vector(outer);

    let result = try_match(&pattern, &value);
    assert_eq!(result, Some(vec![(x, Value::Integer(Integer::from(42)))]));
}

// =========================================================================
// Non-sequence type rejection tests
// =========================================================================

#[test]
fn seq_rejects_non_sequence_types() {
    let pattern = Pattern::Seq {
        items: vec![Pattern::Wildcard],
        rest: None,
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
