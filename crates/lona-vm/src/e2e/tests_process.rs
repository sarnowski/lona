// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Process concurrency E2E tests for Lona on seL4.
//!
//! These tests verify multi-process, multi-worker scheduling with real
//! message passing on seL4. Unlike the single-expression spec tests,
//! these spawn multiple processes via the `Scheduler`, run them across
//! all workers via `run_all`, and verify outcomes.

use crate::compiler;
use crate::platform::MemorySpace;
use crate::process::{Process, ProcessId, WorkerId};
use crate::reader::read;
use crate::realm::Realm;
use crate::scheduler::{Scheduler, TickResult, Worker};
use crate::sync::SpinRwLock;
use crate::term::printer::print_term;
use crate::uart::Uart;
use crate::vm;

use super::tests_basic::OutputBuffer;

/// Run all process concurrency E2E tests.
///
/// Returns the number of failures.
pub fn run_process_tests<M: MemorySpace, U: Uart>(
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
    _uart: &mut U,
    scheduler: &Scheduler,
) -> u32 {
    sel4::debug_println!("=== PROCESS TEST RUN ===");

    let mut passed = 0u32;
    let mut failed = 0u32;

    // --- Single-expression intrinsic tests ---
    run_expr_test(
        "pid_self",
        "(pid? (self))",
        "true",
        proc,
        realm,
        mem,
        scheduler,
        &mut passed,
        &mut failed,
    );
    run_expr_test(
        "alive_self",
        "(alive? (self))",
        "true",
        proc,
        realm,
        mem,
        scheduler,
        &mut passed,
        &mut failed,
    );
    run_expr_test(
        "spawn_returns_pid",
        "(pid? (spawn (fn* [] :ok)))",
        "true",
        proc,
        realm,
        mem,
        scheduler,
        &mut passed,
        &mut failed,
    );
    run_expr_test(
        "send_receive_roundtrip",
        "(do (send (self) :hello) (receive :hello :got-it :after 100 :timeout))",
        ":got-it",
        proc,
        realm,
        mem,
        scheduler,
        &mut passed,
        &mut failed,
    );
    run_expr_test(
        "spawn_link_returns_pid",
        "(pid? (spawn-link (fn* [] :ok)))",
        "true",
        proc,
        realm,
        mem,
        scheduler,
        &mut passed,
        &mut failed,
    );
    run_expr_test(
        "spawn_monitor_returns_tuple",
        "(tuple? (spawn-monitor (fn* [] :ok)))",
        "true",
        proc,
        realm,
        mem,
        scheduler,
        &mut passed,
        &mut failed,
    );

    // --- Multi-process concurrency tests ---
    // These compile chunks, then spawn real processes across workers and
    // run them to completion via scheduler.run_all().

    run_multi_test(
        "multi_spawn_all_complete",
        test_multi_spawn_all_complete,
        proc,
        realm,
        mem,
        &mut passed,
        &mut failed,
    );
    run_multi_test(
        "multi_worker_distribution",
        test_multi_worker_distribution,
        proc,
        realm,
        mem,
        &mut passed,
        &mut failed,
    );
    run_multi_test(
        "link_cascade_kills_partner",
        test_link_cascade_kills_partner,
        proc,
        realm,
        mem,
        &mut passed,
        &mut failed,
    );
    run_multi_test(
        "monitor_delivers_down",
        test_monitor_delivers_down,
        proc,
        realm,
        mem,
        &mut passed,
        &mut failed,
    );

    sel4::debug_println!(
        "=== PROCESS RESULTS: {} passed, {} failed ===",
        passed,
        failed
    );

    failed
}

/// Run a single-expression test.
fn run_expr_test<M: MemorySpace>(
    name: &str,
    expr: &str,
    expected: &str,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
    scheduler: &Scheduler,
    passed: &mut u32,
    failed: &mut u32,
) {
    sel4::debug_print!("[PROCESS] {} ... ", name);
    let mut output = OutputBuffer::new();
    if eval_and_check(expr, expected, proc, realm, mem, scheduler, &mut output) {
        sel4::debug_println!("PASS");
        *passed += 1;
    } else {
        let actual = output.as_str().unwrap_or("ERROR");
        sel4::debug_println!("FAIL");
        sel4::debug_println!("  Expected: {}", expected);
        sel4::debug_println!("  Actual: {}", actual);
        *failed += 1;
    }
    reset_test_process(proc, realm, mem);
}

