// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Scheduler that orchestrates `ProcessTable` + `Worker` to run multiple
//! processes in round-robin order.
//!
//! The scheduler dequeues a process, runs it for one time slice via
//! `Vm::run`, and handles the result (re-enqueue on yield, remove on
//! completion or error).

use super::{ProcessTable, Worker};
use crate::platform::MemorySpace;
use crate::process::{
    INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, ProcessId, ProcessStatus, WorkerId,
};
use crate::realm::Realm;
use crate::term::Term;
use crate::vm::{RunResult, RuntimeError, Vm};

/// Result of a single scheduler tick.
#[derive(Debug)]
#[must_use]
pub enum TickResult {
    /// No processes in the run queue.
    Idle,
    /// A process executed and yielded (re-enqueued).
    Continued(ProcessId),
    /// A process completed with a return value.
    Completed(ProcessId, Term),
    /// A process encountered a runtime error.
    Error(ProcessId, RuntimeError),
}

/// Orchestrates process scheduling within a realm.
///
/// Owns a `ProcessTable` for process storage and a `Worker` for execution.
/// The scheduler picks processes from the run queue, runs them for one
/// time slice, and handles the result.
pub struct Scheduler {
    process_table: ProcessTable,
    worker: Worker,
    /// The `*ns*` var for bootstrapping new processes.
    ns_var: Term,
    /// The `lona.core` namespace for bootstrapping new processes.
    core_ns: Term,
}

impl Scheduler {
    /// Create a new scheduler with an empty process table and `Worker(0)`.
    #[must_use]
    pub fn new(ns_var: Term, core_ns: Term) -> Self {
        Self {
            process_table: ProcessTable::new(),
            worker: Worker::new(WorkerId(0)),
            ns_var,
            core_ns,
        }
    }

    /// Spawn a new process that will execute the bytecode at `chunk_addr`.
    ///
    /// Allocates process memory from the realm's pool, creates a `Process`,
    /// assigns a PID, bootstraps it with `*ns*`, and enqueues it.
    ///
    /// Returns `None` if the process table is full or memory allocation fails.
    pub fn spawn(
        &mut self,
        realm: &mut Realm,
        chunk_addr: crate::Vaddr,
        parent_pid: ProcessId,
    ) -> Option<ProcessId> {
        // Allocate heap memory from the realm's pool
        let (young_base, old_base) =
            realm.allocate_process_memory(INITIAL_YOUNG_HEAP_SIZE, INITIAL_OLD_HEAP_SIZE)?;

        let mut process = crate::process::Process::new(
            young_base,
            INITIAL_YOUNG_HEAP_SIZE,
            old_base,
            INITIAL_OLD_HEAP_SIZE,
        );

        // Allocate a slot in the process table
        let (index, generation) = self.process_table.allocate()?;
        let pid = ProcessId::new(index, generation);

        // Set up process identity and execution state
        process.pid = pid;
        process.parent_pid = parent_pid;
        process.worker_id = self.worker.id;
        process.chunk_addr = Some(chunk_addr);
        process.ip = 0;

        // Bootstrap with *ns* binding
        process.bootstrap(self.ns_var, self.core_ns);

        // Insert into table and enqueue
        self.process_table.insert(process);
        if !self.worker.enqueue(pid) {
            // Run queue full — roll back the table insertion
            self.process_table.remove(pid);
            return None;
        }

        Some(pid)
    }

    /// Execute one scheduling tick: dequeue a process, run it, handle the result.
    ///
    /// Returns what happened during this tick so callers can react to
    /// process lifecycle events.
    pub fn tick<M: MemorySpace>(&mut self, realm: &mut Realm, mem: &mut M) -> TickResult {
        let Some(pid) = self.worker.dequeue() else {
            return TickResult::Idle;
        };

        let Some(proc) = self.process_table.get_mut(pid) else {
            // Every PID in the run queue must exist in the process table.
            // If this fires, an invariant has been violated.
            debug_assert!(false, "stale PID dequeued: {pid:?}");
            return TickResult::Idle;
        };

        proc.status = ProcessStatus::Running;
        proc.reset_reductions();
        self.worker.current_pid = Some(pid);

        let result = Vm::run(&mut self.worker, proc, mem, realm);

        // Update process status while we still hold the borrow
        if matches!(result, RunResult::Yielded) {
            proc.status = ProcessStatus::Ready;
        }
        // `proc` borrow ends here

        self.worker.current_pid = None;

        match result {
            RunResult::Yielded => {
                // Re-enqueue must succeed: we just dequeued from this queue
                // with no intervening enqueue, so capacity is guaranteed.
                let enqueued = self.worker.enqueue(pid);
                debug_assert!(enqueued, "re-enqueue of yielding process must not fail");
                TickResult::Continued(pid)
            }
            RunResult::Completed(value) => {
                self.process_table.remove(pid);
                TickResult::Completed(pid, value)
            }
            RunResult::Error(err) => {
                self.process_table.remove(pid);
                TickResult::Error(pid, err)
            }
        }
    }

    /// Run all processes to completion.
    ///
    /// Loops `tick` until the run queue is empty. Completed values and errors
    /// are discarded. Use `tick` directly to observe process outcomes.
    pub fn run_all<M: MemorySpace>(&mut self, realm: &mut Realm, mem: &mut M) {
        loop {
            match self.tick(realm, mem) {
                TickResult::Idle => break,
                TickResult::Continued(_)
                | TickResult::Completed(_, _)
                | TickResult::Error(_, _) => {}
            }
        }
    }

    /// Number of active processes in the table.
    #[must_use]
    pub const fn process_count(&self) -> usize {
        self.process_table.count()
    }

    /// Get a reference to a process by PID.
    #[must_use]
    pub const fn get_process(&self, pid: ProcessId) -> Option<&crate::process::Process> {
        self.process_table.get(pid)
    }

    /// Check if a process is still alive in the table.
    #[must_use]
    pub const fn is_alive(&self, pid: ProcessId) -> bool {
        self.process_table.get(pid).is_some()
    }

    /// Check if the scheduler has runnable processes.
    #[must_use]
    pub const fn has_work(&self) -> bool {
        self.worker.has_work()
    }
}
