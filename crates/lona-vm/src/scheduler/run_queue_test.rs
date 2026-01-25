// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `RunQueue`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::run_queue::{RUN_QUEUE_CAPACITY, RunQueue};
use crate::process::ProcessId;

#[test]
fn run_queue_new_is_empty() {
    let queue = RunQueue::new();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn run_queue_push_pop_fifo() {
    let mut queue = RunQueue::new();
    let pid1 = ProcessId::new(1, 0);
    let pid2 = ProcessId::new(2, 0);
    let pid3 = ProcessId::new(3, 0);

    queue.push_back(pid1);
    queue.push_back(pid2);
    queue.push_back(pid3);

    assert_eq!(queue.len(), 3);
    assert_eq!(queue.pop_front(), Some(pid1));
    assert_eq!(queue.pop_front(), Some(pid2));
    assert_eq!(queue.pop_front(), Some(pid3));
    assert_eq!(queue.pop_front(), None);
}

#[test]
fn run_queue_empty_pop_returns_none() {
    let mut queue = RunQueue::new();
    assert_eq!(queue.pop_front(), None);
}

#[test]
fn run_queue_full_rejects_push() {
    let mut queue = RunQueue::new();

    for i in 0..RUN_QUEUE_CAPACITY {
        assert!(queue.push_back(ProcessId::new(i as u32, 0)));
    }

    assert!(queue.is_full());
    assert!(!queue.push_back(ProcessId::new(999, 0)));
}

#[test]
fn run_queue_steal_from_back() {
    let mut queue = RunQueue::new();
    let pid1 = ProcessId::new(1, 0);
    let pid2 = ProcessId::new(2, 0);
    let pid3 = ProcessId::new(3, 0);

    queue.push_back(pid1);
    queue.push_back(pid2);
    queue.push_back(pid3);

    // Steal gets from back (most recently added)
    assert_eq!(queue.steal_back(), Some(pid3));
    // Pop gets from front (oldest)
    assert_eq!(queue.pop_front(), Some(pid1));
    assert_eq!(queue.pop_front(), Some(pid2));
    assert_eq!(queue.pop_front(), None);
}

#[test]
fn run_queue_wraparound() {
    let mut queue = RunQueue::new();

    // Add and remove to move head/tail forward
    for _ in 0..100 {
        queue.push_back(ProcessId::new(1, 0));
        queue.pop_front();
    }

    // Now add several and verify FIFO still works
    let pid1 = ProcessId::new(10, 0);
    let pid2 = ProcessId::new(20, 0);
    queue.push_back(pid1);
    queue.push_back(pid2);

    assert_eq!(queue.pop_front(), Some(pid1));
    assert_eq!(queue.pop_front(), Some(pid2));
}

#[test]
fn run_queue_steal_empty_returns_none() {
    let mut queue = RunQueue::new();
    assert_eq!(queue.steal_back(), None);
}

#[test]
fn run_queue_steal_single_element() {
    let mut queue = RunQueue::new();
    let pid = ProcessId::new(42, 0);

    queue.push_back(pid);
    assert_eq!(queue.steal_back(), Some(pid));
    assert!(queue.is_empty());
}

#[test]
fn run_queue_is_full() {
    let mut queue = RunQueue::new();
    assert!(!queue.is_full());

    for i in 0..RUN_QUEUE_CAPACITY {
        queue.push_back(ProcessId::new(i as u32, 0));
    }

    assert!(queue.is_full());
    assert_eq!(queue.len(), RUN_QUEUE_CAPACITY);
}