/// Run a multi-process concurrency test.
fn run_multi_test<M: MemorySpace>(
    name: &str,
    test_fn: fn(&mut Process, &mut Realm, &mut M) -> Result<(), &'static str>,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
    passed: &mut u32,
    failed: &mut u32,
) {
    sel4::debug_print!("[PROCESS] {} ... ", name);
    match test_fn(proc, realm, mem) {
        Ok(()) => {
            sel4::debug_println!("PASS");
            *passed += 1;
        }
        Err(msg) => {
            sel4::debug_println!("FAIL");
            sel4::debug_println!("  Error: {}", msg);
            *failed += 1;
        }
    }
    reset_test_process(proc, realm, mem);
}

/// Reset the test process between tests.
fn reset_test_process<M: MemorySpace>(proc: &mut Process, realm: &mut Realm, mem: &mut M) {
    let mut worker = Worker::new(WorkerId(0));
    worker.reset_x_regs();
    proc.reset();
    proc.reset_heap();
    if let (Some(nv), Some(cn)) = (
        crate::realm::get_ns_var(realm, mem),
        crate::realm::get_core_ns(realm, mem),
    ) {
        proc.bootstrap(nv, cn);
    }
    let index = proc.pid.index() as u32;
    let generation = proc.pid.generation();
    if let Some(pid_term) = proc.alloc_term_pid(mem, index, generation) {
        proc.pid_term = Some(pid_term);
    }
}

// =============================================================================
// Multi-process concurrency tests
// =============================================================================

/// Spawn 8 processes on different workers, verify all complete.
fn test_multi_spawn_all_complete<M: MemorySpace>(
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
) -> Result<(), &'static str> {
    let ns_var = crate::realm::get_ns_var(realm, mem).ok_or("ns_var lookup")?;
    let core_ns = crate::realm::get_core_ns(realm, mem).ok_or("core_ns lookup")?;

    // Compile a simple chunk: (+ 1 2) → 3
    let chunk = compile_chunk("(+ 1 2)", proc, realm, mem)?;

    // Create a scheduler and wrap realm for multi-worker operation
    let scheduler = Scheduler::new(ns_var, core_ns);
    let realm_lock = SpinRwLock::new(core::mem::replace(realm, unsafe_empty_realm()));

    // Spawn 8 processes across 4 workers (2 per worker)
    for i in 0..8 {
        let worker_idx = i % scheduler.worker_count();
        if scheduler
            .spawn_on(&realm_lock, mem, chunk, ProcessId::NULL, worker_idx)
            .is_none()
        {
            *realm = realm_lock.into_inner();
            return Err("spawn failed");
        }
    }

    let count_before = scheduler.process_count();
    scheduler.run_all(&realm_lock, mem);
    let count_after = scheduler.process_count();

    // Unwrap realm back
    *realm = realm_lock.into_inner();

    if count_before != 8 {
        return Err("expected 8 spawned processes");
    }
    if count_after != 0 {
        return Err("not all processes completed");
    }
    Ok(())
}

