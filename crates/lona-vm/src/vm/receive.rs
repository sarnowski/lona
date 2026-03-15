// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Receive opcode handlers for the `receive` special form.
//!
//! These opcodes are handled separately from the main dispatch because they
//! need access to the `Scheduler` for fragment draining and process blocking.

extern crate alloc;

use crate::bytecode::{decode_a, decode_bx, op};
use crate::intrinsics::XRegs;
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::scheduler::Scheduler;

use super::{RunResult, RuntimeError};

/// Execute a receive-related instruction.
///
/// These opcodes implement the `receive` special form:
/// - `RECV_PEEK`: Peek mailbox at save position
/// - `RECV_NEXT`: Advance save position and jump
/// - `RECV_ACCEPT`: Remove matched message
/// - `RECV_WAIT`: Block until message or timeout
/// - `RECV_TIMEOUT_INIT`: Set receive deadline
pub fn execute<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    instr: u32,
    opcode: u8,
    scheduler: Option<&Scheduler>,
) -> Result<u32, RunResult> {
    match opcode {
        op::RECV_PEEK => {
            let dest = decode_a(instr) as usize;
            let wait_label = decode_bx(instr) as usize;

            if let Some((_index, msg)) = proc.mailbox.peek_from_save() {
                x_regs[dest] = msg;
            } else {
                // No more messages at save position — jump to wait label
                proc.ip = wait_label;
            }
            Ok(1)
        }

        op::RECV_NEXT => {
            let target = decode_bx(instr) as usize;
            proc.mailbox.advance_save();
            proc.ip = target;
            Ok(1)
        }

        op::RECV_ACCEPT => {
            proc.mailbox.remove_at_save();
            Ok(1)
        }

        op::RECV_WAIT => {
            let recv_loop_label = decode_bx(instr) as usize;

            // 1. Drain fragment inbox (cross-worker messages delivered while taken)
            if let Some(sched) = scheduler {
                let fragments = sched.with_process_table_mut(|pt| pt.take_fragments(proc.pid));
                if let Some(frag_head) = fragments {
                    if !drain_fragments_to_mailbox(proc, mem, frag_head) {
                        return Err(RunResult::Error(RuntimeError::OutOfMemory));
                    }
                }
            }

            // 2. Check if new messages exist beyond save position
            if proc.mailbox.peek_from_save().is_some() {
                proc.ip = recv_loop_label;
                return Ok(1);
            }

            // 3. Check timeout
            if let Some(deadline) = proc.receive_deadline {
                let now = crate::platform::monotonic_ms();
                if now >= deadline {
                    // Timeout expired — fall through to timeout body
                    proc.receive_deadline = None;
                    return Ok(1);
                }
            }

            // 4. Block: set IP to re-execute RECV_WAIT when woken.
            // The VM dispatch loop already incremented IP past this instruction
            // (fixed 32-bit instructions, word-indexed IP), so decrementing by 1
            // makes the process re-execute RECV_WAIT on next scheduling.
            proc.ip -= 1;
            Err(RunResult::Waiting)
        }

        op::RECV_TIMEOUT_INIT => {
            let timeout_reg = decode_a(instr) as usize;
            let timeout_term = x_regs[timeout_reg];

            // Timeout must be a non-negative integer (BEAM raises badarg for non-integers)
            let Some(timeout_val) = timeout_term.as_small_int() else {
                return Err(RunResult::Error(RuntimeError::BadArgument {
                    intrinsic: "receive",
                    message: "timeout must be a non-negative integer",
                }));
            };

            if timeout_val < 0 {
                return Err(RunResult::Error(RuntimeError::BadArgument {
                    intrinsic: "receive",
                    message: "timeout must be a non-negative integer",
                }));
            }
            // Safe: timeout_val >= 0 guaranteed by the check above.
            let Ok(timeout_ms) = u64::try_from(timeout_val) else {
                return Err(RunResult::Error(RuntimeError::BadArgument {
                    intrinsic: "receive",
                    message: "timeout must be a non-negative integer",
                }));
            };
            let now = crate::platform::monotonic_ms();
            proc.receive_deadline = Some(now.saturating_add(timeout_ms));
            proc.mailbox.reset_save();
            Ok(1)
        }

        _ => Err(RunResult::Error(RuntimeError::InvalidOpcode(opcode))),
    }
}

/// Drain a linked list of heap fragments into a process's mailbox.
///
/// Fragments are in LIFO order (prepended during send). This function
/// reverses them to restore FIFO delivery order, then deep-copies each
/// message to the process heap and pushes it to the mailbox.
///
/// Returns `true` on success, `false` if OOM during deep copy.
pub(super) fn drain_fragments_to_mailbox<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    head: alloc::boxed::Box<crate::process::heap_fragment::HeapFragment>,
) -> bool {
    // Collect messages from fragment chain (LIFO → need to reverse for FIFO)
    let mut messages = alloc::vec::Vec::new();
    let mut current = Some(head);
    while let Some(mut frag) = current {
        messages.push(frag.message());
        current = frag.next.take();
    }

    // Reverse for FIFO order, then deep copy and push to mailbox
    for msg in messages.into_iter().rev() {
        let Some(copied) = crate::process::deep_copy::deep_copy_message_to_process(msg, proc, mem)
        else {
            return false;
        };
        proc.mailbox.push(copied);
    }
    true
}
