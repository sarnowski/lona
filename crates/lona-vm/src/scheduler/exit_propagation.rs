// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Exit signal propagation for process linking and monitoring.
//!
//! Implements BEAM-style exit signal delivery: when a process dies,
//! its linked processes receive exit signals and its monitors receive
//! `:DOWN` messages. Uses an iterative work queue to avoid stack
//! overflow on long link chains.

extern crate alloc;

use super::{ProcessTable, Scheduler};
use crate::platform::MemorySpace;
use crate::process::{Process, ProcessId, ProcessStatus};
use crate::realm::Realm;
use crate::term::Term;

impl Scheduler {
    /// Send an exit signal to a target process (2-arg `exit` intrinsic).
    ///
    /// Handles `:kill` → `:killed` transformation, trap-exit message delivery
    /// with wake-up, and cascade to target's links/monitors for non-trapping kills.
    pub(crate) fn deliver_exit_signal<M: MemorySpace>(
        &self,
        target_pid: ProcessId,
        sender_pid: ProcessId,
        reason: Term,
        realm: &mut Realm,
        mem: &mut M,
    ) {
        let is_kill = is_keyword_named(reason, "kill", realm, mem);
        let is_normal = is_keyword_named(reason, "normal", realm, mem);

        let action = {
            let mut pt = self.process_table.lock();
            let Some(target) = pt.get_mut(target_pid) else {
                return;
            };

            if is_kill {
                let killed = realm.intern_keyword(mem, "killed").unwrap_or(reason);
                let info = KilledProcessInfo::extract(target);
                target.status = ProcessStatus::Error;
                cleanup_outgoing_monitors(&info.monitors_out, &mut pt);
                SignalAction::Kill(killed, info)
            } else if target.trap_exit {
                Self::deliver_exit_as_message(target, sender_pid, reason, realm, mem);
                if target.status == ProcessStatus::Waiting {
                    target.status = ProcessStatus::Ready;
                    SignalAction::WakeTarget(target.worker_id.0 as usize)
                } else {
                    SignalAction::Done
                }
            } else if !is_normal {
                let info = KilledProcessInfo::extract(target);
                target.status = ProcessStatus::Error;
                cleanup_outgoing_monitors(&info.monitors_out, &mut pt);
                SignalAction::Kill(reason, info)
            } else {
                SignalAction::Done
            }
        };

        self.execute_signal_action(action, target_pid, realm, mem);
    }

    /// Propagate exit signals from a dying process to its links and monitors.
    ///
    /// Uses an iterative work queue to avoid stack overflow on long link chains.
    pub(super) fn handle_process_exit<M: MemorySpace>(
        &self,
        exiting_pid: ProcessId,
        exiting_proc: &Process,
        reason: &ExitReason,
        mem: &mut M,
        realm: &mut Realm,
    ) {
        let links = exiting_proc.links.clone();
        let monitored_by = exiting_proc.monitored_by.clone();
        let reason_term = reason.to_term(realm, mem);

        // Deliver :DOWN to all monitoring processes
        for (ref_id, monitor_pid) in &monitored_by {
            self.deliver_down_message(*monitor_pid, *ref_id, exiting_pid, reason_term, realm, mem);
        }

        // Clean up exiting process's outgoing monitors on target processes
        {
            let monitors_out = exiting_proc.monitors_out.clone();
            let mut pt = self.process_table.lock();
            for (ref_id, target_pid) in &monitors_out {
                if let Some(target) = pt.get_mut(*target_pid) {
                    target.monitored_by.remove(ref_id);
                }
            }
        }

        // Propagate exit signals to linked processes
        let mut work_queue: alloc::vec::Vec<(ProcessId, ProcessId, Term)> = alloc::vec::Vec::new();
        for linked_pid in &links {
            work_queue.push((*linked_pid, exiting_pid, reason_term));
        }
        while let Some((target_pid, source_pid, exit_reason)) = work_queue.pop() {
            self.propagate_signal_to_one(
                target_pid,
                source_pid,
                exit_reason,
                &mut work_queue,
                realm,
                mem,
            );
        }

        // Clean up link/monitor references from other processes.
        // Note: `propagate_signal_to_one` already removes the reverse link for
        // processes it visits, so some of these removes are no-ops. This pass
        // handles any processes that were skipped (e.g., dead between clone and
        // propagation) and ensures the cleanup is complete.
        let mut pt = self.process_table.lock();
        for linked_pid in &links {
            if let Some(linked) = pt.get_mut(*linked_pid) {
                linked.links.remove(&exiting_pid);
            }
        }
        for (ref_id, monitor_pid) in &monitored_by {
            if let Some(monitor) = pt.get_mut(*monitor_pid) {
                monitor.monitors_out.remove(ref_id);
            }
        }
    }