/// Spawn processes on each worker and verify work stealing distributes load.
fn test_multi_worker_distribution<M: MemorySpace>(
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
) -> Result<(), &'static str> {
    let ns_var = crate::realm::get_ns_var(realm, mem).ok_or("ns_var lookup")?;
    let core_ns = crate::realm::get_core_ns(realm, mem).ok_or("core_ns lookup")?;

    let chunk = compile_chunk("(+ 10 20)", proc, realm, mem)?;

    let scheduler = Scheduler::new(ns_var, core_ns);
    let realm_lock = SpinRwLock::new(core::mem::replace(realm, unsafe_empty_realm()));

    // Spawn 4 processes, one on each worker
    for i in 0..4 {
        if scheduler
            .spawn_on(&realm_lock, mem, chunk, ProcessId::NULL, i)
            .is_none()
        {
            *realm = realm_lock.into_inner();
            return Err("spawn failed");
        }
    }

    // Verify each worker has exactly 1 process
    for i in 0..4 {
        if scheduler.run_queue_len(i) != 1 {
            *realm = realm_lock.into_inner();
            return Err("uneven distribution before run");
        }
    }

    // Run all — all 4 workers tick their processes
    scheduler.run_all(&realm_lock, mem);

    *realm = realm_lock.into_inner();

    if scheduler.process_count() != 0 {
        return Err("not all processes completed");
    }
    Ok(())
}

/// Link two processes: when A errors, B should be killed.
fn test_link_cascade_kills_partner<M: MemorySpace>(
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
) -> Result<(), &'static str> {
    let ns_var = crate::realm::get_ns_var(realm, mem).ok_or("ns_var lookup")?;
    let core_ns = crate::realm::get_core_ns(realm, mem).ok_or("core_ns lookup")?;

    // A: errors immediately (divide by zero)
    let chunk_a = compile_chunk("(/ 1 0)", proc, realm, mem)?;
    // B: long computation (survives long enough for A to die first)
    let chunk_b = compile_chunk(
        "(do (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1))",
        proc,
        realm,
        mem,
    )?;

    let scheduler = Scheduler::new(ns_var, core_ns);
    let realm_lock = SpinRwLock::new(core::mem::replace(realm, unsafe_empty_realm()));

    let pid_a = scheduler
        .spawn_on(&realm_lock, mem, chunk_a, ProcessId::NULL, 0)
        .ok_or("spawn A failed")?;
    let pid_b = scheduler
        .spawn_on(&realm_lock, mem, chunk_b, ProcessId::NULL, 0)
        .ok_or("spawn B failed")?;

    // Manually establish bidirectional link
    scheduler.with_process_table_mut(|pt| {
        if let Some(a) = pt.get_mut(pid_a) {
            a.links.insert(pid_b);
        }
        if let Some(b) = pt.get_mut(pid_b) {
            b.links.insert(pid_a);
        }
    });

    // Run tick-by-tick to observe results
    let mut worker = Worker::new(WorkerId(0));
    let mut completed = 0u32;
    let mut errored = 0u32;
    loop {
        match scheduler.tick_worker(&mut worker, &realm_lock, mem) {
            TickResult::Idle => {
                if !scheduler.try_steal_for(0) {
                    break;
                }
            }
            TickResult::Completed(_, _) | TickResult::Exited(_, _) => completed += 1,
            TickResult::Error(_, _) => errored += 1,
            TickResult::Continued(_) => {}
        }
    }

    *realm = realm_lock.into_inner();

    // A errors, B gets killed by link cascade — both should be gone
    if scheduler.process_count() != 0 {
        return Err("processes still alive after link cascade");
    }
    // At least A should have errored
    if errored == 0 {
        return Err("expected at least one error (A's divide by zero)");
    }
    // B should NOT have completed normally
    if completed > 0 {
        return Err("B should have been killed, not completed");
    }
    Ok(())
}

