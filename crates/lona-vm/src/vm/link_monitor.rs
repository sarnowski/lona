// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Link, monitor, and exit intrinsic handlers.
//!
//! Implements BEAM-style process linking, monitoring, exit signals,
//! spawn-link, and spawn-monitor. These handlers are dispatched from
//! `special_intrinsics::dispatch`.

use crate::platform::MemorySpace;
use crate::process::deep_copy::deep_copy_message_to_process;
use crate::process::{Process, ProcessStatus};
use crate::realm::Realm;
use crate::scheduler::{ProcessTable, Scheduler, Worker};
use crate::term::Term;

use super::special_intrinsics::validate_spawnable_fun;
use super::{RunResult, RuntimeError};

/// Handle `link`: establish bidirectional link between current process and target.
///
/// If target is self, no-op. If target is dead, the caller receives an
/// immediate exit signal (handled in exit propagation).
pub fn handle_link<M: MemorySpace>(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: &Scheduler,
) -> Result<(), RunResult> {
    let pid_term = worker.x_regs[1];
    let Some((index, generation)) = proc.read_term_pid(mem, pid_term) else {
        return Err(RunResult::Error(RuntimeError::BadArgument {
            intrinsic: "link",
            message: "argument must be a PID",
        }));
    };
    let target_pid = crate::process::ProcessId::new(index, generation);

    // Linking self is a no-op
    if target_pid == proc.pid {
        worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
        return Ok(());
    }

    // Establish link atomically: both sides or neither
    let target_alive = scheduler.with_process_table_mut(|pt| {
        if let Some(target) = pt.get_mut(target_pid) {
            target.links.insert(proc.pid);
            true
        } else {
            false
        }
    });

    if target_alive {
        proc.links.insert(target_pid);
    } else {
        // Target is dead — send immediate :noproc exit signal (BEAM semantics)
        let noproc = realm.intern_keyword(mem, "noproc").unwrap_or(Term::NIL);
        if proc.trap_exit {
            // Trapping: deliver [:EXIT target_pid :noproc] as message
            deliver_exit_message(proc, target_pid, noproc, realm, mem);
        } else {
            // Not trapping: caller dies with :noproc
            return Err(RunResult::Exited(noproc));
        }
    }

    worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
    Ok(())
}

/// Handle `unlink`: remove bidirectional link between current process and target.
pub fn handle_unlink<M: MemorySpace>(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: &Scheduler,
) -> Result<(), RunResult> {
    let pid_term = worker.x_regs[1];
    let Some((index, generation)) = proc.read_term_pid(mem, pid_term) else {
        return Err(RunResult::Error(RuntimeError::BadArgument {
            intrinsic: "unlink",
            message: "argument must be a PID",
        }));
    };
    let target_pid = crate::process::ProcessId::new(index, generation);

    proc.links.remove(&target_pid);

    scheduler.with_process_table_mut(|pt| {
        if let Some(target) = pt.get_mut(target_pid) {
            target.links.remove(&proc.pid);
        }
    });

    worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
    Ok(())
}

/// Handle `trap-exit`: set or clear the `trap_exit` flag on the current process.
pub fn handle_trap_exit<M: MemorySpace>(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
) {
    let flag = worker.x_regs[1];
    proc.trap_exit = flag != Term::FALSE && !flag.is_nil();
    worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
}

/// Handle `exit`: terminate current process or send exit signal to another.
///
/// 1-arg: returns `RunResult::Exited(reason)`.
/// 2-arg: sends exit signal to target process, returns `:ok`.
///
/// For 2-arg, the signal is delivered immediately:
/// - `:kill` is untrappable (target receives `:killed`, links/monitors notified)
/// - Trapping target receives `[:EXIT sender reason]` message
/// - Non-trapping target with non-normal reason is killed (links/monitors notified)
/// - `:normal` signal to non-trapping target is ignored
pub fn handle_exit<M: MemorySpace>(
    argc: u8,
    worker: &mut Worker,
    proc: &Process,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: Option<&Scheduler>,
) -> Result<(), RunResult> {
    if argc == 1 {
        let reason = worker.x_regs[1];
        return Err(RunResult::Exited(reason));
    }

    let pid_term = worker.x_regs[1];
    let reason = worker.x_regs[2];

    let Some((index, generation)) = proc.read_term_pid(mem, pid_term) else {
        return Err(RunResult::Error(RuntimeError::BadArgument {
            intrinsic: "exit",
            message: "first argument must be a PID",
        }));
    };
    let target_pid = crate::process::ProcessId::new(index, generation);

    if target_pid == proc.pid {
        return Err(RunResult::Exited(reason));
    }

    let Some(scheduler) = scheduler else {
        worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
        return Ok(());
    };

    // Deliver the exit signal via the scheduler's signal delivery mechanism
    scheduler.deliver_exit_signal(target_pid, proc.pid, reason, realm, mem);

    worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
    Ok(())
}

