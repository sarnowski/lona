// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Multi-worker scheduler that orchestrates `ProcessTable` + `Worker` to run
//! multiple processes across N workers.
//!
//! Each worker has its own run queue. Processes are spawned onto a specific
//! worker (default: the spawning worker, for locality). When a worker's queue
//! is empty, it steals from the busiest neighbor.

use super::{ProcessTable, Worker};
use crate::platform::MemorySpace;
use crate::process::{
    INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, ProcessId, ProcessStatus, WorkerId,
};
use crate::realm::Realm;
use crate::term::Term;
use crate::vm::{RunResult, RuntimeError, Vm};

/// Default number of workers (matches typical core count).
pub const DEFAULT_WORKER_COUNT: usize = 4;

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

/// Orchestrates multi-worker process scheduling within a realm.
///
/// Owns a `ProcessTable` for process storage and N `Worker`s for execution.
/// Uses the take/put-back pattern to avoid borrow conflicts between
/// the process table and VM execution.
pub struct Scheduler {
    process_table: ProcessTable,
    workers: [Worker; DEFAULT_WORKER_COUNT],
    /// Number of active workers (1 to `DEFAULT_WORKER_COUNT`).
    worker_count: usize,
    /// The `*ns*` var for bootstrapping new processes.
    ns_var: Term,
    /// The `lona.core` namespace for bootstrapping new processes.
    core_ns: Term,
}

impl Scheduler {
    /// Create a new scheduler with the default number of workers.
    #[must_use]
    pub fn new(ns_var: Term, core_ns: Term) -> Self {
        Self::with_worker_count(ns_var, core_ns, DEFAULT_WORKER_COUNT)
    }

    /// Create a new scheduler with a specific number of workers.
    ///
    /// # Panics
    ///
    /// Panics if `count` is 0 or exceeds `DEFAULT_WORKER_COUNT`.
    #[must_use]
    pub fn with_worker_count(ns_var: Term, core_ns: Term, count: usize) -> Self {
        assert!(
            count > 0 && count <= DEFAULT_WORKER_COUNT,
            "worker count must be 1..={DEFAULT_WORKER_COUNT}"
        );

        let workers = core::array::from_fn(|i| Worker::new(WorkerId(i as u8)));

        Self {
            process_table: ProcessTable::new(),
            workers,
            worker_count: count,
            ns_var,
            core_ns,
        }
    }

    /// Spawn a new process on a specific worker.
    ///
    /// Allocates process memory from the realm's pool, creates a `Process`,
    /// assigns a PID, bootstraps it with `*ns*`, and enqueues it on the
    /// given worker.
    ///
    /// Returns `None` if the process table is full or memory allocation fails.
    pub fn spawn_on<M: MemorySpace>(
        &mut self,
        realm: &mut Realm,
        mem: &mut M,
        chunk_addr: crate::Vaddr,
        parent_pid: ProcessId,
        worker_idx: usize,
    ) -> Option<ProcessId> {
        debug_assert!(
            worker_idx < self.worker_count,
            "worker_idx {worker_idx} >= worker_count {}",
            self.worker_count
        );
        let target_worker = worker_idx.min(self.worker_count - 1);

        let (young_base, old_base) =
            realm.allocate_process_memory(INITIAL_YOUNG_HEAP_SIZE, INITIAL_OLD_HEAP_SIZE)?;

        let mut process = crate::process::Process::new(
            young_base,
            INITIAL_YOUNG_HEAP_SIZE,
            old_base,
            INITIAL_OLD_HEAP_SIZE,
        );

        let (index, generation) = self.process_table.allocate()?;
        let pid = ProcessId::new(index, generation);

        process.pid = pid;
        process.parent_pid = parent_pid;
        process.worker_id = self.workers[target_worker].id;
        process.chunk_addr = Some(chunk_addr);
        process.ip = 0;

        process.bootstrap(self.ns_var, self.core_ns);

        // Allocate PID term so (self) works in spawned processes
        if let Some(pid_term) = process.alloc_term_pid(mem, index, generation) {
            process.pid_term = Some(pid_term);
        }

        self.process_table.insert(process);
        if !self.workers[target_worker].enqueue(pid) {
            self.process_table.remove(pid);
            return None;
        }

        Some(pid)
    }

    /// Spawn a new process on Worker 0 (convenience for single-worker usage).
    pub fn spawn<M: MemorySpace>(
        &mut self,
        realm: &mut Realm,
        mem: &mut M,
        chunk_addr: crate::Vaddr,
        parent_pid: ProcessId,
    ) -> Option<ProcessId> {
        self.spawn_on(realm, mem, chunk_addr, parent_pid, 0)
    }

