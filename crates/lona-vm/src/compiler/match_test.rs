// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for match clause compilation.
//!
//! Tests the `match` special form compilation including:
//! - Literal patterns (integers, keywords, booleans, nil)
//! - Binding patterns (variable capture)
//! - Wildcard patterns
//! - Tuple destructuring
//! - Vector destructuring
//! - Map destructuring
//! - Guard clauses
//! - Multiple clause fallthrough
//! - Y register usage for bindings surviving function calls

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::compiler::compile;
use crate::platform::MockVSpace;
use crate::process::{Process, WorkerId};
use crate::reader::read;
use crate::realm::{Realm, bootstrap};
use crate::scheduler::Worker;
use crate::value::Value;
use crate::vm::execute;

// =============================================================================
// Test setup helpers
// =============================================================================

/// Create a test environment with bootstrapped realm and process.
fn setup() -> (Process, Realm, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let mut proc = Process::new(young_base, young_size, old_base, old_size);

    let realm_base = base.add(128 * 1024);
    let mut realm = Realm::new(realm_base, 64 * 1024);

    let result = bootstrap(&mut realm, &mut mem).unwrap();
    proc.bootstrap(result.ns_var, result.core_ns);

    (proc, realm, mem)
}

/// Evaluate a Lonala expression and return the result.
fn eval(src: &str, proc: &mut Process, realm: &mut Realm, mem: &mut MockVSpace) -> Value {
    let expr = read(src, proc, realm, mem)
        .expect("read error")
        .expect("empty input");
    let chunk = compile(expr, proc, mem, realm).expect("compile error");
    proc.set_chunk(chunk);
    let mut worker = Worker::new(WorkerId(0));
    execute(&mut worker, proc, mem, realm).expect("runtime error")
}

// =============================================================================
// Literal pattern tests
// =============================================================================

#[test]
fn match_literal_integer() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval("(match 42 42 :yes _ :no)", &mut proc, &mut realm, &mut mem);
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "yes");
}

#[test]
fn match_literal_integer_no_match_first_clause() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval("(match 99 42 :no _ :yes)", &mut proc, &mut realm, &mut mem);
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "yes");
}

#[test]
fn match_literal_keyword() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match :foo :bar 1 :foo 2 :baz 3)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn match_literal_nil() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match nil nil :is-nil _ :not-nil)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "is-nil");
}

#[test]
fn match_literal_true() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match true true :is-true false :is-false _ :other)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "is-true");
}

#[test]
fn match_literal_false() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match false true :is-true false :is-false _ :other)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "is-false");
}

// =============================================================================
// Binding pattern tests
// =============================================================================

#[test]
fn match_binding_returns_value() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval("(match 42 x x)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Value::Int(42));
}

#[test]
fn match_binding_in_expression() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval("(match 42 x (+ x 1))", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Value::Int(43));
}

#[test]
fn match_binding_multiple_variables() {
    // Multiple bindings in sequence
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match 10 x (match 20 y (+ x y)))",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(30));
}

// =============================================================================
// Wildcard pattern tests
// =============================================================================

#[test]
fn match_wildcard_always_matches() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval("(match 42 _ :matched)", &mut proc, &mut realm, &mut mem);
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "matched");
}

#[test]
fn match_wildcard_ignores_value() {
    let (mut proc, mut realm, mut mem) = setup();
    // Wildcard discards the value, body doesn't reference it
    let result = eval("(match :any-value _ 123)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Value::Int(123));
}

// =============================================================================
// Tuple destructuring tests
// =============================================================================

#[test]
fn match_tuple_simple() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match [1 2] [a b] (+ a b))",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(3));
}

#[test]
fn match_tuple_three_elements() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match [1 2 3] [a b c] (+ a (+ b c)))",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(6));
}

#[test]
fn match_tuple_with_literal() {
    let (mut proc, mut realm, mut mem) = setup();
    // Pattern [1 x] only matches tuples where first element is 1
    let result = eval(
        "(match [1 42] [1 x] x [2 x] 0)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(42));
}

#[test]
fn match_tuple_with_wildcard() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match [1 2 3] [a _ c] (+ a c))",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(4));
}

