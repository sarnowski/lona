// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Macro introspection integration tests.
//!
//! Tests for macro?, macroexpand-1, and macroexpand in the full seL4 environment.

use crate::{println, repl};
use lona_core::value::Value;
use lona_test::Status;

/// Tests macro? predicate returns true for defined macros.
pub fn test_macro_predicate_true() -> Status {
    let mut repl_instance = repl::Repl::new();

    // Define a simple macro
    match repl_instance.eval("(defmacro when [test body] (list 'if test body nil))") {
        Ok(_) => {}
        Err(ref msg) => {
            println!("test_macro_predicate_true: failed to define macro: {msg}");
            return Status::Fail;
        }
    }

    // Check that macro? returns true
    match repl_instance.eval("(macro? 'when)") {
        Ok(Value::Bool(true)) => Status::Pass,
        Ok(ref other) => {
            println!("test_macro_predicate_true: expected true, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macro_predicate_true: error: {msg}");
            Status::Fail
        }
    }
}

/// Tests macro? predicate returns false for non-macros.
pub fn test_macro_predicate_false() -> Status {
    let mut repl_instance = repl::Repl::new();

    // Check that macro? returns false for undefined symbol
    match repl_instance.eval("(macro? 'not-a-macro)") {
        Ok(Value::Bool(false)) => Status::Pass,
        Ok(ref other) => {
            println!("test_macro_predicate_false: expected false, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macro_predicate_false: error: {msg}");
            Status::Fail
        }
    }
}

/// Tests macroexpand-1 expands a macro call once.
pub fn test_macroexpand_1() -> Status {
    let mut repl_instance = repl::Repl::new();

    // Define an identity macro that returns its argument
    match repl_instance.eval("(defmacro identity [x] x)") {
        Ok(_) => {}
        Err(ref msg) => {
            println!("test_macroexpand_1: failed to define macro: {msg}");
            return Status::Fail;
        }
    }

    // macroexpand-1 should expand once
    match repl_instance.eval("(macroexpand-1 '(identity 42))") {
        Ok(Value::Integer(ref n)) if n.to_i64() == Some(42_i64) => Status::Pass,
        Ok(ref other) => {
            println!("test_macroexpand_1: expected 42, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macroexpand_1: error: {msg}");
            Status::Fail
        }
    }
}

/// Tests macroexpand-1 returns non-macro form unchanged.
pub fn test_macroexpand_1_non_macro() -> Status {
    let mut repl_instance = repl::Repl::new();

    // macroexpand-1 on a non-macro form should return it unchanged
    match repl_instance.eval("(macroexpand-1 '(+ 1 2))") {
        Ok(Value::List(ref list)) if list.len() == 3 => Status::Pass,
        Ok(ref other) => {
            println!("test_macroexpand_1_non_macro: expected list, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macroexpand_1_non_macro: error: {msg}");
            Status::Fail
        }
    }
}

/// Tests macroexpand fully expands nested macro calls.
pub fn test_macroexpand() -> Status {
    let mut repl_instance = repl::Repl::new();

    // Define an identity macro
    match repl_instance.eval("(defmacro pass-through [x] x)") {
        Ok(_) => {}
        Err(ref msg) => {
            println!("test_macroexpand: failed to define macro: {msg}");
            return Status::Fail;
        }
    }

    // macroexpand should fully expand
    match repl_instance.eval("(macroexpand '(pass-through 99))") {
        Ok(Value::Integer(ref n)) if n.to_i64() == Some(99_i64) => Status::Pass,
        Ok(ref other) => {
            println!("test_macroexpand: expected 99, got {other:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_macroexpand: error: {msg}");
            Status::Fail
        }
    }
}
