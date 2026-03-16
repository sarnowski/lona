// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Multi-worker scheduler that orchestrates `ProcessTable` + run queues to run
//! multiple processes across N workers.
//!
//! Each worker has its own run queue (behind `SpinMutex` for work stealing).
//! Processes are spawned onto a specific worker (default: Worker 0, for locality).
//! When a worker's queue is empty, it steals from the busiest neighbor.
//!
//! The `ProcessTable` is behind `SpinMutex` to enable concurrent access from
//! multiple workers. All `Scheduler` methods take `&self` (not `&mut self`)
//! and acquire locks as needed.

extern crate alloc;

use core::sync::atomic::{AtomicU64, Ordering};

use super::{ProcessTable, RunQueue, Worker};
use crate::platform::MemorySpace;
use crate::process::{
    INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process, ProcessId, ProcessStatus, WorkerId,
};
use crate::realm::Realm;
use crate::sync::{SpinMutex, SpinRwLock};
use crate::term::Term;
use crate::vm::{RunResult, RuntimeError, Vm};

use super::exit_propagation::ExitReason;

/// Default number of workers (matches typical core count).
pub const DEFAULT_WORKER_COUNT: usize = 4;

/// Minimum number of processes a worker must have before work can be stolen.
///
/// Prevents ping-pong stealing when a worker has only one process.
const MIN_STEAL_THRESHOLD: usize = 2;

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
    /// A process explicitly called `exit(reason)`.
    Exited(ProcessId, Term),
    /// A process encountered a runtime error.
    Error(ProcessId, RuntimeError),
}

/// Orchestrates multi-worker process scheduling within a realm.
///
/// Owns a `ProcessTable` (behind `SpinMutex`) for process storage and
/// N run queues (each behind `SpinMutex`) for work distribution.
///
/// Workers (with their X registers and per-TCB state) are NOT owned by the
/// Scheduler. Each seL4 TCB owns its own `Worker` and passes it to
/// `tick_worker`. This enables safe concurrent access: `&self` methods
/// lock only what they need.
pub struct Scheduler {
    pub(super) process_table: SpinMutex<ProcessTable>,
    run_queues: [SpinMutex<RunQueue>; DEFAULT_WORKER_COUNT],
    /// Number of active workers (1 to `DEFAULT_WORKER_COUNT`).
    worker_count: usize,
    /// The `*ns*` var for bootstrapping new processes.
    ns_var: Term,
    /// The `lona.core` namespace for bootstrapping new processes.
    core_ns: Term,
    /// Counter for generating unique reference IDs (for monitors).
    next_ref_id: AtomicU64,
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

