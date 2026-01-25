// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `ProcessId` and `WorkerId` types.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{ProcessId, WorkerId};

// =============================================================================
// ProcessId Tests
// =============================================================================

#[test]
fn process_id_null() {
    assert!(ProcessId::NULL.is_null());
    assert!(!ProcessId::new(0, 0).is_null());
}

#[test]
fn process_id_equality() {
    let a = ProcessId::new(5, 1);
    let b = ProcessId::new(5, 1);
    let c = ProcessId::new(5, 2); // Different generation
    let d = ProcessId::new(6, 1); // Different index

    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
}

#[test]
fn process_id_index() {
    let pid = ProcessId::new(42, 7);
    assert_eq!(pid.index(), 42);
}

#[test]
fn process_id_generation() {
    let pid = ProcessId::new(42, 7);
    assert_eq!(pid.generation(), 7);
}

#[test]
fn process_id_null_index() {
    // NULL uses u32::MAX as index
    assert_eq!(ProcessId::NULL.index(), u32::MAX as usize);
}

#[test]
fn process_id_debug() {
    let pid = ProcessId::new(5, 1);
    let debug_str = format!("{pid:?}");
    assert!(debug_str.contains('5'));
    assert!(debug_str.contains('1'));
}

#[test]
fn process_id_copy() {
    let a = ProcessId::new(1, 2);
    let b = a; // Copy
    assert_eq!(a, b);
}

// =============================================================================
// WorkerId Tests
// =============================================================================

#[test]
fn worker_id_creation() {
    let worker = WorkerId(5);
    assert_eq!(worker.0, 5);
}

#[test]
fn worker_id_equality() {
    let a = WorkerId(3);
    let b = WorkerId(3);
    let c = WorkerId(4);

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn worker_id_copy() {
    let a = WorkerId(7);
    let b = a; // Copy
    assert_eq!(a, b);
}

#[test]
fn worker_id_debug() {
    let worker = WorkerId(12);
    let debug_str = format!("{worker:?}");
    assert!(debug_str.contains("12"));
}
