// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Special intrinsic handlers for the VM.
//!
//! These intrinsics need access to Worker, Realm, or `Scheduler` that
//! the normal `call_intrinsic` dispatch doesn't provide.

use crate::gc;
use crate::intrinsics;
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::scheduler::{ProcessTable, Scheduler, Worker};
use crate::term::Term;
use crate::term::header::Header;
use crate::term::tag::object;

use super::{RunResult, RuntimeError, build_process_info, term_type_name};

/// Dispatch special intrinsics.
///
/// Returns `Some(Ok(()))` if handled, `Some(Err(result))` if execution
/// terminated, or `None` if not a special intrinsic.
pub fn dispatch<M: MemorySpace>(
    id: u8,
    argc: u8,
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: Option<&Scheduler>,
) -> Option<Result<(), RunResult>> {
    match id {
        intrinsics::id::GARBAGE_COLLECT => {
            let is_full = argc >= 1 && worker.x_regs[1].is_keyword();
            if is_full {
                let _ = gc::major_gc(proc, worker, realm.pool_mut(), mem);
            } else {
                let _ = gc::minor_gc(proc, worker, mem);
            }
            worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
            Some(Ok(()))
        }
        intrinsics::id::PROCESS_INFO => build_process_info(proc, realm, mem).map(|map| {
            worker.x_regs[0] = map;
            Ok(())
        }),
        intrinsics::id::EVAL => Some(handle_eval(worker, proc, mem, realm)),
        intrinsics::id::SPAWN => {
            let Some(sched) = scheduler else {
                return Some(Err(RunResult::Error(RuntimeError::ProcessLimitReached)));
            };
            Some(handle_spawn(worker, proc, mem, realm, sched))
        }
        intrinsics::id::ALIVE => {
            worker.x_regs[0] =
                scheduler.map_or(Term::FALSE, |sched| handle_alive(worker, proc, mem, sched));
            Some(Ok(()))
        }
        _ => None,
    }
}

/// Handle `eval`: push eval frame, compile form, set up execution.
///
/// Saves `htop` before compilation so heap allocations from a failed
/// compile are rolled back, preventing memory leaks.
fn handle_eval<M: MemorySpace>(
    worker: &Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
) -> Result<(), RunResult> {
    use crate::process::MAX_EVAL_DEPTH;

    let form = worker.x_regs[1];

    if proc.eval_depth >= MAX_EVAL_DEPTH {
        return Err(RunResult::Error(RuntimeError::StackOverflow));
    }

    proc.eval_stack[proc.eval_depth] = crate::process::EvalFrame {
        saved_ip: proc.ip,
        saved_chunk_addr: proc.chunk_addr,
        saved_frame_base: proc.frame_base,
        saved_y_count: proc.current_y_count,
        saved_stop: proc.stop,
    };
    proc.eval_depth += 1;

    // Save htop so failed compile doesn't leak heap
    let saved_htop = proc.htop;

    if let Ok(chunk) = crate::compiler::compile(form, proc, mem, realm) {
        if proc.write_chunk_to_heap(mem, &chunk) {
            return Ok(());
        }
        proc.eval_depth -= 1;
        proc.htop = saved_htop;
        return Err(RunResult::Error(RuntimeError::OutOfMemory));
    }

    proc.eval_depth -= 1;
    proc.htop = saved_htop;
    // Compile error is NOT OOM — use EvalError to avoid spurious GC retry
    Err(RunResult::Error(RuntimeError::EvalError))
}

/// Handle `spawn`: create new process from a compiled function in X1.
///
/// Only bare functions (FUN) are supported. Closures require capture
/// loading into registers which is not yet implemented — they are
/// rejected with a type error.
fn handle_spawn<M: MemorySpace>(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: &Scheduler,
) -> Result<(), RunResult> {
    use crate::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, ProcessId};

    let fn_term = worker.x_regs[1];
    validate_spawnable_fun(mem, fn_term).map_err(RunResult::Error)?;

    let (young_base, old_base) = realm
        .allocate_process_memory(INITIAL_YOUNG_HEAP_SIZE, INITIAL_OLD_HEAP_SIZE)
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

    let mut new_proc = Process::new(
        young_base,
        INITIAL_YOUNG_HEAP_SIZE,
        old_base,
        INITIAL_OLD_HEAP_SIZE,
    );

    let copied_fn =
        crate::realm::copy::deep_copy_term_to_process(fn_term, proc, &mut new_proc, mem)
            .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

    let (index, generation) = scheduler
        .with_process_table_mut(ProcessTable::allocate)
        .ok_or(RunResult::Error(RuntimeError::ProcessLimitReached))?;
    let pid = ProcessId::new(index, generation);

    new_proc.pid = pid;
    new_proc.parent_pid = proc.pid;
    new_proc.worker_id = worker.id;
    new_proc.chunk_addr = Some(copied_fn.to_vaddr());
    new_proc.ip = 0;

    if let (Some(ns_var), Some(core_ns)) = (
        crate::realm::get_ns_var(realm, mem),
        crate::realm::get_core_ns(realm, mem),
    ) {
        new_proc.bootstrap(ns_var, core_ns);
    }

    if let Some(pid_term) = new_proc.alloc_term_pid(mem, index, generation) {
        new_proc.pid_term = Some(pid_term);
    }

    scheduler.with_process_table_mut(|pt| pt.insert(new_proc));

    let worker_idx = worker.id.0 as usize;
    if !scheduler.enqueue_on(worker_idx, pid) {
        scheduler.with_process_table_mut(|pt| {
            pt.remove(pid);
        });
        return Err(RunResult::Error(RuntimeError::ProcessLimitReached));
    }

    worker.x_regs[0] = proc
        .alloc_term_pid(mem, index, generation)
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

    Ok(())
}

/// Validate that a term is a spawnable bare function (FUN only, not CLOSURE).
///
/// Closures are rejected because the spawned process cannot access the
/// closure's captured environment — capture loading into registers at
/// process start is not yet implemented. This will be addressed when
/// `spawn` gains closure support.
fn validate_spawnable_fun<M: MemorySpace>(mem: &M, fn_term: Term) -> Result<(), RuntimeError> {
    if !fn_term.is_boxed() {
        return Err(RuntimeError::NotCallable {
            type_name: term_type_name(mem, fn_term),
        });
    }
    let header: Header = mem.read(fn_term.to_vaddr());
    let tag = header.object_tag();
    if tag == object::CLOSURE {
        // Closures cannot be spawned yet — captures would be inaccessible
        return Err(RuntimeError::NotCallable {
            type_name: "closure (spawn requires a bare function, not a closure)",
        });
    }
    if tag != object::FUN {
        return Err(RuntimeError::NotCallable {
            type_name: term_type_name(mem, fn_term),
        });
    }
    Ok(())
}

/// Handle `alive?`: check if PID exists in process table.
fn handle_alive<M: MemorySpace>(
    worker: &Worker,
    proc: &Process,
    mem: &M,
    scheduler: &Scheduler,
) -> Term {
    let pid_term = worker.x_regs[1];
    if let Some((index, generation)) = proc.read_term_pid(mem, pid_term) {
        let pid = crate::process::ProcessId::new(index, generation);
        Term::bool(scheduler.is_alive(pid))
    } else {
        Term::FALSE
    }
}
