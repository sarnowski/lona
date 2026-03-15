// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the Scheduler.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::Vaddr;
use crate::compiler::compile;
use crate::platform::MockVSpace;
use crate::process::pool::ProcessPool;
use crate::process::{Process, ProcessId};
use crate::reader::read;
use crate::realm::{Realm, bootstrap};
use crate::term::Term;

/// Create a test environment with bootstrapped realm and scheduler.
///
/// Uses a 1MB pool (128KB code region) and 2MB `MockVSpace` to support
/// multiple processes and a compiler process.
fn setup() -> (Scheduler, Realm, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(2 * 1024 * 1024, base);

    let mut pool = ProcessPool::new(base, 1024 * 1024);
    let code_base = pool.allocate(128 * 1024, 8).expect("code region alloc");
    let mut realm = Realm::new(pool, code_base, 128 * 1024);

    let result = bootstrap(&mut realm, &mut mem).expect("bootstrap failed");
    let scheduler = Scheduler::new(result.ns_var, result.core_ns);

    (scheduler, realm, mem)
}

/// Compile a Lonala expression and return its chunk address.
///
/// Uses a temporary compiler process allocated from the realm. The chunk
/// persists in the shared `MockVSpace` so spawned processes can execute it.
///
/// Panics if the realm's pool is exhausted or compilation fails.
fn compile_expr(realm: &mut Realm, mem: &mut MockVSpace, src: &str) -> Vaddr {
    // Large heap for compiling expressions with many sub-expressions
    let (young_base, old_base) = realm
        .allocate_process_memory(128 * 1024, 4 * 1024)
        .expect("compiler process allocation");
    let mut proc = Process::new(young_base, 128 * 1024, old_base, 4 * 1024);

    let ns_var = crate::realm::get_ns_var(realm, mem).expect("ns_var lookup");
    let core_ns = crate::realm::get_core_ns(realm, mem).expect("core_ns lookup");
    proc.bootstrap(ns_var, core_ns);

    let expr = read(src, &mut proc, realm, mem)
        .ok()
        .flatten()
        .expect("parse failed");
    let chunk = compile(expr, &mut proc, mem, realm).expect("compile failed");
    assert!(proc.write_chunk_to_heap(mem, &chunk), "write chunk failed");
    proc.chunk_addr.unwrap()
}

/// Spawn a Lonala expression as a new process on the scheduler.
fn spawn_expr(
    scheduler: &mut Scheduler,
    realm: &mut Realm,
    mem: &mut MockVSpace,
    src: &str,
) -> ProcessId {
    let chunk_addr = compile_expr(realm, mem, src);
    scheduler
        .spawn(realm, mem, chunk_addr, ProcessId::NULL)
        .expect("spawn failed")
}

/// Build a long-running expression using nested `do` blocks.
///
/// Each `(+ 1 1)` costs ~3 reductions. With `MAX_REDUCTIONS` = 2000,
/// 800 additions (2400 reductions) should cause at least one yield.
///
/// Lists are limited to 64 elements by the parser, so we nest `do` blocks
/// of up to 50 sub-expressions each.
fn long_running_expr(additions: usize) -> std::string::String {
    const BLOCK_SIZE: usize = 50;
    let mut remaining = additions;
    let mut blocks = std::vec::Vec::new();

    while remaining > 0 {
        let n = remaining.min(BLOCK_SIZE);
        let mut block = std::string::String::from("(do");
        for _ in 0..n {
            block.push_str(" (+ 1 1)");
        }
        block.push(')');
        blocks.push(block);
        remaining -= n;
    }

    // Nest blocks: (do block1 (do block2 (do block3 ...)))
    // But each nesting level adds one element, staying well under 64
    while blocks.len() > 1 {
        let last = blocks.pop().unwrap();
        let prev = blocks.last_mut().unwrap();
        // Replace trailing ')' with the nested block
        prev.pop(); // remove ')'
        prev.push(' ');
        prev.push_str(&last);
        prev.push(')');
    }

    blocks.into_iter().next().unwrap_or_else(|| "(do)".into())
}

// =============================================================================
// Construction tests
// =============================================================================

#[test]
fn new_creates_empty_scheduler() {
    let (scheduler, _, _) = setup();
    assert_eq!(scheduler.process_count(), 0);
    assert!(!scheduler.has_work());
    assert_eq!(scheduler.worker_count(), DEFAULT_WORKER_COUNT);
}

