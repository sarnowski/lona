// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Depth limit tests for pattern matching.

use alloc::vec;

use lona_core::integer::Integer;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_core::vector::Vector;

use super::make_symbol;
use crate::vm::pattern::{MAX_PATTERN_DEPTH, Pattern, try_match};

#[test]
fn depth_limit_prevents_stack_overflow() {
    let interner = Interner::new();
    let x = make_symbol(&interner, "x");

    // Create a pattern nested deeper than MAX_PATTERN_DEPTH
    let mut pattern = Pattern::Bind(x);
    for _ in 0..=MAX_PATTERN_DEPTH.saturating_add(10) {
        pattern = Pattern::Seq {
            items: vec![pattern],
            rest: None,
        };
    }

    // Create a correspondingly nested value
    let mut value = Value::Integer(Integer::from(42));
    for _ in 0..=MAX_PATTERN_DEPTH.saturating_add(10) {
        value = Value::Vector(Vector::from_vec(vec![value]));
    }

    // Should return None due to depth limit, not panic
    assert_eq!(try_match(&pattern, &value), None);
}
