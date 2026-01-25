// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Per-worker state for process scheduling.
//!
//! Each worker owns X registers for VM execution. In BEAM's model, registers
//! are per-scheduler (worker), not per-process. All processes running on a
//! worker share that worker's registers. Context switch saves/restores from
//! the process heap, not by copying registers between processes.

use super::RunQueue;
use crate::process::{ProcessId, WorkerId, X_REG_COUNT};
use crate::term::Term;

/// Per-worker state including X registers, run queue, and current process.
pub struct Worker {
    /// Worker identifier.
    pub id: WorkerId,
    /// X registers (temporaries) for VM execution.
    ///
    /// These are shared by all processes running on this worker.
    /// Arguments are passed in X1..X(argc), return value in X0.
    pub x_regs: [Term; X_REG_COUNT],
    /// Run queue for this worker.
    pub run_queue: RunQueue,
    /// Currently running process (if any).
    pub current_pid: Option<ProcessId>,
}

impl Worker {
    /// Create a new worker with an empty run queue and zeroed registers.
    #[must_use]
    pub const fn new(id: WorkerId) -> Self {
        Self {
            id,
            x_regs: [Term::NIL; X_REG_COUNT],
            run_queue: RunQueue::new(),
            current_pid: None,
        }
    }

    /// Reset X registers to nil.
    pub const fn reset_x_regs(&mut self) {
        self.x_regs = [Term::NIL; X_REG_COUNT];
    }

    /// Enqueue process on this worker's run queue.
    pub const fn enqueue(&mut self, pid: ProcessId) -> bool {
        self.run_queue.push_back(pid)
    }

    /// Dequeue next runnable process.
    pub const fn dequeue(&mut self) -> Option<ProcessId> {
        self.run_queue.pop_front()
    }

    /// Check if worker has runnable processes.
    #[must_use]
    pub const fn has_work(&self) -> bool {
        !self.run_queue.is_empty()
    }

    /// Allow stealing from this worker.
    pub const fn steal(&mut self) -> Option<ProcessId> {
        self.run_queue.steal_back()
    }
}