#[test]
fn with_worker_count_creates_custom() {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(2 * 1024 * 1024, base);
    let mut pool = ProcessPool::new(base, 1024 * 1024);
    let code_base = pool.allocate(128 * 1024, 8).unwrap();
    let mut realm = Realm::new(pool, code_base, 128 * 1024);
    let result = bootstrap(&mut realm, &mut mem).unwrap();
    let s = Scheduler::with_worker_count(result.ns_var, result.core_ns, 2);
    assert_eq!(s.worker_count(), 2);
}

#[test]
fn spawn_returns_valid_pid() {
    let (mut scheduler, mut realm, mut mem) = setup();
    let pid = spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 1 2)");
    assert!(!pid.is_null());
    assert_eq!(pid.index(), 0);
    assert_eq!(pid.generation(), 0);
}

#[test]
fn spawn_multiple_distinct_pids() {
    let (mut scheduler, mut realm, mut mem) = setup();
    let pid1 = spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 1 2)");
    let pid2 = spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 3 4)");
    let pid3 = spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 5 6)");
    assert_ne!(pid1, pid2);
    assert_ne!(pid2, pid3);
    assert_ne!(pid1, pid3);
}

#[test]
fn spawn_enqueues_on_run_queue() {
    let (mut scheduler, mut realm, mut mem) = setup();
    assert!(!scheduler.has_work());
    spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 1 2)");
    assert!(scheduler.has_work());
}

#[test]
fn spawn_bootstraps_process() {
    let (mut scheduler, mut realm, mut mem) = setup();
    let pid = spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 1 2)");
    let proc = scheduler.get_process(pid).expect("process should exist");
    // Process should have *ns* binding (bootstrapped)
    assert!(
        !proc.bindings.is_empty(),
        "process should have bindings from bootstrap"
    );
}

#[test]
fn spawn_fails_when_pool_exhausted() {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(256 * 1024, base);

    // Tiny pool: 64KB code + barely enough for one compiler process
    let mut pool = ProcessPool::new(base, 128 * 1024);
    let code_base = pool.allocate(64 * 1024, 8).unwrap();
    let mut realm = Realm::new(pool, code_base, 64 * 1024);

    let result = bootstrap(&mut realm, &mut mem).unwrap();
    let mut scheduler = Scheduler::new(result.ns_var, result.core_ns);

    // Compile with what space we have — use a small heap for tiny expression
    let (young_base, old_base) = realm
        .allocate_process_memory(16 * 1024, 1024)
        .expect("compiler alloc");
    let mut proc = Process::new(young_base, 16 * 1024, old_base, 1024);
    proc.bootstrap(result.ns_var, result.core_ns);
    let expr = read("1", &mut proc, &mut realm, &mut mem)
        .ok()
        .flatten()
        .unwrap();
    let chunk = compile(expr, &mut proc, &mut mem, &mut realm).unwrap();
    assert!(proc.write_chunk_to_heap(&mut mem, &chunk));
    let chunk_addr = proc.chunk_addr.unwrap();

    // Exhaust remaining pool space — spawn should eventually fail
    let max_spawn_attempts = 100;
    let mut spawned = 0;
    for _ in 0..max_spawn_attempts {
        if scheduler
            .spawn(&mut realm, &mut mem, chunk_addr, ProcessId::NULL)
            .is_none()
        {
            break;
        }
        spawned += 1;
    }
    // At least one spawn should succeed, but pool should exhaust before the cap
    assert!(spawned > 0, "at least one spawn should succeed");
    assert!(spawned < max_spawn_attempts, "should have exhausted pool");
}

#[test]
fn is_alive_true_for_spawned() {
    let (mut scheduler, mut realm, mut mem) = setup();
    let pid = spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 1 2)");
    assert!(scheduler.is_alive(pid));
}

#[test]
fn is_alive_false_after_completion() {
    let (mut scheduler, mut realm, mut mem) = setup();
    let pid = spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 1 2)");
    scheduler.run_all(&mut realm, &mut mem);
    assert!(!scheduler.is_alive(pid));
}

// =============================================================================
// Scheduler loop tests
// =============================================================================

#[test]
fn tick_returns_idle_when_empty() {
    let (mut scheduler, mut realm, mut mem) = setup();
    assert!(matches!(
        scheduler.tick(&mut realm, &mut mem),
        TickResult::Idle
    ));
}