#[test]
fn match_tuple_wrong_arity_falls_through() {
    let (mut proc, mut realm, mut mem) = setup();
    // Pattern [a b] expects 2 elements, falls through to wildcard
    let result = eval(
        "(match [1 2 3] [a b] :two-elem _ :other)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "other");
}

#[test]
fn match_tuple_nested() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match [[1 2] 3] [[a b] c] (+ a (+ b c)))",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(6));
}

#[test]
fn match_non_tuple_against_tuple_pattern() {
    let (mut proc, mut realm, mut mem) = setup();
    // Integer doesn't match tuple pattern, falls through
    let result = eval(
        "(match 42 [a b] :tuple _ :not-tuple)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "not-tuple");
}

// =============================================================================
// Vector destructuring tests
// =============================================================================

#[test]
fn match_vector_simple() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match {1 2 3} {a b c} (+ a (+ b c)))",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(6));
}

#[test]
fn match_non_vector_against_vector_pattern() {
    let (mut proc, mut realm, mut mem) = setup();
    // Tuple doesn't match vector pattern
    let result = eval(
        "(match [1 2] {a b} :vector _ :not-vector)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "not-vector");
}

// =============================================================================
// Map destructuring tests
// =============================================================================

#[test]
fn match_map_single_key() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match %{:a 1} %{:a x} x _ 0)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(1));
}

#[test]
fn match_map_multiple_keys() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match %{:a 1 :b 2} %{:a x :b y} (+ x y) _ 0)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(3));
}

#[test]
fn match_map_missing_key_falls_through() {
    let (mut proc, mut realm, mut mem) = setup();
    // Map doesn't have :b key, pattern fails
    let result = eval(
        "(match %{:a 1} %{:a x :b y} :has-both _ :missing-key)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "missing-key");
}

// =============================================================================
// Guard clause tests
// =============================================================================

#[test]
fn match_guard_positive() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match 5 n when (> n 0) :positive n when (< n 0) :negative _ :zero)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "positive");
}

#[test]
fn match_guard_negative() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match -5 n when (> n 0) :positive n when (< n 0) :negative _ :zero)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "negative");
}

#[test]
fn match_guard_zero() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match 0 n when (> n 0) :positive n when (< n 0) :negative _ :zero)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "zero");
}

#[test]
fn match_guard_with_binding_in_guard() {
    let (mut proc, mut realm, mut mem) = setup();
    // Guard can reference bound variable
    let result = eval(
        "(match 42 x when (= x 42) :found _ :not-found)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "found");
}

// =============================================================================
// Multiple clause tests
// =============================================================================

#[test]
fn match_multiple_clauses_first() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval("(match :a :a 1 :b 2 :c 3)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Value::Int(1));
}

#[test]
fn match_multiple_clauses_middle() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval("(match :b :a 1 :b 2 :c 3)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Value::Int(2));
}

#[test]
fn match_multiple_clauses_last() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval("(match :c :a 1 :b 2 :c 3)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Value::Int(3));
}

// =============================================================================
// Complex patterns tests
// =============================================================================

#[test]
fn match_tuple_with_keyword_tag() {
    // Common pattern: [:ok value] or [:error msg]
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match [:ok 42] [:ok x] x [:error _] -1 _ 0)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(42));
}

#[test]
fn match_error_tuple() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match [:error \"oops\"] [:ok x] x [:error msg] :got-error _ :other)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "got-error");
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn match_empty_tuple() {
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match [] [] :empty _ :not-empty)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "empty");
}

#[test]
fn match_single_clause() {
    let (mut proc, mut realm, mut mem) = setup();
    // Match with just one clause that always matches
    let result = eval("(match 42 x x)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Value::Int(42));
}

// =============================================================================
// Y register tests (bindings surviving function calls)
// =============================================================================

#[test]
fn match_binding_survives_call() {
    // The binding `x` must survive the call to `+`
    // In BEAM-style VMs, this requires Y registers
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match 42 x (+ x (+ 1 2)))",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(45));
}

#[test]
fn match_binding_used_multiple_times_with_calls() {
    let (mut proc, mut realm, mut mem) = setup();
    // x is used multiple times across calls
    let result = eval(
        "(match 10 x (+ x (+ x x)))",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(30));
}

