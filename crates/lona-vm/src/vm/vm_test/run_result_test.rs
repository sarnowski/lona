// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `RunResult` enum.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn completed_is_terminal() {
    assert!(RunResult::Completed(Value::Nil).is_terminal());
    assert!(RunResult::Completed(Value::int(42)).is_terminal());
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
    assert!(!RunResult::Completed(Value::Nil).is_yielded());
}

#[test]
fn error_is_not_yielded() {
    assert!(!RunResult::Error(RuntimeError::NoCode).is_yielded());
}