#[test]
fn tick_completes_simple_process() {
    let (mut scheduler, mut realm, mut mem) = setup();
    let pid = spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 1 2)");

    // Simple addition should complete in one tick
    let result = scheduler.tick(&mut realm, &mut mem);
    match result {
        TickResult::Completed(completed_pid, value) => {
            assert_eq!(completed_pid, pid);
            assert_eq!(value, Term::small_int(3).unwrap());
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[test]
fn tick_removes_completed_from_table() {
    let (mut scheduler, mut realm, mut mem) = setup();
    spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 1 2)");
    assert_eq!(scheduler.process_count(), 1);

    assert!(matches!(
        scheduler.tick(&mut realm, &mut mem),
        TickResult::Completed(_, _)
    ));
    assert_eq!(scheduler.process_count(), 0);
}

#[test]
fn tick_removes_errored_from_table() {
    let (mut scheduler, mut realm, mut mem) = setup();
    // Division by zero causes a runtime error
    let pid = spawn_expr(&mut scheduler, &mut realm, &mut mem, "(/ 1 0)");
    assert_eq!(scheduler.process_count(), 1);

    let result = scheduler.tick(&mut realm, &mut mem);
    assert!(matches!(result, TickResult::Error(p, _) if p == pid));
    assert_eq!(scheduler.process_count(), 0);
}

#[test]
fn tick_requeues_yielded_process() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // 800 additions × ~3 reductions each = ~2400 > MAX_REDUCTIONS (2000)
    let expr = long_running_expr(800);
    let pid = spawn_expr(&mut scheduler, &mut realm, &mut mem, &expr);

    let result = scheduler.tick(&mut realm, &mut mem);
    match result {
        TickResult::Continued(continued_pid) => {
            assert_eq!(continued_pid, pid);
            assert!(scheduler.is_alive(pid));
            assert!(scheduler.has_work());
        }
        other => {
            panic!("expected Continued (800 additions should exceed MAX_REDUCTIONS), got {other:?}")
        }
    }
}

#[test]
fn run_all_completes_single_process() {
    let (mut scheduler, mut realm, mut mem) = setup();
    spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 10 20)");
    scheduler.run_all(&mut realm, &mut mem);
    assert_eq!(scheduler.process_count(), 0);
}

#[test]
fn run_all_completes_multiple_processes() {
    let (mut scheduler, mut realm, mut mem) = setup();
    spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 1 2)");
    spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 3 4)");
    spawn_expr(&mut scheduler, &mut realm, &mut mem, "(+ 5 6)");
    assert_eq!(scheduler.process_count(), 3);

    scheduler.run_all(&mut realm, &mut mem);
    assert_eq!(scheduler.process_count(), 0);
}

#[test]
fn round_robin_three_processes() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // Three single-tick processes completing in FIFO order proves round-robin
    let chunk1 = compile_expr(&mut realm, &mut mem, "(+ 1 2)");
    let chunk2 = compile_expr(&mut realm, &mut mem, "(+ 3 4)");
    let chunk3 = compile_expr(&mut realm, &mut mem, "(+ 5 6)");

    let pid1 = scheduler
        .spawn(&mut realm, &mut mem, chunk1, ProcessId::NULL)
        .unwrap();
    let pid2 = scheduler
        .spawn(&mut realm, &mut mem, chunk2, ProcessId::NULL)
        .unwrap();
    let pid3 = scheduler
        .spawn(&mut realm, &mut mem, chunk3, ProcessId::NULL)
        .unwrap();

    // Each tick dequeues and completes one process in spawn order
    let mut completed_order = std::vec::Vec::new();
    for _ in 0..3 {
        match scheduler.tick(&mut realm, &mut mem) {
            TickResult::Completed(pid, _) => completed_order.push(pid),
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    // FIFO order: pid1, pid2, pid3
    assert_eq!(completed_order, [pid1, pid2, pid3]);
    assert_eq!(scheduler.process_count(), 0);
}

// =============================================================================
// Integration tests
// =============================================================================

#[test]
fn three_processes_compute_values() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // Spawn 3 processes computing different additions
    let chunk1 = compile_expr(&mut realm, &mut mem, "(+ 1 2)");
    let chunk2 = compile_expr(&mut realm, &mut mem, "(+ 3 4)");
    let chunk3 = compile_expr(&mut realm, &mut mem, "(+ 5 6)");

    let pid1 = scheduler
        .spawn(&mut realm, &mut mem, chunk1, ProcessId::NULL)
        .unwrap();
    let pid2 = scheduler
        .spawn(&mut realm, &mut mem, chunk2, ProcessId::NULL)
        .unwrap();
    let pid3 = scheduler
        .spawn(&mut realm, &mut mem, chunk3, ProcessId::NULL)
        .unwrap();

    // Collect results
    let mut results = std::vec::Vec::new();
    loop {
        match scheduler.tick(&mut realm, &mut mem) {
            TickResult::Idle => break,
            TickResult::Completed(pid, value) => results.push((pid, value)),
            TickResult::Continued(_) => {}
            TickResult::Error(pid, err) => panic!("process {pid:?} errored: {err:?}"),
        }
    }

    assert_eq!(results.len(), 3);

    // Find result for each PID
    let find = |pid| results.iter().find(|(p, _)| *p == pid).map(|(_, v)| *v);
    assert_eq!(find(pid1), Some(Term::small_int(3).unwrap()));
    assert_eq!(find(pid2), Some(Term::small_int(7).unwrap()));
    assert_eq!(find(pid3), Some(Term::small_int(11).unwrap()));
}