    /// Execute one tick on a specific worker.
    ///
    /// Uses the take/put-back pattern: the process is extracted from the
    /// table during execution so `Vm::run` can receive `&mut ProcessTable`
    /// (needed for spawn/alive? intrinsics in later steps).
    pub fn tick_worker<M: MemorySpace>(
        &mut self,
        worker_idx: usize,
        realm: &mut Realm,
        mem: &mut M,
    ) -> TickResult {
        debug_assert!(worker_idx < self.worker_count, "worker_idx out of bounds");
        // Split borrow: access worker and process_table independently
        let Self {
            workers,
            process_table,
            ..
        } = self;

        let worker = &mut workers[worker_idx];

        let Some(pid) = worker.dequeue() else {
            return TickResult::Idle;
        };

        let Some(mut proc) = process_table.take(pid) else {
            debug_assert!(false, "stale PID dequeued: {pid:?}");
            return TickResult::Idle;
        };

        proc.status = ProcessStatus::Running;
        proc.reset_reductions();
        worker.current_pid = Some(pid);

        let result = Vm::run(worker, &mut proc, mem, realm, Some(process_table));

        worker.current_pid = None;

        match result {
            RunResult::Yielded => {
                proc.status = ProcessStatus::Ready;
                process_table.put_back(pid, proc);
                let enqueued = worker.enqueue(pid);
                debug_assert!(enqueued, "re-enqueue of yielding process must not fail");
                TickResult::Continued(pid)
            }
            RunResult::Completed(value) => {
                process_table.free_taken_slot(pid);
                TickResult::Completed(pid, value)
            }
            RunResult::Error(err) => {
                process_table.free_taken_slot(pid);
                TickResult::Error(pid, err)
            }
        }
    }

    /// Execute one tick on Worker 0 (convenience for single-worker patterns).
    pub fn tick<M: MemorySpace>(&mut self, realm: &mut Realm, mem: &mut M) -> TickResult {
        self.tick_worker(0, realm, mem)
    }

    /// Try to steal work for an idle worker from the busiest neighbor.
    ///
    /// Returns `true` if a process was stolen.
    pub(crate) fn try_steal_for(&mut self, idle_idx: usize) -> bool {
        // Find the busiest worker
        let mut busiest_idx = 0;
        let mut busiest_len = 0;

        for i in 0..self.worker_count {
            if i == idle_idx {
                continue;
            }
            let len = self.workers[i].run_queue.len();
            if len > busiest_len {
                busiest_len = len;
                busiest_idx = i;
            }
        }

        // Only steal if the busiest has at least 2 processes
        if busiest_len < 2 {
            return false;
        }

        if let Some(pid) = self.workers[busiest_idx].steal() {
            self.workers[idle_idx].enqueue(pid);
            true
        } else {
            false
        }
    }

    /// Run all processes to completion across all workers.
    ///
    /// Round-robins `tick_worker` across all workers. When a worker is idle,
    /// attempts work stealing. Stops when all queues are empty and no work
    /// was done in a full cycle.
    pub fn run_all<M: MemorySpace>(&mut self, realm: &mut Realm, mem: &mut M) {
        loop {
            let mut any_work = false;
            for idx in 0..self.worker_count {
                match self.tick_worker(idx, realm, mem) {
                    TickResult::Idle => {
                        if self.try_steal_for(idx) {
                            any_work = true;
                        }
                    }
                    _ => {
                        any_work = true;
                    }
                }
            }
            if !any_work {
                break;
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

    /// Check if a process is still alive.
    ///
    /// Returns `true` if the process exists in the table or is currently
    /// taken for execution.
    #[must_use]
    pub fn is_alive(&self, pid: ProcessId) -> bool {
        self.process_table.get(pid).is_some() || self.process_table.is_taken(pid)
    }

    /// Check if any worker has runnable processes.
    #[must_use]
    pub fn has_work(&self) -> bool {
        self.workers[..self.worker_count]
            .iter()
            .any(Worker::has_work)
    }

    /// Get the number of active workers.
    #[must_use]
    pub const fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Get a reference to the process table.
    #[must_use]
    pub const fn process_table(&self) -> &ProcessTable {
        &self.process_table
    }

    /// Get a mutable reference to the process table.
    pub const fn process_table_mut(&mut self) -> &mut ProcessTable {
        &mut self.process_table
    }

    /// Get a reference to a worker by index.
    #[must_use]
    pub const fn worker(&self, idx: usize) -> &Worker {
        &self.workers[idx]
    }

    /// Get a mutable reference to a worker by index.
    pub const fn worker_mut(&mut self, idx: usize) -> &mut Worker {
        &mut self.workers[idx]
    }
}
