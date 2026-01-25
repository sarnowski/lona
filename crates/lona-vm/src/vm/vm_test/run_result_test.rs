// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `RunResult` enum.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

#[test]
fn completed_is_terminal() {
    assert!(RunResult::Completed(Term::NIL).is_terminal());
    assert!(RunResult::Completed(int(42)).is_terminal());
}

#[test]
fn error_is_terminal() {
    assert!(RunResult::Error(RuntimeError::NoCode).is_terminal());
}

#[test]
fn yielded_is_not_terminal() {
    assert!(!RunResult::Yielded.is_terminal());
}

#[test]
fn yielded_is_yielded() {
    assert!(RunResult::Yielded.is_yielded());
}

#[test]
fn completed_is_not_yielded() {
    assert!(!RunResult::Completed(Term::NIL).is_yielded());
}

#[test]
fn error_is_not_yielded() {
    assert!(!RunResult::Error(RuntimeError::NoCode).is_yielded());
}