    /// Propagate a single exit signal to one linked process.
    fn propagate_signal_to_one<M: MemorySpace>(
        &self,
        target_pid: ProcessId,
        source_pid: ProcessId,
        exit_reason: Term,
        work_queue: &mut alloc::vec::Vec<(ProcessId, ProcessId, Term)>,
        realm: &mut Realm,
        mem: &mut M,
    ) {
        let is_normal = is_keyword_named(exit_reason, "normal", realm, mem);
        let is_kill = is_keyword_named(exit_reason, "kill", realm, mem);

        let action = {
            let mut pt = self.process_table.lock();
            let Some(target) = pt.get_mut(target_pid) else {
                return;
            };
            target.links.remove(&source_pid);

            if is_kill {
                let killed = realm.intern_keyword(mem, "killed").unwrap_or(exit_reason);
                let info = KilledProcessInfo::extract(target);
                target.status = ProcessStatus::Error;
                cleanup_outgoing_monitors(&info.monitors_out, &mut pt);
                SignalAction::Kill(killed, info)
            } else if target.trap_exit {
                Self::deliver_exit_as_message(target, source_pid, exit_reason, realm, mem);
                if target.status == ProcessStatus::Waiting {
                    target.status = ProcessStatus::Ready;
                    SignalAction::WakeTarget(target.worker_id.0 as usize)
                } else {
                    SignalAction::Done
                }
            } else if !is_normal {
                let info = KilledProcessInfo::extract(target);
                target.status = ProcessStatus::Error;
                cleanup_outgoing_monitors(&info.monitors_out, &mut pt);
                SignalAction::Kill(exit_reason, info)
            } else {
                SignalAction::Done
            }
        };

        match action {
            SignalAction::Kill(reason, info) => {
                for (ref_id, monitor_pid) in &info.monitored_by {
                    self.deliver_down_message(
                        *monitor_pid,
                        *ref_id,
                        info.dying_pid,
                        reason,
                        realm,
                        mem,
                    );
                }
                for linked_pid in &info.links {
                    work_queue.push((*linked_pid, info.dying_pid, reason));
                }
            }
            SignalAction::WakeTarget(worker_idx) => {
                self.enqueue_on(worker_idx, target_pid);
            }
            SignalAction::Done => {}
        }
    }

    /// Execute a signal action (shared by `deliver_exit_signal` and `propagate_signal_to_one`).
    fn execute_signal_action<M: MemorySpace>(
        &self,
        action: SignalAction,
        target_pid: ProcessId,
        realm: &mut Realm,
        mem: &mut M,
    ) {
        match action {
            SignalAction::Kill(kill_reason, info) => {
                for (ref_id, monitor_pid) in &info.monitored_by {
                    self.deliver_down_message(
                        *monitor_pid,
                        *ref_id,
                        info.dying_pid,
                        kill_reason,
                        realm,
                        mem,
                    );
                }
                let mut work_queue: alloc::vec::Vec<(ProcessId, ProcessId, Term)> =
                    alloc::vec::Vec::new();
                for linked_pid in &info.links {
                    work_queue.push((*linked_pid, info.dying_pid, kill_reason));
                }
                while let Some((tp, sp, er)) = work_queue.pop() {
                    self.propagate_signal_to_one(tp, sp, er, &mut work_queue, realm, mem);
                }
            }
            SignalAction::WakeTarget(worker_idx) => {
                self.enqueue_on(worker_idx, target_pid);
            }
            SignalAction::Done => {}
        }
    }

    /// Deliver `[:EXIT sender reason]` to a process that traps exits.
    fn deliver_exit_as_message<M: MemorySpace>(
        target: &mut Process,
        source_pid: ProcessId,
        reason: Term,
        realm: &mut Realm,
        mem: &mut M,
    ) {
        let exit_kw = realm.intern_keyword(mem, "EXIT").unwrap_or(Term::NIL);
        let pid_term = target
            .alloc_term_pid(mem, source_pid.index() as u32, source_pid.generation())
            .unwrap_or(Term::NIL);
        let copied_reason =
            crate::process::deep_copy::deep_copy_message_to_process(reason, target, mem)
                .unwrap_or(reason);
        if let Some(msg) = target.alloc_term_tuple(mem, &[exit_kw, pid_term, copied_reason]) {
            target.mailbox.push(msg);
        }
    }

