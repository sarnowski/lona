// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Special intrinsic handlers for the VM.
//!
//! These intrinsics need access to Worker, Realm, or `Scheduler` that
//! the normal `call_intrinsic` dispatch doesn't provide.
//!
//! Link/monitor/exit handlers are in `link_monitor.rs`.

extern crate alloc;

use alloc::boxed::Box;

use crate::gc;
use crate::intrinsics;
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::process::deep_copy::{deep_copy_message_to_fragment, deep_copy_message_to_process};
use crate::process::heap_fragment::HeapFragment;
use crate::realm::Realm;
use crate::scheduler::{Scheduler, Worker};
use crate::term::Term;
use crate::term::header::Header;
use crate::term::tag::object;

use super::link_monitor;
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
        intrinsics::id::SEND => Some(handle_send(worker, proc, mem, realm, scheduler)),
        intrinsics::id::LINK => {
            let Some(sched) = scheduler else {
                worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
                return Some(Ok(()));
            };
            Some(link_monitor::handle_link(worker, proc, mem, realm, sched))
        }
        intrinsics::id::UNLINK => {
            let Some(sched) = scheduler else {
                worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
                return Some(Ok(()));
            };
            Some(link_monitor::handle_unlink(worker, proc, mem, realm, sched))
        }
        intrinsics::id::TRAP_EXIT => {
            link_monitor::handle_trap_exit(worker, proc, mem, realm);
            Some(Ok(()))
        }
        intrinsics::id::MONITOR => {
            let Some(sched) = scheduler else {
                worker.x_regs[0] = Term::NIL;
                return Some(Ok(()));
            };
            Some(link_monitor::handle_monitor(
                worker, proc, mem, realm, sched,
            ))
        }
        intrinsics::id::DEMONITOR => {
            let Some(sched) = scheduler else {
                worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
                return Some(Ok(()));
            };
            Some(link_monitor::handle_demonitor(
                worker, proc, mem, realm, sched,
            ))
        }
        intrinsics::id::EXIT => Some(link_monitor::handle_exit(
            argc, worker, proc, mem, realm, scheduler,
        )),
        intrinsics::id::SPAWN_LINK => {
            let Some(sched) = scheduler else {
                return Some(Err(RunResult::Error(RuntimeError::ProcessLimitReached)));
            };
            Some(link_monitor::handle_spawn_link(
                worker, proc, mem, realm, sched,
            ))
        }
        intrinsics::id::SPAWN_MONITOR => {
            let Some(sched) = scheduler else {
                return Some(Err(RunResult::Error(RuntimeError::ProcessLimitReached)));
            };
            Some(link_monitor::handle_spawn_monitor(
                worker, proc, mem, realm, sched,
            ))
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

    let (index, generation) = link_monitor::allocate_slot_with_growth(scheduler, realm)?;
    let pid = ProcessId::new(index, generation);

    link_monitor::setup_new_process(&mut new_proc, pid, proc.pid, worker, realm, mem, copied_fn);

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
pub(super) fn validate_spawnable_fun<M: MemorySpace>(
    mem: &M,
    fn_term: Term,
) -> Result<(), RuntimeError> {
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

/// Handle `send`: deliver a message to a process's mailbox.
///
/// Send paths:
/// 1. Self-send: deep copy message to own heap, push to own mailbox
/// 2. Direct copy: receiver is in table -> deep copy to receiver's heap + mailbox
/// 3. Fragment: receiver is taken -> allocate fragment, deep copy, push to slot inbox
/// 4. Dead PID: silently ignored (BEAM semantics), returns `:ok`
fn handle_send<M: MemorySpace>(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: Option<&Scheduler>,
) -> Result<(), RunResult> {
    let pid_term = worker.x_regs[1];
    let message = worker.x_regs[2];

    let Some((index, generation)) = proc.read_term_pid(mem, pid_term) else {
        return Err(RunResult::Error(RuntimeError::BadArgument {
            intrinsic: "send",
            message: "first argument must be a PID",
        }));
    };
    let target_pid = crate::process::ProcessId::new(index, generation);

    if target_pid == proc.pid {
        let mut copied = deep_copy_message_to_process(message, proc, mem);
        if copied.is_none() {
            let _ = gc::minor_gc(proc, worker, mem);
            copied = deep_copy_message_to_process(message, proc, mem);
        }
        let copied = copied.ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
        proc.mailbox.push(copied);
        worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
        return Ok(());
    }

    let Some(scheduler) = scheduler else {
        worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
        return Ok(());
    };

    let delivered = scheduler.with_process_table_mut(|pt| {
        if let Some(receiver) = pt.get_mut(target_pid) {
            if let Some(copied) = deep_copy_message_to_process(message, receiver, mem) {
                receiver.mailbox.push(copied);
                let was_waiting = receiver.status == crate::process::ProcessStatus::Waiting;
                if was_waiting {
                    receiver.status = crate::process::ProcessStatus::Ready;
                }
                return SendResult::Delivered {
                    wake_worker: if was_waiting {
                        Some(receiver.worker_id.0 as usize)
                    } else {
                        None
                    },
                };
            }
            return SendResult::Taken;
        }

        if pt.is_taken(target_pid) {
            return SendResult::Taken;
        }

        SendResult::Dead
    });

    match delivered {
        SendResult::Delivered { wake_worker } => {
            if let Some(worker_idx) = wake_worker {
                scheduler.enqueue_on(worker_idx, target_pid);
            }
        }
        SendResult::Taken => {
            send_via_fragment(message, target_pid, mem, realm, scheduler)?;
        }
        SendResult::Dead => {}
    }

    worker.x_regs[0] = realm.intern_keyword(mem, "ok").unwrap_or(Term::TRUE);
    Ok(())
}

/// Result of trying to deliver a message via the `ProcessTable`.
enum SendResult {
    /// Message delivered directly to receiver's heap and mailbox.
    Delivered {
        /// If receiver was Waiting, the worker index to enqueue it on.
        wake_worker: Option<usize>,
    },
    /// Receiver is taken or heap-full (use fragment fallback).
    Taken,
    /// Receiver PID is dead/invalid.
    Dead,
}

/// Minimum fragment size in bytes.
const MIN_FRAGMENT_SIZE: usize = 64;

/// Maximum retries when fragment is too small for the message.
const MAX_FRAGMENT_RETRIES: usize = 4;

/// Allocate a heap fragment and deliver message to taken process's slot inbox.
fn send_via_fragment<M: MemorySpace>(
    message: Term,
    target_pid: crate::process::ProcessId,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: &Scheduler,
) -> Result<(), RunResult> {
    let mut frag_size = estimate_copy_size(mem, message).max(MIN_FRAGMENT_SIZE);

    for _ in 0..=MAX_FRAGMENT_RETRIES {
        let frag_base = realm
            .pool_mut()
            .allocate(frag_size, 8)
            .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

        let mut fragment = HeapFragment::new(frag_base, frag_size);

        if let Some(copied) = deep_copy_message_to_fragment(message, &mut fragment, mem) {
            fragment.set_message(copied);
            scheduler.with_process_table_mut(|pt| {
                pt.push_fragment(target_pid, Box::new(fragment));
            });
            return Ok(());
        }

        frag_size = frag_size.saturating_mul(2);
    }

    Err(RunResult::Error(RuntimeError::OutOfMemory))
}

/// Estimate the size needed to deep copy a term.
fn estimate_copy_size<M: MemorySpace>(mem: &M, term: Term) -> usize {
    if term.is_immediate() || term.is_nil() {
        return 0;
    }

    if term.is_list() {
        return 128;
    }

    if term.is_boxed() {
        let header: Header = mem.read(term.to_vaddr());
        return header.object_size().saturating_mul(2);
    }

    64
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
