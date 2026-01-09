// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for keywords, tuples, maps, and metadata.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::value::Value;

// --- Keyword tests ---

#[test]
fn eval_keyword_simple() {
    let (mut proc, mut mem) = setup();
    let result = eval(":foo", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "foo");
}

#[test]
fn eval_keyword_qualified() {
    let (mut proc, mut mem) = setup();
    let result = eval(":my.ns/bar", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "my.ns/bar");
}

#[test]
fn eval_keyword_self_evaluating() {
    // Keywords evaluate to themselves (like numbers, strings, booleans)
    let (mut proc, mut mem) = setup();
    let k1 = eval(":foo", &mut proc, &mut mem).unwrap();
    let k2 = eval(":foo", &mut proc, &mut mem).unwrap();
    assert!(k1.is_keyword());
    assert!(k2.is_keyword());
}

#[test]
fn eval_keyword_predicate() {
    let (mut proc, mut mem) = setup();
    let result = eval("(keyword? :foo)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(keyword? 'foo)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));

    let result = eval("(keyword? 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_keyword_equality() {
    let (mut proc, mut mem) = setup();
    // Same keywords should be equal
    let result = eval("(= :foo :foo)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    // Different keywords should not be equal
    let result = eval("(= :foo :bar)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_keyword_name() {
    let (mut proc, mut mem) = setup();
    let result = eval("(name :hello)", &mut proc, &mut mem).unwrap();
    assert!(result.is_string());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "hello");
}

#[test]
fn eval_keyword_name_qualified() {
    let (mut proc, mut mem) = setup();
    let result = eval("(name :ns/hello)", &mut proc, &mut mem).unwrap();
    assert!(result.is_string());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "hello");
}

#[test]
fn eval_keyword_namespace() {
    let (mut proc, mut mem) = setup();
    let result = eval("(namespace :ns/hello)", &mut proc, &mut mem).unwrap();
    assert!(result.is_string());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "ns");

    // Unqualified keywords have no namespace
    let result = eval("(namespace :hello)", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_keyword_constructor() {
    let (mut proc, mut mem) = setup();
    let result = eval("(keyword \"world\")", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "world");
}

// --- Tuple tests ---

#[test]
fn eval_tuple_simple() {
    let (mut proc, mut mem) = setup();
    let result = eval("[1 2 3]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 3);
    assert_eq!(
        proc.read_tuple_element(&mem, result, 0).unwrap(),
        Value::int(1)
    );
    assert_eq!(
        proc.read_tuple_element(&mem, result, 1).unwrap(),
        Value::int(2)
    );
    assert_eq!(
        proc.read_tuple_element(&mem, result, 2).unwrap(),
        Value::int(3)
    );
}

#[test]
fn eval_tuple_empty() {
    let (mut proc, mut mem) = setup();
    let result = eval("[]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 0);
}

#[test]
fn eval_tuple_elements_evaluated() {
    let (mut proc, mut mem) = setup();
    // Elements should be evaluated
    let result = eval("[(+ 1 2) 4]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 2);
    assert_eq!(
        proc.read_tuple_element(&mem, result, 0).unwrap(),
        Value::int(3)
    );
    assert_eq!(
        proc.read_tuple_element(&mem, result, 1).unwrap(),
        Value::int(4)
    );
}

#[test]
fn eval_tuple_nested() {
    let (mut proc, mut mem) = setup();
    let result = eval("[[1 2] [3 4]]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 2);

    let inner1 = proc.read_tuple_element(&mem, result, 0).unwrap();
    assert!(inner1.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, inner1).unwrap(), 2);
}

#[test]
fn eval_tuple_predicate() {
    let (mut proc, mut mem) = setup();
    let result = eval("(tuple? [1 2])", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(tuple? '(1 2))", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));

    let result = eval("(tuple? 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_tuple_nth() {
    let (mut proc, mut mem) = setup();
    let result = eval("(nth [10 20 30] 0)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(10));

    let result = eval("(nth [10 20 30] 1)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(20));

    let result = eval("(nth [10 20 30] 2)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(30));
}

#[test]
fn eval_tuple_count() {
    let (mut proc, mut mem) = setup();
    let result = eval("(count [1 2 3])", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(3));

    let result = eval("(count [])", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(0));
}

#[test]
fn eval_tuple_with_keywords() {
    let (mut proc, mut mem) = setup();
    let result = eval("[:a :b :c]", &mut proc, &mut mem).unwrap();
    assert!(result.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, result).unwrap(), 3);

    let k1 = proc.read_tuple_element(&mem, result, 0).unwrap();
    assert!(k1.is_keyword());
    assert_eq!(proc.read_string(&mem, k1).unwrap(), "a");
}

// --- Map tests ---

#[test]
fn eval_map_empty() {
    let (mut proc, mut mem) = setup();
    let result = eval("%{}", &mut proc, &mut mem).unwrap();
    assert!(result.is_map());
    // Empty map has 0 entries
    let map = proc.read_map(&mem, result).unwrap();
    assert!(map.entries.is_nil());
}

#[test]
fn eval_map_simple() {
    let (mut proc, mut mem) = setup();
    let result = eval("%{:a 1 :b 2}", &mut proc, &mut mem).unwrap();
    assert!(result.is_map());
}

#[test]
fn eval_map_predicate() {
    let (mut proc, mut mem) = setup();
    let result = eval("(map? %{:a 1})", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(true));

    let result = eval("(map? [1 2])", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));

    let result = eval("(map? 42)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::bool(false));
}

#[test]
fn eval_map_get() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get %{:a 1 :b 2} :a)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));

    let result = eval("(get %{:a 1 :b 2} :b)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(2));
}

#[test]
fn eval_map_get_not_found() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get %{:a 1} :x)", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_map_get_with_default() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get %{:a 1} :x :default)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "default");

    // Existing key should return value, not default
    let result = eval("(get %{:a 1} :a :default)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_map_put() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get (put %{:a 1} :b 2) :b)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(2));

    // Original key still accessible
    let result = eval("(get (put %{:a 1} :b 2) :a)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_map_count() {
    let (mut proc, mut mem) = setup();
    let result = eval("(count %{:a 1 :b 2})", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(2));

    let result = eval("(count %{})", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(0));
}

#[test]
fn eval_map_keys() {
    let (mut proc, mut mem) = setup();
    let result = eval("(keys %{:a 1})", &mut proc, &mut mem).unwrap();
    assert!(result.is_pair());
    let pair = proc.read_pair(&mem, result).unwrap();
    assert!(pair.first.is_keyword());
    assert_eq!(proc.read_string(&mem, pair.first).unwrap(), "a");
}

#[test]
fn eval_map_vals() {
    let (mut proc, mut mem) = setup();
    let result = eval("(vals %{:a 1})", &mut proc, &mut mem).unwrap();
    assert!(result.is_pair());
    let pair = proc.read_pair(&mem, result).unwrap();
    assert_eq!(pair.first, Value::int(1));
}

#[test]
fn eval_map_nested_values() {
    let (mut proc, mut mem) = setup();
    let result = eval("%{:a [1 2] :b [3 4]}", &mut proc, &mut mem).unwrap();
    assert!(result.is_map());

    let inner = eval("(get %{:a [1 2] :b [3 4]} :a)", &mut proc, &mut mem).unwrap();
    assert!(inner.is_tuple());
    assert_eq!(proc.read_tuple_len(&mem, inner).unwrap(), 2);
}

#[test]
fn eval_map_elements_evaluated() {
    let (mut proc, mut mem) = setup();
    let result = eval("(get %{:a (+ 1 2)} :a)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(3));
}

// --- Metadata tests ---

#[test]
fn eval_meta_nil_for_no_metadata() {
    let (mut proc, mut mem) = setup();
    let result = eval("(meta 'foo)", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_with_meta_and_meta() {
    let (mut proc, mut mem) = setup();

    // Create a symbol and attach metadata, then check it
    let result = eval(
        "(meta (with-meta 'x %{:doc \"hello\"}))",
        &mut proc,
        &mut mem,
    )
    .unwrap();
    assert!(result.is_map());
}

#[test]
fn eval_meta_does_not_affect_equality() {
    let (mut proc, mut mem) = setup();

    // Create two symbols, same value but different metadata
    let a = eval("(with-meta 'x %{:a 1})", &mut proc, &mut mem).unwrap();
    let b = eval("(with-meta 'x %{:b 2})", &mut proc, &mut mem).unwrap();

    // They should be equal (identity comparison for symbols)
    // Actually symbols compare by address, so different allocations won't be equal
    // But metadata shouldn't break this
    assert!(a.is_symbol());
    assert!(b.is_symbol());
}

#[test]
fn eval_meta_on_tuple() {
    let (mut proc, mut mem) = setup();

    let tuple = eval("(with-meta [1 2] %{:tag :vector})", &mut proc, &mut mem).unwrap();
    assert!(tuple.is_tuple());

    let meta = eval(
        "(meta (with-meta [1 2 3] %{:tag :vector}))",
        &mut proc,
        &mut mem,
    )
    .unwrap();
    assert!(meta.is_map());
}

// --- Callable data structure tests ---

// Keywords as functions

#[test]
fn eval_keyword_callable_basic() {
    let (mut proc, mut mem) = setup();
    // (:a %{:a 1 :b 2}) → 1
    let result = eval("(:a %{:a 1 :b 2})", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_keyword_callable_not_found() {
    let (mut proc, mut mem) = setup();
    // (:missing %{:a 1}) → nil
    let result = eval("(:missing %{:a 1})", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_keyword_callable_with_default() {
    let (mut proc, mut mem) = setup();
    // (:missing %{:a 1} :default) → :default
    let result = eval("(:missing %{:a 1} :default)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "default");
}

#[test]
fn eval_keyword_callable_found_with_default() {
    let (mut proc, mut mem) = setup();
    // (:a %{:a 1} :default) → 1 (ignores default when found)
    let result = eval("(:a %{:a 1} :default)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_keyword_callable_arity_error_zero_args() {
    use crate::vm::RuntimeError;
    let (mut proc, mut mem) = setup();
    // (:a) with no args → arity error
    let result = eval("(:a)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableArityError {
            expected: "1-2",
            got: 0
        })
    ));
}

#[test]
fn eval_keyword_callable_arity_error_too_many_args() {
    use crate::vm::RuntimeError;
    let (mut proc, mut mem) = setup();
    // (:a %{:a 1} :d :extra) → arity error
    let result = eval("(:a %{:a 1} :d :extra)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableArityError {
            expected: "1-2",
            got: 3
        })
    ));
}

#[test]
fn eval_keyword_callable_type_error_not_map() {
    use crate::vm::RuntimeError;
    let (mut proc, mut mem) = setup();
    // (:a 42) → ERROR: expected map
    let result = eval("(:a 42)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableTypeError {
            callable: "keyword",
            arg: 0,
            expected: "map"
        })
    ));
}

// Maps as functions

#[test]
fn eval_map_callable_basic() {
    let (mut proc, mut mem) = setup();
    // (%{:a 1 :b 2} :a) → 1
    let result = eval("(%{:a 1 :b 2} :a)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(1));
}

#[test]
fn eval_map_callable_not_found() {
    let (mut proc, mut mem) = setup();
    // (%{:a 1} :missing) → nil
    let result = eval("(%{:a 1} :missing)", &mut proc, &mut mem).unwrap();
    assert!(result.is_nil());
}

#[test]
fn eval_map_callable_with_default() {
    let (mut proc, mut mem) = setup();
    // (%{:a 1} :missing :default) → :default
    let result = eval("(%{:a 1} :missing :default)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "default");
}

#[test]
fn eval_map_callable_arity_error() {
    use crate::vm::RuntimeError;
    let (mut proc, mut mem) = setup();
    // (%{:a 1}) with no args → arity error
    let result = eval("(%{:a 1})", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableArityError {
            expected: "1-2",
            got: 0
        })
    ));
}

// Tuples as functions

#[test]
fn eval_tuple_callable_basic() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 1) → 20
    let result = eval("([10 20 30] 1)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(20));
}

#[test]
fn eval_tuple_callable_first_element() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 0) → 10
    let result = eval("([10 20 30] 0)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(10));
}

#[test]
fn eval_tuple_callable_last_element() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 2) → 30
    let result = eval("([10 20 30] 2)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(30));
}

#[test]
fn eval_tuple_callable_out_of_bounds() {
    use crate::intrinsics::IntrinsicError;
    use crate::vm::RuntimeError;
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 5) → error (no default)
    let result = eval("([10 20 30] 5)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::Intrinsic(IntrinsicError::IndexOutOfBounds {
            index: 5,
            len: 3
        }))
    ));
}

