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
    assert!(worker.current_pid.is_none());
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

#[test]
fn worker_reset_x_regs() {
    let mut worker = Worker::new(WorkerId(0));
    worker.x_regs[0] = crate::term::Term::TRUE;
    worker.x_regs[1] = crate::term::Term::small_int(42).unwrap();
    worker.reset_x_regs();
    assert_eq!(worker.x_regs[0], crate::term::Term::NIL);
    assert_eq!(worker.x_regs[1], crate::term::Term::NIL);
}