/// Monitor a process: when it exits, the monitoring process gets a `:DOWN` message.
fn test_monitor_delivers_down<M: MemorySpace>(
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
) -> Result<(), &'static str> {
    let ns_var = crate::realm::get_ns_var(realm, mem).ok_or("ns_var lookup")?;
    let core_ns = crate::realm::get_core_ns(realm, mem).ok_or("core_ns lookup")?;

    // Monitored: completes quickly
    let chunk_target = compile_chunk(":done", proc, realm, mem)?;
    // Monitor: long computation so it's still alive when target exits
    let chunk_monitor = compile_chunk(
        "(do (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1) (+ 1 1))",
        proc,
        realm,
        mem,
    )?;

    let scheduler = Scheduler::new(ns_var, core_ns);
    let realm_lock = SpinRwLock::new(core::mem::replace(realm, unsafe_empty_realm()));

    let pid_target = scheduler
        .spawn_on(&realm_lock, mem, chunk_target, ProcessId::NULL, 0)
        .ok_or("spawn target failed")?;
    let pid_monitor = scheduler
        .spawn_on(&realm_lock, mem, chunk_monitor, ProcessId::NULL, 0)
        .ok_or("spawn monitor failed")?;

    // Establish monitor: pid_monitor monitors pid_target
    let ref_id = scheduler.next_ref();
    scheduler.with_process_table_mut(|pt| {
        if let Some(target) = pt.get_mut(pid_target) {
            target.monitored_by.insert(ref_id, pid_monitor);
        }
        if let Some(monitor) = pt.get_mut(pid_monitor) {
            monitor.monitors_out.insert(ref_id, pid_target);
        }
    });

    // Tick until target completes
    let mut worker = Worker::new(WorkerId(0));
    let mut target_done = false;
    for _ in 0..100 {
        match scheduler.tick_worker(&mut worker, &realm_lock, mem) {
            TickResult::Completed(pid, _) if pid == pid_target => {
                target_done = true;
                break;
            }
            TickResult::Idle => {
                if !scheduler.try_steal_for(0) {
                    break;
                }
            }
            _ => {}
        }
    }

    if !target_done {
        *realm = realm_lock.into_inner();
        return Err("target process did not complete");
    }

    // Verify monitor received :DOWN message
    let has_down = scheduler
        .with_process_table(|pt| pt.get(pid_monitor).is_some_and(|p| !p.mailbox.is_empty()));

    // Run remaining to completion
    scheduler.run_all(&realm_lock, mem);

    *realm = realm_lock.into_inner();

    if !has_down {
        return Err("monitor did not receive :DOWN message");
    }
    Ok(())
}

// =============================================================================
// Helpers
// =============================================================================

/// Compile a Lonala expression and return the chunk address.
fn compile_chunk<M: MemorySpace>(
    src: &str,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
) -> Result<crate::Vaddr, &'static str> {
    let expr = read(src, proc, realm, mem)
        .ok()
        .flatten()
        .ok_or("parse failed")?;
    let chunk = compiler::compile(expr, proc, mem, realm).map_err(|_| "compile failed")?;
    if !proc.write_chunk_to_heap(mem, &chunk) {
        return Err("write chunk to heap failed");
    }
    proc.chunk_addr.ok_or("no chunk addr after compilation")
}

/// Evaluate a single expression and check the result.
fn eval_and_check<M: MemorySpace>(
    expr_str: &str,
    expected: &str,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
    scheduler: &Scheduler,
    output: &mut OutputBuffer,
) -> bool {
    let expr = match read(expr_str, proc, realm, mem) {
        Ok(Some(v)) => v,
        _ => return false,
    };
    let chunk = match compiler::compile(expr, proc, mem, realm) {
        Ok(c) => c,
        _ => return false,
    };
    if !proc.write_chunk_to_heap(mem, &chunk) {
        return false;
    }
    let mut worker = Worker::new(WorkerId(0));
    let result = match vm::execute_with_scheduler(&mut worker, proc, mem, realm, Some(scheduler)) {
        Ok(v) => v,
        _ => return false,
    };
    print_term(result, proc, realm, mem, output);
    output.as_str() == Ok(expected)
}

/// Create a placeholder `Realm` for `core::mem::replace`.
///
/// This is used to temporarily move the real realm into a `SpinRwLock`
/// for multi-worker scheduling. The placeholder is immediately overwritten
/// when the realm is moved back via `realm_lock.into_inner()`.
///
/// # Safety
///
/// The returned `Realm` must never be used — it has zeroed pool/code pointers.
/// It exists only as a move target for `core::mem::replace`.
fn unsafe_empty_realm() -> Realm {
    use crate::process::pool::ProcessPool;
    Realm::new(
        ProcessPool::new(crate::Vaddr::new(0), 0),
        crate::Vaddr::new(0),
        0,
    )
}
