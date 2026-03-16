// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Integration tests and error handling tests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::intrinsics::IntrinsicError;
use crate::term::Term;
use crate::vm::RuntimeError;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

// --- Error tests ---

#[test]
fn eval_div_by_zero() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(/ 10 0)", &mut proc, &mut realm, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::Intrinsic(IntrinsicError::DivisionByZero))
    ));
}

#[test]
fn eval_type_error() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval("(+ true 2)", &mut proc, &mut realm, &mut mem);
    assert!(matches!(
        result,
        Err(RuntimeError::Intrinsic(IntrinsicError::TypeError { .. }))
    ));
}

// --- Integration tests matching ROADMAP test cases ---

#[test]
fn roadmap_test_cases() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    assert_eq!(
        eval("42", &mut proc, &mut realm, &mut mem).unwrap(),
        int(42)
    );
    assert_eq!(
        eval("(+ 1 2)", &mut proc, &mut realm, &mut mem).unwrap(),
        int(3)
    );
    assert_eq!(
        eval("(< 1 2)", &mut proc, &mut realm, &mut mem).unwrap(),
        Term::TRUE
    );
    assert_eq!(
        eval("(>= 5 5)", &mut proc, &mut realm, &mut mem).unwrap(),
        Term::TRUE
    );
    assert_eq!(
        eval("(not true)", &mut proc, &mut realm, &mut mem).unwrap(),
        Term::FALSE
    );
    assert_eq!(
        eval("(nil? nil)", &mut proc, &mut realm, &mut mem).unwrap(),
        Term::TRUE
    );
    assert_eq!(
        eval("(integer? 42)", &mut proc, &mut realm, &mut mem).unwrap(),
        Term::TRUE
    );
    assert_eq!(
        eval("(string? \"hello\")", &mut proc, &mut realm, &mut mem).unwrap(),
        Term::TRUE
    );
    assert_eq!(
        eval("(mod 17 5)", &mut proc, &mut realm, &mut mem).unwrap(),
        int(2)
    );
}

#[test]
fn roadmap_str_test() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let result = eval(
        "(str \"hello\" \" \" \"world\")",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    let s = proc.read_term_string(&mem, result).unwrap();
    assert_eq!(s, "hello world");
}

// --- def special form tests ---

#[test]
fn def_basic_value() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a var with an integer value
    let var = eval("(def x 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_var(&mem, var));

    // Access the var's value
    let result = eval("x", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(42));
}

#[test]
fn def_with_function() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a function
    eval(
        "(def inc1 (fn* [n] (+ n 1)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    // Call the function
    let result = eval("(inc1 5)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(6));
}

#[test]
fn def_late_binding() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a function
    eval("(def f (fn* [x] x))", &mut proc, &mut realm, &mut mem).unwrap();
    let result = eval("(f 1)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(1));

    // Redefine the function
    eval(
        "(def f (fn* [x] (+ x 10)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    let result = eval("(f 1)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(11));
}

#[test]
fn def_with_closure() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a closure factory
    eval(
        "(def make-adder (fn* [n] (fn* [x] (+ x n))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    // Create a closure that adds 5
    eval("(def add5 (make-adder 5))", &mut proc, &mut realm, &mut mem).unwrap();

    // Use the closure
    let result = eval("(add5 10)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(15));
}

#[test]
fn def_var_access() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a var
    eval("(def x 42)", &mut proc, &mut realm, &mut mem).unwrap();

    // Access the var object via #' syntax
    let var = eval("#'x", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_var(&mem, var));

    // Get the var's value via var-get
    let result = eval("(var-get #'x)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(42));
}

#[test]
fn def_unbound_var_error() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define an unbound var
    eval("(def y)", &mut proc, &mut realm, &mut mem).unwrap();

    // Accessing unbound var should error
    let result = eval("y", &mut proc, &mut realm, &mut mem);
    assert!(result.is_err());
}

#[test]
fn def_multiple_vars() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define multiple vars
    eval("(def a 1)", &mut proc, &mut realm, &mut mem).unwrap();
    eval("(def b 2)", &mut proc, &mut realm, &mut mem).unwrap();
    eval("(def c 3)", &mut proc, &mut realm, &mut mem).unwrap();

    // Use them in expression
    let result = eval("(+ a (+ b c))", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(6));
}