/// Handle `monitor`: create a unidirectional monitor on a target process.
///
/// Returns a unique `HeapRef` term. If the target is dead, sends an immediate
/// `[:DOWN ref pid :noproc]` message to the caller.
pub fn handle_monitor<M: MemorySpace>(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: &Scheduler,
) -> Result<(), RunResult> {
    let pid_term = worker.x_regs[1];
    let Some((index, generation)) = proc.read_term_pid(mem, pid_term) else {
        return Err(RunResult::Error(RuntimeError::BadArgument {
            intrinsic: "monitor",
            message: "argument must be a PID",
        }));
    };
    let target_pid = crate::process::ProcessId::new(index, generation);

    let ref_id = scheduler.next_ref();
    let ref_term = proc
        .alloc_term_ref(mem, ref_id)
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

    proc.monitors_out.insert(ref_id, target_pid);

    let target_alive = scheduler.with_process_table_mut(|pt| {
        if let Some(target) = pt.get_mut(target_pid) {
            target.monitored_by.insert(ref_id, proc.pid);
            true
        } else {
            false
        }
    });

    if !target_alive {
        let down_kw = realm.intern_keyword(mem, "DOWN").unwrap_or(Term::NIL);
        let noproc_kw = realm.intern_keyword(mem, "noproc").unwrap_or(Term::NIL);
        let msg_ref = proc
            .alloc_term_ref(mem, ref_id)
            .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
        let msg_pid = proc
            .alloc_term_pid(mem, index, generation)
            .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
        let msg = proc
            .alloc_term_tuple(mem, &[down_kw, msg_ref, msg_pid, noproc_kw])
            .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
        proc.mailbox.push(msg);
        proc.monitors_out.remove(&ref_id);
    }

    worker.x_regs[0] = ref_term;
    Ok(())
}

/// Handle `demonitor`: remove a monitor by reference.
pub fn handle_demonitor<M: MemorySpace>(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: &Scheduler,
) -> Result<(), RunResult> {
    let ref_term = worker.x_regs[1];
    let Some(ref_id) = proc.read_term_ref(mem, ref_term) else {
        return Err(RunResult::Error(RuntimeError::BadArgument {
            intrinsic: "demonitor",
            message: "argument must be a reference",
        }));
    };

    if let Some(target_pid) = proc.monitors_out.remove(&ref_id) {
        scheduler.with_process_table_mut(|pt| {
            if let Some(target) = pt.get_mut(target_pid) {
                target.monitored_by.remove(&ref_id);
            }
        });
    }

    worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
    Ok(())
}

/// Handle `spawn-link`: spawn a new process and atomically link before enqueue.
pub fn handle_spawn_link<M: MemorySpace>(
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

    setup_new_process(&mut new_proc, pid, proc.pid, worker, realm, mem, copied_fn);

    // Atomically establish link before enqueue
    new_proc.links.insert(proc.pid);
    proc.links.insert(pid);

    scheduler.with_process_table_mut(|pt| pt.insert(new_proc));

    let worker_idx = worker.id.0 as usize;
    if !scheduler.enqueue_on(worker_idx, pid) {
        proc.links.remove(&pid);
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

/// Handle `spawn-monitor`: spawn and atomically monitor before enqueue.
///
/// Returns `[pid ref]` tuple.
pub fn handle_spawn_monitor<M: MemorySpace>(
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

    setup_new_process(&mut new_proc, pid, proc.pid, worker, realm, mem, copied_fn);

    // Atomically establish monitor before enqueue
    let ref_id = scheduler.next_ref();
    new_proc.monitored_by.insert(ref_id, proc.pid);
    proc.monitors_out.insert(ref_id, pid);

    scheduler.with_process_table_mut(|pt| pt.insert(new_proc));

    let worker_idx = worker.id.0 as usize;
    if !scheduler.enqueue_on(worker_idx, pid) {
        proc.monitors_out.remove(&ref_id);
        scheduler.with_process_table_mut(|pt| {
            pt.remove(pid);
        });
        return Err(RunResult::Error(RuntimeError::ProcessLimitReached));
    }

    let pid_result = proc
        .alloc_term_pid(mem, index, generation)
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
    let ref_result = proc
        .alloc_term_ref(mem, ref_id)
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
    let tuple = proc
        .alloc_term_tuple(mem, &[pid_result, ref_result])
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

    worker.x_regs[0] = tuple;
    Ok(())
}

/// Deliver a `[:EXIT sender reason]` message to a process that traps exits.
fn deliver_exit_message<M: MemorySpace>(
    target: &mut Process,
    sender_pid: crate::process::ProcessId,
    reason: Term,
    realm: &mut Realm,
    mem: &mut M,
) {
    let exit_kw = realm.intern_keyword(mem, "EXIT").unwrap_or(Term::NIL);
    let pid_term = target
        .alloc_term_pid(mem, sender_pid.index() as u32, sender_pid.generation())
        .unwrap_or(Term::NIL);
    let copied_reason = deep_copy_message_to_process(reason, target, mem).unwrap_or(reason);

    if let Some(msg) = target.alloc_term_tuple(mem, &[exit_kw, pid_term, copied_reason]) {
        target.mailbox.push(msg);
        if target.status == ProcessStatus::Waiting {
            target.status = ProcessStatus::Ready;
        }
    }
}

/// Common setup for a newly spawned process (shared by spawn, spawn-link, spawn-monitor).
pub(super) fn setup_new_process<M: MemorySpace>(
    new_proc: &mut Process,
    pid: crate::process::ProcessId,
    parent_pid: crate::process::ProcessId,
    worker: &Worker,
    realm: &Realm,
    mem: &mut M,
    copied_fn: Term,
) {
    new_proc.pid = pid;
    new_proc.parent_pid = parent_pid;
    new_proc.worker_id = worker.id;
    new_proc.chunk_addr = Some(copied_fn.to_vaddr());
    new_proc.ip = 0;

    if let (Some(ns_var), Some(core_ns)) = (
        crate::realm::get_ns_var(realm, mem),
        crate::realm::get_core_ns(realm, mem),
    ) {
        new_proc.bootstrap(ns_var, core_ns);
    }

    let index = pid.index() as u32;
    let generation = pid.generation();
    if let Some(pid_term) = new_proc.alloc_term_pid(mem, index, generation) {
        new_proc.pid_term = Some(pid_term);
    }
}
