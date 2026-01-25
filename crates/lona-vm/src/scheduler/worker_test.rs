// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `Worker`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::Worker;
use crate::process::{ProcessId, WorkerId};

#[test]
fn worker_new() {
    let worker = Worker::new(WorkerId(0));
    assert_eq!(worker.id, WorkerId(0));
    assert!(!worker.has_work());
    assert!(worker.current_pid.is_none());
}

#[test]
fn worker_enqueue_dequeue() {
    let mut worker = Worker::new(WorkerId(0));
    let pid = ProcessId::new(1, 0);

    assert!(worker.enqueue(pid));
    assert!(worker.has_work());
    assert_eq!(worker.dequeue(), Some(pid));
    assert!(!worker.has_work());
}

#[test]
fn worker_steal() {
    let mut worker = Worker::new(WorkerId(0));
    let pid1 = ProcessId::new(1, 0);
    let pid2 = ProcessId::new(2, 0);

    worker.enqueue(pid1);
    worker.enqueue(pid2);

    // Steal takes from back
    assert_eq!(worker.steal(), Some(pid2));
    assert_eq!(worker.dequeue(), Some(pid1));
    assert!(!worker.has_work());
}

#[test]
fn worker_steal_empty() {
    let mut worker = Worker::new(WorkerId(0));
    assert_eq!(worker.steal(), None);
}

#[test]
fn worker_current_pid() {
    let mut worker = Worker::new(WorkerId(0));

    // Initially no current process
    assert!(worker.current_pid.is_none());

    // Set current process
    let pid = ProcessId::new(5, 0);
    worker.current_pid = Some(pid);
    assert_eq!(worker.current_pid, Some(pid));

    // Clear current process
    worker.current_pid = None;
    assert!(worker.current_pid.is_none());
}