#[test]
fn def_string_value() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a var with a string value
    eval("(def greeting \"hello\")", &mut proc, &mut realm, &mut mem).unwrap();

    // Access the var's value
    let result = eval("greeting", &mut proc, &mut realm, &mut mem).unwrap();
    let s = proc.read_term_string(&mem, result).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn def_function_calls_another_def() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a function
    eval(
        "(def add1 (fn* [x] (+ x 1)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    // Define another function that calls the first
    eval(
        "(def add2 (fn* [x] (add1 (add1 x))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    // Call the second function
    let result = eval("(add2 5)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(7));
}

#[test]
fn def_function_passes_parameter_to_another_def() {
    // This tests the parameter clobbering bug fix: when a function calls another
    // def'd function and passes its parameter as an argument, the parameter value
    // must be preserved across the VAR_GET intrinsic call.
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define identity function
    eval("(def id (fn* [x] x))", &mut proc, &mut realm, &mut mem).unwrap();

    // Define a wrapper that passes its parameter to id
    eval(
        "(def call-id (fn* [y] (id y)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    // This MUST return 42, not the var #'lona.core/id
    let result = eval("(call-id 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(42));
}

#[test]
fn def_nested_function_calls_with_parameters() {
    // More complex test: multiple levels of function calls with parameters
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    eval(
        "(def double (fn* [n] (+ n n)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    eval(
        "(def quadruple (fn* [n] (double (double n))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    let result = eval("(quadruple 3)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(12));
}

#[test]
fn def_function_multiple_parameters_passed_to_another() {
    // Test that all parameters are preserved when calling another function
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    eval(
        "(def sum3 (fn* [a b c] (+ a (+ b c))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    eval(
        "(def call-sum3 (fn* [x y z] (sum3 x y z)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    let result = eval("(call-sum3 1 2 3)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(6));
}

#[test]
fn def_process_bound_var() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a process-bound var
    eval(
        "(def ^:process-bound *counter* 0)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    // Access the var's value
    let result = eval("*counter*", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(0));

    // Redefine (sets process binding)
    eval("(def *counter* 1)", &mut proc, &mut realm, &mut mem).unwrap();

    // Value should be updated
    let result = eval("*counter*", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(1));
}

// TODO: Phase 8.8 - This test needs investigation after Value removal is complete.
// The metadata attachment via DEF_META intrinsic may not be working correctly
// after the term representation refactoring.
#[test]
#[ignore = "Phase 8.8: Var metadata storage/retrieval needs investigation"]
fn def_with_metadata() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a var with metadata
    eval(
        "(def ^%{:doc \"A test value\"} documented-var 42)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    // Access the var's value
    let result = eval("documented-var", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(42));

    // Access the var's metadata
    let meta = eval("(meta #'documented-var)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_map(&mem, meta), "metadata should be a map");

    // Check that :doc key is present in metadata
    let doc_value = eval(
        "(get (meta #'documented-var) :doc)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    let doc_str = proc.read_term_string(&mem, doc_value).unwrap();
    assert_eq!(doc_str, "A test value");
}

// TODO: Phase 8.8 - Process-bound checking was intentionally removed from the Term model.
// The intern_or_get_var function no longer has access to var flags to detect conflicts.
// This needs to be reimplemented with the new HeapVar structure.
#[test]
#[ignore = "Phase 8.8: Process-bound conflict checking needs reimplementation"]
fn def_process_bound_conflict() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // First define a regular (non-process-bound) var
    eval("(def regular-var 42)", &mut proc, &mut realm, &mut mem).unwrap();

    // Trying to redefine it as process-bound should fail at compile time
    // The compiler should detect that the var already exists without process-bound flag
    let result = eval(
        "(def ^:process-bound regular-var 100)",
        &mut proc,
        &mut realm,
        &mut mem,
    );
    assert!(
        result.is_err(),
        "redefining non-process-bound var as process-bound should fail"
    );
}

#[test]
fn def_qualified_symbol_in_call() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Use fully qualified lona.core/+ in an expression
    let result = eval("(lona.core/+ 1 2)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(3));

    // Multiple qualified intrinsics in nested expression
    let result = eval(
        "(lona.core/+ (lona.core/* 2 3) (lona.core/- 10 5))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, int(11)); // (+ 6 5) = 11
}

#[test]
fn def_qualified_symbol_in_def() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a function that uses qualified intrinsics
    eval(
        "(def add-mul (fn* [a b c] (lona.core/+ a (lona.core/* b c))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    let result = eval("(add-mul 1 2 3)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(7)); // 1 + (2 * 3) = 7
}

#[test]
fn qualified_lona_process_self() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Set a concrete pid_term so the result is distinguishable from NIL
    let pid_term = proc.alloc_term_pid(&mut mem, 7, 1).expect("pid alloc");
    proc.pid_term = Some(pid_term);

    // lona.process/self should resolve to the same intrinsic as self
    let qualified = eval("(lona.process/self)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(qualified, pid_term);

    let unqualified = eval("(self)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(unqualified, pid_term);
    assert_eq!(qualified, unqualified);
}

#[test]
fn qualified_lona_process_pid_predicate() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // lona.process/pid? should resolve via qualified namespace lookup
    let result = eval("(lona.process/pid? 42)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, Term::FALSE);

    // lona.process/ref? should also resolve
    let result = eval(
        "(lona.process/ref? :hello)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert_eq!(result, Term::FALSE);
}