    /// Deliver `[:DOWN ref pid reason]` to a monitoring process.
    fn deliver_down_message<M: MemorySpace>(
        &self,
        monitor_pid: ProcessId,
        ref_id: u64,
        exiting_pid: ProcessId,
        reason: Term,
        realm: &mut Realm,
        mem: &mut M,
    ) {
        let mut pt = self.process_table.lock();
        let Some(monitor) = pt.get_mut(monitor_pid) else {
            return;
        };
        monitor.monitors_out.remove(&ref_id);

        let down_kw = realm.intern_keyword(mem, "DOWN").unwrap_or(Term::NIL);
        let ref_term = monitor.alloc_term_ref(mem, ref_id).unwrap_or(Term::NIL);
        let pid_term = monitor
            .alloc_term_pid(mem, exiting_pid.index() as u32, exiting_pid.generation())
            .unwrap_or(Term::NIL);
        let copied_reason =
            crate::process::deep_copy::deep_copy_message_to_process(reason, monitor, mem)
                .unwrap_or(reason);

        if let Some(msg) =
            monitor.alloc_term_tuple(mem, &[down_kw, ref_term, pid_term, copied_reason])
        {
            monitor.mailbox.push(msg);
            if monitor.status == ProcessStatus::Waiting {
                monitor.status = ProcessStatus::Ready;
                let w = monitor.worker_id.0 as usize;
                // Release lock before enqueue to prevent deadlock
                drop(pt);
                self.enqueue_on(w, monitor_pid);
            }
        }
    }
}

/// Reason for a process exit.
pub(super) enum ExitReason {
    /// Normal completion (`:normal` keyword).
    Normal,
    /// Runtime error (`:error` keyword).
    Error,
    /// Explicit `exit(reason)` call with a term.
    Term(Term),
}

impl ExitReason {
    /// Convert to a Term for message delivery.
    pub(super) fn to_term<M: MemorySpace>(&self, realm: &mut Realm, mem: &mut M) -> Term {
        match self {
            Self::Normal => realm.intern_keyword(mem, "normal").unwrap_or(Term::NIL),
            Self::Error => realm.intern_keyword(mem, "error").unwrap_or(Term::NIL),
            Self::Term(t) => *t,
        }
    }
}

/// Check if a term is a keyword with a specific name.
fn is_keyword_named<M: MemorySpace>(term: Term, name: &str, realm: &Realm, mem: &M) -> bool {
    term.is_keyword()
        && realm.keyword_name(mem, term.as_keyword_index().unwrap_or(u32::MAX)) == Some(name)
}

/// Information extracted from a dying process.
struct KilledProcessInfo {
    dying_pid: ProcessId,
    links: alloc::collections::BTreeSet<ProcessId>,
    monitored_by: alloc::collections::BTreeMap<u64, ProcessId>,
    monitors_out: alloc::collections::BTreeMap<u64, ProcessId>,
}

impl KilledProcessInfo {
    /// Extract link/monitor data from a process (does not modify the process).
    fn extract(proc: &Process) -> Self {
        Self {
            dying_pid: proc.pid,
            links: proc.links.clone(),
            monitored_by: proc.monitored_by.clone(),
            monitors_out: proc.monitors_out.clone(),
        }
    }
}

/// Action determined while holding the process table lock.
enum SignalAction {
    /// Kill target and cascade to its links/monitors.
    Kill(Term, KilledProcessInfo),
    /// Wake a waiting target (already delivered message).
    WakeTarget(usize),
    /// No further action needed.
    Done,
}

/// Clean up a dying process's outgoing monitors in the process table.
fn cleanup_outgoing_monitors(
    monitors_out: &alloc::collections::BTreeMap<u64, ProcessId>,
    pt: &mut ProcessTable,
) {
    for (ref_id, monitored_pid) in monitors_out {
        if let Some(monitored) = pt.get_mut(*monitored_pid) {
            monitored.monitored_by.remove(ref_id);
        }
    }
}