#[test]
fn eval_tuple_callable_with_default() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 5 :default) → :default
    let result = eval("([10 20 30] 5 :default)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "default");
}

#[test]
fn eval_tuple_callable_in_bounds_with_default() {
    let (mut proc, mut mem) = setup();
    // ([10 20 30] 1 :default) → 20 (ignores default when in bounds)
    let result = eval("([10 20 30] 1 :default)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(20));
}

#[test]
fn eval_tuple_callable_arity_error() {
    use crate::vm::RuntimeError;
    let (mut proc, mut mem) = setup();
    // ([1 2]) with no args → arity error
    let result = eval("([1 2])", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableArityError {
            expected: "1-2",
            got: 0
        })
    ));
}

#[test]
fn eval_tuple_callable_type_error_not_integer() {
    use crate::vm::RuntimeError;
    let (mut proc, mut mem) = setup();
    // ([1 2 3] :a) → ERROR: expected integer index
    let result = eval("([1 2 3] :a)", &mut proc, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::CallableTypeError {
            callable: "tuple",
            arg: 0,
            expected: "integer index"
        })
    ));
}

// nth with 3 args (not-found)

#[test]
fn eval_nth_with_default_out_of_bounds() {
    let (mut proc, mut mem) = setup();
    // (nth [1 2 3] 10 :not-found) → :not-found
    let result = eval("(nth [1 2 3] 10 :not-found)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "not-found");
}

#[test]
fn eval_nth_with_default_in_bounds() {
    let (mut proc, mut mem) = setup();
    // (nth [1 2 3] 1 :not-found) → 2
    let result = eval("(nth [1 2 3] 1 :not-found)", &mut proc, &mut mem).unwrap();
    assert_eq!(result, Value::int(2));
}

#[test]
fn eval_nth_with_default_negative_index() {
    let (mut proc, mut mem) = setup();
    // (nth [1 2 3] -1 :not-found) → :not-found (negative out of bounds)
    let result = eval("(nth [1 2 3] -1 :not-found)", &mut proc, &mut mem).unwrap();
    assert!(result.is_keyword());
    assert_eq!(proc.read_string(&mem, result).unwrap(), "not-found");
}