#[test]
fn match_multiple_bindings_survive_calls() {
    let (mut proc, mut realm, mut mem) = setup();
    // Both a and b must survive the nested additions
    let result = eval(
        "(match [3 4] [a b] (+ a (+ b (+ a b))))",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert_eq!(result, Value::Int(14));
}

// =============================================================================
// Tuple rest pattern tests
// =============================================================================

#[test]
fn match_tuple_rest_simple() {
    let (mut proc, mut realm, mut mem) = setup();
    // [h & t] should bind h to first element, t to rest as a tuple
    let result = eval("(match [1 2 3] [h & t] t)", &mut proc, &mut realm, &mut mem);
    // t should be [2 3] (a tuple, not a list)
    assert!(result.is_tuple());
    let len = proc.read_tuple_len(&mem, result).unwrap();
    assert_eq!(len, 2);
    let elem0 = proc.read_tuple_element(&mem, result, 0).unwrap();
    let elem1 = proc.read_tuple_element(&mem, result, 1).unwrap();
    assert_eq!(elem0, Value::Int(2));
    assert_eq!(elem1, Value::Int(3));
}

#[test]
fn match_tuple_rest_head_binding() {
    let (mut proc, mut realm, mut mem) = setup();
    // Head element should be bound correctly
    let result = eval("(match [1 2 3] [h & t] h)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Value::Int(1));
}

#[test]
fn match_tuple_rest_multiple_head() {
    let (mut proc, mut realm, mut mem) = setup();
    // [a b & t] should bind a to first, b to second, t to rest
    let result = eval(
        "(match [1 2 3 4] [a b & t] t)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_tuple());
    let len = proc.read_tuple_len(&mem, result).unwrap();
    assert_eq!(len, 2);
    let elem0 = proc.read_tuple_element(&mem, result, 0).unwrap();
    let elem1 = proc.read_tuple_element(&mem, result, 1).unwrap();
    assert_eq!(elem0, Value::Int(3));
    assert_eq!(elem1, Value::Int(4));
}

#[test]
fn match_tuple_rest_empty_tail() {
    let (mut proc, mut realm, mut mem) = setup();
    // When rest is empty, t should be an empty tuple
    let result = eval("(match [1] [h & t] t)", &mut proc, &mut realm, &mut mem);
    assert!(result.is_tuple());
    let len = proc.read_tuple_len(&mem, result).unwrap();
    assert_eq!(len, 0);
}

#[test]
fn match_tuple_rest_with_wildcard() {
    let (mut proc, mut realm, mut mem) = setup();
    // [h & _] should discard the rest
    let result = eval("(match [1 2 3] [h & _] h)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Value::Int(1));
}

// =============================================================================
// Regression tests: tuple rest pattern with insufficient elements
// =============================================================================

#[test]
fn regression_tuple_rest_short_tuple_falls_through() {
    // Issue 2: [a b & t] pattern should NOT match [1] (only 1 element, need 2+)
    // Before fix: crashes with "out of memory" when trying to extract element at index 1
    // After fix: falls through to wildcard clause
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval(
        "(match [1] [a b & t] :matched _ :nomatch)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(result.is_keyword());
    let kw_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(kw_str, "nomatch");
}

#[test]
fn regression_tuple_rest_exact_head_count_matches() {
    // [a b & t] on [1 2] should match with t = []
    let (mut proc, mut realm, mut mem) = setup();
    let result = eval("(match [1 2] [a b & t] t)", &mut proc, &mut realm, &mut mem);
    assert!(result.is_tuple());
    let len = proc.read_tuple_len(&mem, result).unwrap();
    assert_eq!(len, 0);
}

// =============================================================================
// Badmatch error tests
// =============================================================================

#[test]
fn regression_match_no_clause_raises_badmatch() {
    // When no clause matches, the process should exit with RuntimeError::Badmatch
    let (mut proc, mut realm, mut mem) = setup();

    let expr = read(
        "(match 42 1 :one 2 :two 3 :three)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .expect("read error")
    .expect("empty input");
    let chunk = compile(expr, &mut proc, &mut mem, &mut realm).expect("compile error");
    proc.set_chunk(chunk);
    let mut worker = Worker::new(WorkerId(0));
    let result = execute(&mut worker, &mut proc, &mut mem, &mut realm);

    // Should return RuntimeError::Badmatch with value 42
    match result {
        Err(crate::vm::RuntimeError::Badmatch { value }) => {
            assert_eq!(value, Value::Int(42));
        }
        Err(other) => panic!("expected Badmatch error, got: {other:?}"),
        Ok(value) => panic!("expected error, got value: {value:?}"),
    }
}
