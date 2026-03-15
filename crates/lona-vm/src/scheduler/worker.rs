// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Per-worker state for process scheduling.
//!
//! Each worker owns X registers for VM execution. In BEAM's model, registers
//! are per-scheduler (worker), not per-process. All processes running on a
//! worker share that worker's registers. Context switch saves/restores from
//! the process heap, not by copying registers between processes.
//!
//! The run queue is managed by the `Scheduler`, not the `Worker`, to enable
//! safe cross-worker access for work stealing via `SpinMutex`.

use crate::process::{ProcessId, WorkerId, X_REG_COUNT};
use crate::term::Term;

/// Per-worker state including X registers and current process.
///
/// Run queues are owned by the `Scheduler` (behind `SpinMutex`) so that
/// work stealing can access other workers' queues without aliased borrows.
pub struct Worker {
    /// Worker identifier.
    pub id: WorkerId,
    /// X registers (temporaries) for VM execution.
    ///
    /// These are shared by all processes running on this worker.
    /// Arguments are passed in X1..X(argc), return value in X0.
    pub x_regs: [Term; X_REG_COUNT],
    /// Currently running process (if any).
    pub current_pid: Option<ProcessId>,
}

impl Worker {
    /// Create a new worker with zeroed registers.
    #[must_use]
    pub const fn new(id: WorkerId) -> Self {
        Self {
            id,
            x_regs: [Term::NIL; X_REG_COUNT],
            current_pid: None,
        }
    }

    /// Reset X registers to nil.
    pub const fn reset_x_regs(&mut self) {
        self.x_regs = [Term::NIL; X_REG_COUNT];
    }
}