#[test]
fn process_error_doesnt_affect_others() {
    let (mut scheduler, mut realm, mut mem) = setup();

    let good_chunk = compile_expr(&mut realm, &mut mem, "(+ 10 20)");
    let bad_chunk = compile_expr(&mut realm, &mut mem, "(/ 1 0)");

    let good_pid = scheduler
        .spawn(&mut realm, &mut mem, good_chunk, ProcessId::NULL)
        .unwrap();
    let bad_pid = scheduler
        .spawn(&mut realm, &mut mem, bad_chunk, ProcessId::NULL)
        .unwrap();

    let mut completed = std::vec::Vec::new();
    let mut errored = std::vec::Vec::new();
    loop {
        match scheduler.tick(&mut realm, &mut mem) {
            TickResult::Idle => break,
            TickResult::Completed(pid, value) => completed.push((pid, value)),
            TickResult::Error(pid, _) => errored.push(pid),
            TickResult::Continued(_) => {}
        }
    }

    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].0, good_pid);
    assert_eq!(completed[0].1, Term::small_int(30).unwrap());

    assert_eq!(errored.len(), 1);
    assert_eq!(errored[0], bad_pid);
}

#[test]
fn yield_and_resume_preserves_state() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // 800 additions, each producing 2. The `do` form returns the last result.
    let expr = long_running_expr(800);
    spawn_expr(&mut scheduler, &mut realm, &mut mem, &expr);

    // Tick until completion
    let mut ticks = 0;
    loop {
        match scheduler.tick(&mut realm, &mut mem) {
            TickResult::Idle => break,
            TickResult::Completed(_, value) => {
                // `do` returns the last `(+ 1 1)` = 2
                assert_eq!(value, Term::small_int(2).unwrap());
                break;
            }
            TickResult::Continued(_) => ticks += 1,
            TickResult::Error(_, err) => panic!("unexpected error: {err:?}"),
        }
    }

    // Should have yielded at least once (~2400 reductions > MAX_REDUCTIONS 2000)
    assert!(ticks >= 1, "long computation should yield at least once");
}

#[test]
fn mixed_short_and_long_processes() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // Short process: simple computation
    let short_chunk = compile_expr(&mut realm, &mut mem, "(+ 1 1)");
    let short_pid = scheduler
        .spawn(&mut realm, &mut mem, short_chunk, ProcessId::NULL)
        .unwrap();

    // Long process: many additions
    let long_expr = long_running_expr(800);
    let long_chunk = compile_expr(&mut realm, &mut mem, &long_expr);
    let long_pid = scheduler
        .spawn(&mut realm, &mut mem, long_chunk, ProcessId::NULL)
        .unwrap();

    // Run until all done
    let mut completed_order = std::vec::Vec::new();
    loop {
        match scheduler.tick(&mut realm, &mut mem) {
            TickResult::Idle => break,
            TickResult::Completed(pid, _) => completed_order.push(pid),
            TickResult::Continued(_) => {}
            TickResult::Error(pid, err) => panic!("process {pid:?} errored: {err:?}"),
        }
    }

    // Both should complete
    assert!(completed_order.contains(&short_pid));
    assert!(completed_order.contains(&long_pid));

    // Short process should complete before long process
    let short_idx = completed_order
        .iter()
        .position(|&p| p == short_pid)
        .unwrap();
    let long_idx = completed_order.iter().position(|&p| p == long_pid).unwrap();
    assert!(
        short_idx < long_idx,
        "short process should complete before long process"
    );
}

// =============================================================================
// Multi-worker tests
// =============================================================================