        Self {
            process_table: SpinMutex::new(ProcessTable::new()),
            run_queues: core::array::from_fn(|_| SpinMutex::new(RunQueue::new())),
            worker_count: count,
            ns_var,
            core_ns,
            next_ref_id: AtomicU64::new(1),
        }
    }

    /// Ensure the process table has at least one free slot.
    ///
    /// Uses lock ordering (PT → Realm → PT) to avoid deadlock when growing:
    /// 1. Check free list (PT lock) → release
    /// 2. Allocate segment memory (Realm lock) → release
    /// 3. Add segment to table (PT lock) → release
    ///
    /// In tests, segments are allocated from the system heap.
    /// On seL4, segments come from the `ProcessPool` via the Realm.
    ///
    /// Returns `true` if capacity is available, `false` if growth failed.
    pub fn ensure_capacity(&self, realm: &SpinRwLock<Realm>) -> bool {
        let needs_growth = {
            let pt = self.process_table.lock();
            !pt.has_free_slots()
        };
        if !needs_growth {
            return true;
        }

        #[cfg(test)]
        let segment_ptr = {
            let _ = realm;
            ProcessTable::alloc_test_segment()
        };

        #[cfg(not(test))]
        let segment_ptr = {
            use super::process_table::{SEGMENT_SIZE, Slot};
            let size = core::mem::size_of::<Slot>() * SEGMENT_SIZE;
            let mut realm_guard = realm.write();
            let Some(vaddr) = realm_guard.pool_mut().allocate_with_growth(size, 8) else {
                return false;
            };
            vaddr.as_u64() as *mut Slot
        };

        let mut pt = self.process_table.lock();
        // SAFETY: segment_ptr points to exclusively-owned memory of SEGMENT_SIZE
        // Slots, allocated from either the system heap (tests) or ProcessPool (seL4).
        unsafe { pt.grow_segment(segment_ptr) };
        true
    }

    /// Spawn a new process on a specific worker.
    ///
    /// Allocates process memory from the realm's pool, creates a `Process`,
    /// assigns a PID, bootstraps it with `*ns*`, and enqueues it on the
    /// given worker's run queue.
    ///
    /// Returns `None` if the process table is full or memory allocation fails.
    pub fn spawn_on<M: MemorySpace>(
        &self,
        realm: &SpinRwLock<Realm>,
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

        // Grow process table if needed (lock ordering: PT → Realm → PT)
        if !self.ensure_capacity(realm) {
            return None;
        }

        let (young_base, old_base) = {
            let mut realm_guard = realm.write();
            realm_guard.allocate_process_memory(INITIAL_YOUNG_HEAP_SIZE, INITIAL_OLD_HEAP_SIZE)?
        };

        let mut process = crate::process::Process::new(
            young_base,
            INITIAL_YOUNG_HEAP_SIZE,
            old_base,
            INITIAL_OLD_HEAP_SIZE,
        );

        let (index, generation) = {
            let mut pt = self.process_table.lock();
            pt.allocate()?
        };
        let pid = ProcessId::new(index, generation);

        process.pid = pid;
        process.parent_pid = parent_pid;
        process.worker_id = WorkerId(target_worker as u8);
        process.chunk_addr = Some(chunk_addr);
        process.ip = 0;

        process.bootstrap(self.ns_var, self.core_ns);

        // Allocate PID term so (self) works in spawned processes
        if let Some(pid_term) = process.alloc_term_pid(mem, index, generation) {
            process.pid_term = Some(pid_term);
        }

        {
            let mut pt = self.process_table.lock();
            pt.insert(process);
        }

        {
            let mut rq = self.run_queues[target_worker].lock();
            if !rq.push_back(pid) {
                let mut pt = self.process_table.lock();
                pt.remove(pid);
                return None;
            }
        }

        Some(pid)
    }

    /// Spawn a new process on Worker 0 (convenience for single-worker usage).
    pub fn spawn<M: MemorySpace>(
        &self,
        realm: &SpinRwLock<Realm>,
        mem: &mut M,
        chunk_addr: crate::Vaddr,
        parent_pid: ProcessId,
    ) -> Option<ProcessId> {
        self.spawn_on(realm, mem, chunk_addr, parent_pid, 0)
    }

    /// Enqueue a PID on a specific worker's run queue.
    ///
    /// Used by special intrinsics (spawn) that need to enqueue a newly
    /// created process.
    pub(crate) fn enqueue_on(&self, worker_idx: usize, pid: ProcessId) -> bool {
        debug_assert!(
            worker_idx < self.worker_count,
            "worker_idx {worker_idx} >= worker_count {}",
            self.worker_count
        );
        let target = worker_idx.min(self.worker_count - 1);
        let mut rq = self.run_queues[target].lock();
        rq.push_back(pid)
    }

    /// Execute one tick on a specific worker.
    ///
    /// Uses the take/put-back pattern: the process is extracted from the
    /// table during execution so `Vm::run` can access the `ProcessTable`
    /// (via `&Scheduler`) for spawn/alive? intrinsics.
    pub fn tick_worker<M: MemorySpace>(
        &self,
        worker: &mut Worker,
        realm: &SpinRwLock<Realm>,
        mem: &mut M,
    ) -> TickResult {
        let worker_idx = worker.id.0 as usize;
        debug_assert!(worker_idx < self.worker_count, "worker_idx out of bounds");

        let pid = {
            let mut rq = self.run_queues[worker_idx].lock();
            rq.pop_front()
        };

        let Some(pid) = pid else {
            return TickResult::Idle;
        };

        let mut proc = {
            let mut pt = self.process_table.lock();
            if let Some(p) = pt.take(pid) {
                p
            } else {
                debug_assert!(false, "stale PID dequeued: {pid:?}");
                return TickResult::Idle;
            }
        };

        // Check if the process was killed by an external exit signal while
        // it was in the run queue (e.g., via 2-arg `exit` or link cascade).
        if proc.status == ProcessStatus::Error {
            worker.current_pid = None;
            let mut realm_guard = realm.write();
            self.handle_process_exit(pid, &proc, &ExitReason::Error, mem, &mut realm_guard);
            {
                let mut pt = self.process_table.lock();
                pt.free_taken_slot(pid);
            }
            return TickResult::Error(
                pid,
                RuntimeError::BadArgument {
                    intrinsic: "exit",
                    message: "killed by exit signal",
                },
            );
        }

        proc.status = ProcessStatus::Running;
        proc.reset_reductions();
        worker.current_pid = Some(pid);

        // Run the process in batches, releasing the realm lock between batches.
        // This prevents other workers from spinning on the lock for the entire
        // reduction budget (~500µs). Each batch holds the lock for at most
        // REALM_LOCK_BATCH reductions (~25µs), keeping lock contention bounded.
        let result = Self::run_batched(worker, &mut proc, mem, realm, self);

        worker.current_pid = None;

        match result {
            RunResult::Yielded => {
                proc.status = ProcessStatus::Ready;
                {
                    let mut pt = self.process_table.lock();
                    pt.put_back(pid, proc);
                }
                // Drain exit signals queued while process was taken
                self.drain_pending_signals(pid, realm, mem);
                {
                    let mut rq = self.run_queues[worker_idx].lock();
                    let enqueued = rq.push_back(pid);
                    debug_assert!(enqueued, "re-enqueue of yielding process must not fail");
                }
                TickResult::Continued(pid)
            }
            RunResult::Waiting => {
                // Process is blocked on `receive`. Put back in table but
                // do NOT enqueue — `send` will wake it by setting status
                // to Ready and enqueueing on the run queue.
                //
                // Invariant: a Waiting process is placed back in the
                // ProcessTable (not Taken), so `send` can deep-copy
                // directly to its heap. Fragments are for Taken processes;
                // since we put_back here, the process is reachable directly.
                proc.status = ProcessStatus::Waiting;
                {
                    let mut pt = self.process_table.lock();
                    pt.put_back(pid, proc);
                }
                // Drain exit signals queued while process was taken
                self.drain_pending_signals(pid, realm, mem);
                TickResult::Continued(pid)
            }
            RunResult::Completed(value) => {
                let mut realm_guard = realm.write();
                self.handle_process_exit(pid, &proc, &ExitReason::Normal, mem, &mut realm_guard);
                {
                    let mut pt = self.process_table.lock();
                    pt.free_taken_slot(pid);
                }
                TickResult::Completed(pid, value)
            }
            RunResult::Exited(reason) => {
                let mut realm_guard = realm.write();
                self.handle_process_exit(
                    pid,
                    &proc,
                    &ExitReason::Term(reason),
                    mem,
                    &mut realm_guard,
                );
                {
                    let mut pt = self.process_table.lock();
                    pt.free_taken_slot(pid);
                }
                TickResult::Exited(pid, reason)
            }
            RunResult::Error(err) => {
                let mut realm_guard = realm.write();
                self.handle_process_exit(pid, &proc, &ExitReason::Error, mem, &mut realm_guard);
                {
                    let mut pt = self.process_table.lock();
                    pt.free_taken_slot(pid);
                }
                TickResult::Error(pid, err)
            }
        }
    }

    /// Run a process in batches, releasing the realm lock between batches.
    ///
    /// Each batch executes up to `REALM_LOCK_BATCH` reductions with the realm
    /// write lock held. Between batches, the lock is released to give other
    /// workers a chance to acquire it. This bounds worst-case lock contention
    /// to ~25µs per batch instead of ~500µs for the full time slice.
    fn run_batched<M: MemorySpace>(
        worker: &mut Worker,
        proc: &mut Process,
        mem: &mut M,
        realm: &SpinRwLock<Realm>,
        scheduler: &Self,
    ) -> RunResult {
        /// Maximum reductions per realm lock hold.
        const REALM_LOCK_BATCH: u32 = 100;

        loop {
            let batch = proc.reductions.min(REALM_LOCK_BATCH);
            if batch == 0 {
                return RunResult::Yielded;
            }

            // Save the full budget and set a limited sub-budget
            let saved = proc.reductions;
            proc.reductions = batch;

            let result = {
                let mut realm_guard = realm.write();
                Vm::run(worker, proc, mem, &mut realm_guard, Some(scheduler))
            };
            // Realm lock released here

            match result {
                RunResult::Yielded => {
                    // Sub-budget exhausted — restore remaining outer budget
                    let consumed = batch.saturating_sub(proc.reductions);
                    proc.reductions = saved.saturating_sub(consumed);
                    if proc.reductions == 0 {
                        return RunResult::Yielded;
                    }
                    // Continue with next batch
                }
                // Terminal or blocking results pass through immediately
                other => return other,
            }
        }
    }

    /// Try to steal work for an idle worker from the busiest neighbor.
    ///
    /// Uses ordered locking (lower index first) to prevent deadlock.
    /// The busiest-worker scan is TOCTOU-tolerant: queue lengths may change
    /// between the scan and the steal, so we re-check inside the critical section.
    ///
    /// Returns `true` if a process was stolen.
    pub fn try_steal_for(&self, idle_idx: usize) -> bool {
        // Find the busiest worker (advisory — may be stale by the time we lock)
        let mut busiest_idx = 0;
        let mut busiest_len = 0;

        for i in 0..self.worker_count {
            if i == idle_idx {
                continue;
            }
            let rq = self.run_queues[i].lock();
            let len = rq.len();
            if len > busiest_len {
                busiest_len = len;
                busiest_idx = i;
            }
        }

        // Only steal if the busiest had at least MIN_STEAL_THRESHOLD processes
        if busiest_len < MIN_STEAL_THRESHOLD {
            return false;
        }

        // Ordered locking: acquire lower index first to prevent deadlock
        let (lo, hi) = if idle_idx < busiest_idx {
            (idle_idx, busiest_idx)
        } else {
            (busiest_idx, idle_idx)
        };

        let mut lo_guard = self.run_queues[lo].lock();
        let mut hi_guard = self.run_queues[hi].lock();

        let (src, dst) = if busiest_idx == lo {
            (&mut *lo_guard, &mut *hi_guard)
        } else {
            (&mut *hi_guard, &mut *lo_guard)
        };

        // Re-check threshold under lock (scan was advisory, lengths may have changed)
        if src.len() < MIN_STEAL_THRESHOLD {
            return false;
        }

        src.steal_back().is_some_and(|pid| {
            dst.push_back(pid);
            true
        })
    }

    /// Run all processes to completion across all workers (single-threaded).
    ///
    /// Creates local workers and sequentially round-robins `tick_worker`
    /// across them. Intended for tests and single-threaded execution —
    /// real multi-TCB execution uses per-TCB `tick_worker` loops.
    ///
    /// When a worker is idle, attempts work stealing. Stops when all queues
    /// are empty and no work was done in a full cycle.
    pub fn run_all<M: MemorySpace>(&self, realm: &SpinRwLock<Realm>, mem: &mut M) {
        let mut workers: [Worker; DEFAULT_WORKER_COUNT] =
            core::array::from_fn(|i| Worker::new(WorkerId(i as u8)));

        loop {
            let mut any_work = false;
            for (idx, worker) in workers.iter_mut().enumerate().take(self.worker_count) {
                match self.tick_worker(worker, realm, mem) {
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
    pub fn process_count(&self) -> usize {
        let pt = self.process_table.lock();
        pt.count()
    }

    /// Get a reference to the process table (locked).
    ///
    /// This acquires the lock. The caller must drop the guard promptly.
    pub fn with_process_table<R>(&self, f: impl FnOnce(&ProcessTable) -> R) -> R {
        let pt = self.process_table.lock();
        f(&pt)
    }

    /// Get a mutable reference to the process table (locked).
    ///
    /// This acquires the lock. The caller must drop the guard promptly.
    pub fn with_process_table_mut<R>(&self, f: impl FnOnce(&mut ProcessTable) -> R) -> R {
        let mut pt = self.process_table.lock();
        f(&mut pt)
    }

    /// Check if a process is still alive.
    ///
    /// Returns `true` if the process exists in the table or is currently
    /// taken for execution.
    #[must_use]
    pub fn is_alive(&self, pid: ProcessId) -> bool {
        let pt = self.process_table.lock();
        pt.get(pid).is_some() || pt.is_taken(pid)
    }

    /// Check if any worker has runnable processes.
    #[must_use]
    pub fn has_work(&self) -> bool {
        for i in 0..self.worker_count {
            let rq = self.run_queues[i].lock();
            if !rq.is_empty() {
                return true;
            }
        }
        false
    }

    /// Get the number of active workers.
    #[must_use]
    pub const fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Get the length of a worker's run queue.
    #[must_use]
    pub fn run_queue_len(&self, worker_idx: usize) -> usize {
        let rq = self.run_queues[worker_idx].lock();
        rq.len()
    }

    /// Generate a unique reference ID for monitors.
    ///
    /// IDs are unique per realm (each realm has its own scheduler).
    /// Uses `Relaxed` ordering since uniqueness comes from `fetch_add`,
    /// not from cross-thread visibility of the counter itself.
    pub fn next_ref(&self) -> u64 {
        self.next_ref_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Drain pending exit signals queued while a process was taken.
    ///
    /// Called after `put_back` to deliver signals that arrived during execution.
    /// Each signal is delivered via `deliver_exit_signal`, which may kill the
    /// process or deliver a trap-exit message.
    fn drain_pending_signals<M: MemorySpace>(
        &self,
        pid: ProcessId,
        realm: &SpinRwLock<Realm>,
        mem: &mut M,
    ) {
        let signals = {
            let mut pt = self.process_table.lock();
            pt.take_pending_signals(pid)
        };
        if signals.is_empty() {
            return;
        }
        let mut realm_guard = realm.write();
        for (sender, reason) in signals {
            self.deliver_exit_signal(pid, sender, reason, &mut realm_guard, mem);
        }
    }

    // Exit propagation methods are in `exit_propagation.rs`.
}
