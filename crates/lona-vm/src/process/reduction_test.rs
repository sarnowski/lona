// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for reduction counting.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// Create a test process with default heap configuration.
fn create_test_process() -> Process {
    Process::new(
        1,
        Vaddr::new(0x1000),
        INITIAL_YOUNG_HEAP_SIZE,
        Vaddr::new(0x10000),
        INITIAL_OLD_HEAP_SIZE,
    )
}

#[test]
fn reset_reductions_sets_to_max() {
    let mut proc = create_test_process();
    assert_eq!(proc.reductions, 0);

    proc.reset_reductions();
    assert_eq!(proc.reductions, MAX_REDUCTIONS);
}

#[test]
fn consume_reductions_success() {
    let mut proc = create_test_process();
    proc.reductions = 100;

    assert!(proc.consume_reductions(30));
    assert_eq!(proc.reductions, 70);
    assert_eq!(proc.total_reductions, 30);

    assert!(proc.consume_reductions(70));
    assert_eq!(proc.reductions, 0);
    assert_eq!(proc.total_reductions, 100);
}

#[test]
fn consume_reductions_exhausted() {
    let mut proc = create_test_process();
    proc.reductions = 10;

    // Trying to consume more than available returns false
    assert!(!proc.consume_reductions(20));
    assert_eq!(proc.reductions, 0);
    assert!(proc.should_yield());
    // Should still have consumed the remaining 10
    assert_eq!(proc.total_reductions, 10);
}

#[test]
fn should_yield_when_zero() {
    let mut proc = create_test_process();
    proc.reductions = 1;
    assert!(!proc.should_yield());

    proc.reductions = 0;
    assert!(proc.should_yield());
}

#[test]
fn total_reductions_accumulates() {
    let mut proc = create_test_process();
    proc.total_reductions = 1000;
    proc.reductions = 50;

    proc.consume_reductions(25);
    proc.consume_reductions(25);

    assert_eq!(proc.total_reductions, 1050);
}

#[test]
fn consume_reductions_exact_budget() {
    let mut proc = create_test_process();
    proc.reductions = 50;

    // Consuming exactly the budget should succeed
    assert!(proc.consume_reductions(50));
    assert_eq!(proc.reductions, 0);
    assert!(proc.should_yield());
}

#[test]
fn total_reductions_wraps_on_overflow() {
    let mut proc = create_test_process();
    proc.total_reductions = u64::MAX - 5;
    proc.reductions = 10;

    proc.consume_reductions(10);
    // Should wrap around
    assert_eq!(proc.total_reductions, 4); // (MAX - 5 + 10) wraps to 4
}