#[test]
fn multi_worker_creation() {
    let (scheduler, _, _) = setup();
    assert_eq!(scheduler.worker_count(), DEFAULT_WORKER_COUNT);

    // Each worker has a distinct ID
    for i in 0..scheduler.worker_count() {
        assert_eq!(scheduler.worker(i).id, crate::process::WorkerId(i as u8));
    }
}

#[test]
fn spawn_on_specific_worker() {
    let (mut scheduler, mut realm, mut mem) = setup();
    let chunk = compile_expr(&mut realm, &mut mem, "(+ 1 2)");

    // Spawn on Worker 2
    let pid = scheduler
        .spawn_on(&mut realm, &mut mem, chunk, ProcessId::NULL, 2)
        .unwrap();

    // Worker 0 should be empty, Worker 2 should have work
    assert!(!scheduler.worker(0).has_work());
    assert!(scheduler.worker(2).has_work());

    // tick_worker(2) should pick it up
    let result = scheduler.tick_worker(2, &mut realm, &mut mem);
    assert!(matches!(result, TickResult::Completed(p, _) if p == pid));
}

#[test]
fn multi_worker_round_robin() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // Spawn 4 processes on alternating workers (one per worker)
    // Use a single shared chunk to conserve pool memory
    let chunk = compile_expr(&mut realm, &mut mem, "(+ 1 1)");
    for i in 0..4 {
        scheduler
            .spawn_on(&mut realm, &mut mem, chunk, ProcessId::NULL, i)
            .unwrap();
    }

    assert_eq!(scheduler.process_count(), 4);

    // run_all should complete everything
    scheduler.run_all(&mut realm, &mut mem);
    assert_eq!(scheduler.process_count(), 0);
}

#[test]
fn work_stealing_balances_load() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // Spawn 4 long-running processes all on Worker 0
    for _ in 0..4 {
        let expr = long_running_expr(800);
        let chunk = compile_expr(&mut realm, &mut mem, &expr);
        scheduler
            .spawn_on(&mut realm, &mut mem, chunk, ProcessId::NULL, 0)
            .unwrap();
    }

    // Worker 0 has 4, others have 0
    assert_eq!(scheduler.worker(0).run_queue.len(), 4);
    assert_eq!(scheduler.worker(1).run_queue.len(), 0);

    // Tick worker 1 (idle) — should trigger steal
    let result = scheduler.tick_worker(1, &mut realm, &mut mem);
    assert!(matches!(result, TickResult::Idle));

    // try_steal_for should steal from worker 0
    let stolen = scheduler.try_steal_for(1);
    assert!(stolen, "should steal from overloaded worker 0");
    assert!(scheduler.worker(1).has_work());
}

#[test]
fn run_all_multi_worker_completes_all() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // Use a single compiled chunk to conserve pool memory
    let chunk = compile_expr(&mut realm, &mut mem, "(+ 1 1)");

    // Spawn 4 processes on different workers
    for i in 0..4 {
        scheduler
            .spawn_on(&mut realm, &mut mem, chunk, ProcessId::NULL, i)
            .unwrap();
    }

    scheduler.run_all(&mut realm, &mut mem);
    assert_eq!(scheduler.process_count(), 0);
}

// =============================================================================
// Work stealing boundary tests
// =============================================================================

#[test]
fn work_stealing_no_steal_with_one_process() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // Only 1 long-running process on Worker 0
    let expr = long_running_expr(800);
    let chunk = compile_expr(&mut realm, &mut mem, &expr);
    scheduler
        .spawn_on(&mut realm, &mut mem, chunk, ProcessId::NULL, 0)
        .unwrap();

    // Worker 0 has 1, others have 0 — threshold is 2, should NOT steal
    assert!(!scheduler.try_steal_for(1));
}

#[test]
fn work_stealing_steals_with_two_processes() {
    let (mut scheduler, mut realm, mut mem) = setup();

    // 2 long-running processes on Worker 0
    let expr = long_running_expr(800);
    let chunk = compile_expr(&mut realm, &mut mem, &expr);
    scheduler
        .spawn_on(&mut realm, &mut mem, chunk, ProcessId::NULL, 0)
        .unwrap();
    scheduler
        .spawn_on(&mut realm, &mut mem, chunk, ProcessId::NULL, 0)
        .unwrap();

    // Worker 0 has 2 — exactly at threshold, should steal
    assert!(scheduler.try_steal_for(1));
    assert!(scheduler.worker(1).has_work());
}
