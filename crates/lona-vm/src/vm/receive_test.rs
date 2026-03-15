// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for receive opcode handlers.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::Vaddr;
use crate::bytecode::{encode_abx, op};
use crate::platform::MockVSpace;
use crate::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process};
use crate::term::Term;
use crate::vm::{RunResult, RuntimeError};

/// Create a process for testing.
fn make_process() -> (MockVSpace, Process) {
    let mem = MockVSpace::new(64 * 1024, Vaddr::new(0x1_0000));
    let proc = Process::new(
        Vaddr::new(0x1_0000),
        INITIAL_YOUNG_HEAP_SIZE,
        Vaddr::new(0x1_0000 + INITIAL_YOUNG_HEAP_SIZE as u64),
        INITIAL_OLD_HEAP_SIZE,
    );
    (mem, proc)
}

// --- RECV_PEEK tests ---

#[test]
fn recv_peek_empty_mailbox_jumps() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];
    proc.ip = 5; // Will be decremented if we need to track

    let instr = encode_abx(op::RECV_PEEK, 0, 42); // dest=X0, wait_label=42
    let result =
        super::receive::execute(&mut x_regs, &mut proc, &mut mem, instr, op::RECV_PEEK, None);

    assert_eq!(result, Ok(1));
    assert_eq!(proc.ip, 42); // Jumped to wait label
}

#[test]
fn recv_peek_with_message_loads_register() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    let msg = Term::small_int(99).unwrap();
    proc.mailbox.push(msg);

    let instr = encode_abx(op::RECV_PEEK, 3, 42); // dest=X3, wait_label=42
    let result =
        super::receive::execute(&mut x_regs, &mut proc, &mut mem, instr, op::RECV_PEEK, None);

    assert_eq!(result, Ok(1));
    assert_eq!(x_regs[3], msg); // Message loaded into X3
}

// --- RECV_NEXT tests ---

#[test]
fn recv_next_advances_save_and_jumps() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    proc.mailbox.push(Term::small_int(1).unwrap());
    proc.mailbox.push(Term::small_int(2).unwrap());
    assert_eq!(proc.mailbox.save_position(), 0);

    let instr = encode_abx(op::RECV_NEXT, 0, 10); // target=10
    let result =
        super::receive::execute(&mut x_regs, &mut proc, &mut mem, instr, op::RECV_NEXT, None);

    assert_eq!(result, Ok(1));
    assert_eq!(proc.mailbox.save_position(), 1);
    assert_eq!(proc.ip, 10); // Jumped to target
}

// --- RECV_ACCEPT tests ---

#[test]
fn recv_accept_removes_message() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    proc.mailbox.push(Term::small_int(1).unwrap());
    proc.mailbox.push(Term::small_int(2).unwrap());
    assert_eq!(proc.mailbox.len(), 2);

    let instr = encode_abx(op::RECV_ACCEPT, 0, 0);
    let result = super::receive::execute(
        &mut x_regs,
        &mut proc,
        &mut mem,
        instr,
        op::RECV_ACCEPT,
        None,
    );

    assert_eq!(result, Ok(1));
    assert_eq!(proc.mailbox.len(), 1); // One message removed
    assert_eq!(proc.mailbox.save_position(), 0); // Save reset
}

// --- RECV_WAIT tests ---

#[test]
fn recv_wait_no_messages_no_timeout_blocks() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];
    proc.ip = 5; // Simulates instruction was at index 4, IP advanced to 5

    let instr = encode_abx(op::RECV_WAIT, 0, 10); // recv_loop_label=10
    let result =
        super::receive::execute(&mut x_regs, &mut proc, &mut mem, instr, op::RECV_WAIT, None);

    assert_eq!(result, Err(RunResult::Waiting));
    assert_eq!(proc.ip, 4); // IP decremented to re-execute RECV_WAIT
}

#[test]
fn recv_wait_with_messages_jumps() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    proc.mailbox.push(Term::small_int(42).unwrap());

    let instr = encode_abx(op::RECV_WAIT, 0, 10); // recv_loop_label=10
    let result =
        super::receive::execute(&mut x_regs, &mut proc, &mut mem, instr, op::RECV_WAIT, None);

    assert_eq!(result, Ok(1));
    assert_eq!(proc.ip, 10); // Jumped to recv_loop
}

#[test]
fn recv_wait_expired_timeout_falls_through() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    // Set deadline to 0 (already expired since now >= 0)
    proc.receive_deadline = Some(0);

    let instr = encode_abx(op::RECV_WAIT, 0, 10);
    let result =
        super::receive::execute(&mut x_regs, &mut proc, &mut mem, instr, op::RECV_WAIT, None);

    assert_eq!(result, Ok(1));
    assert!(proc.receive_deadline.is_none()); // Cleared after timeout
}

// --- RECV_TIMEOUT_INIT tests ---

#[test]
fn recv_timeout_init_sets_deadline() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    x_regs[2] = Term::small_int(100).unwrap(); // 100ms timeout

    let instr = encode_abx(op::RECV_TIMEOUT_INIT, 2, 0); // timeout_reg=X2
    let result = super::receive::execute(
        &mut x_regs,
        &mut proc,
        &mut mem,
        instr,
        op::RECV_TIMEOUT_INIT,
        None,
    );

    assert_eq!(result, Ok(1));
    assert!(proc.receive_deadline.is_some());
    assert_eq!(proc.mailbox.save_position(), 0); // Save reset
}

#[test]
fn recv_timeout_init_zero_timeout() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    x_regs[0] = Term::small_int(0).unwrap(); // 0ms timeout (non-blocking)

    let instr = encode_abx(op::RECV_TIMEOUT_INIT, 0, 0);
    let result = super::receive::execute(
        &mut x_regs,
        &mut proc,
        &mut mem,
        instr,
        op::RECV_TIMEOUT_INIT,
        None,
    );

    assert_eq!(result, Ok(1));
    // Deadline should be <= now (immediate timeout)
    let deadline = proc.receive_deadline.unwrap();
    let now = crate::platform::monotonic_ms();
    assert!(now >= deadline);
}

// --- RECV_TIMEOUT_INIT edge cases ---

#[test]
fn recv_timeout_init_negative_is_error() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    x_regs[0] = Term::small_int(-500).unwrap();

    let instr = encode_abx(op::RECV_TIMEOUT_INIT, 0, 0);
    let result = super::receive::execute(
        &mut x_regs,
        &mut proc,
        &mut mem,
        instr,
        op::RECV_TIMEOUT_INIT,
        None,
    );

    assert!(matches!(
        result,
        Err(RunResult::Error(RuntimeError::BadArgument { .. }))
    ));
}

#[test]
fn recv_timeout_init_non_integer_is_error() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    // nil is not an integer
    x_regs[0] = Term::NIL;

    let instr = encode_abx(op::RECV_TIMEOUT_INIT, 0, 0);
    let result = super::receive::execute(
        &mut x_regs,
        &mut proc,
        &mut mem,
        instr,
        op::RECV_TIMEOUT_INIT,
        None,
    );

    assert!(matches!(
        result,
        Err(RunResult::Error(RuntimeError::BadArgument { .. }))
    ));
}

#[test]
fn recv_wait_future_deadline_blocks_without_clearing() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];
    proc.ip = 5;

    // Set deadline far in the future (never expires)
    proc.receive_deadline = Some(u64::MAX);

    let instr = encode_abx(op::RECV_WAIT, 0, 10);
    let result =
        super::receive::execute(&mut x_regs, &mut proc, &mut mem, instr, op::RECV_WAIT, None);

    assert_eq!(result, Err(RunResult::Waiting));
    assert_eq!(proc.ip, 4); // IP decremented
    // Deadline should NOT be cleared (not expired)
    assert_eq!(proc.receive_deadline, Some(u64::MAX));
}

#[test]
fn recv_accept_at_nonzero_save_position() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    let msg_a = Term::small_int(10).unwrap();
    let msg_b = Term::small_int(20).unwrap();
    let msg_c = Term::small_int(30).unwrap();
    proc.mailbox.push(msg_a);
    proc.mailbox.push(msg_b);
    proc.mailbox.push(msg_c);

    // Advance save to position 1 (skip msg_a)
    proc.mailbox.advance_save();
    assert_eq!(proc.mailbox.save_position(), 1);

    // RECV_ACCEPT removes msg_b (at save position 1)
    let instr = encode_abx(op::RECV_ACCEPT, 0, 0);
    let result = super::receive::execute(
        &mut x_regs,
        &mut proc,
        &mut mem,
        instr,
        op::RECV_ACCEPT,
        None,
    );

    assert_eq!(result, Ok(1));
    assert_eq!(proc.mailbox.len(), 2); // msg_a and msg_c remain
    assert_eq!(proc.mailbox.save_position(), 0); // Save reset

    // Verify remaining messages: msg_a, msg_c
    let (_, m1) = proc.mailbox.peek_from_save().unwrap();
    assert_eq!(m1, msg_a);
    proc.mailbox.advance_save();
    let (_, m2) = proc.mailbox.peek_from_save().unwrap();
    assert_eq!(m2, msg_c);
}

// --- Fragment drain FIFO ordering ---

#[test]
fn drain_fragments_preserves_fifo_order() {
    use crate::process::heap_fragment::HeapFragment;

    let (mut mem, mut proc) = make_process();

    // Simulate 3 fragments prepended in LIFO order (msg3 → msg2 → msg1)
    // After drain, mailbox should have msg1, msg2, msg3 (FIFO)
    let msg1 = Term::small_int(1).unwrap();
    let msg2 = Term::small_int(2).unwrap();
    let msg3 = Term::small_int(3).unwrap();

    let mut frag1 = HeapFragment::new(Vaddr::new(0x5_0000), 64);
    frag1.set_message(msg1);
    let mut frag2 = HeapFragment::new(Vaddr::new(0x5_1000), 64);
    frag2.set_message(msg2);
    let mut frag3 = HeapFragment::new(Vaddr::new(0x5_2000), 64);
    frag3.set_message(msg3);

    // Chain: frag3 → frag2 → frag1 (LIFO prepend order)
    frag2.next = Some(Box::new(frag1));
    frag3.next = Some(Box::new(frag2));

    let result = super::receive::drain_fragments_to_mailbox(&mut proc, &mut mem, Box::new(frag3));
    assert!(result, "drain should succeed");

    // Mailbox should have messages in FIFO order: 1, 2, 3
    assert_eq!(proc.mailbox.len(), 3);
    let (_, m1) = proc.mailbox.peek_from_save().unwrap();
    assert_eq!(m1, msg1);
    proc.mailbox.advance_save();
    let (_, m2) = proc.mailbox.peek_from_save().unwrap();
    assert_eq!(m2, msg2);
    proc.mailbox.advance_save();
    let (_, m3) = proc.mailbox.peek_from_save().unwrap();
    assert_eq!(m3, msg3);
}

// --- RunResult::Waiting tests ---

#[test]
fn waiting_is_not_terminal() {
    assert!(!RunResult::Waiting.is_terminal());
}

#[test]
fn waiting_is_waiting() {
    assert!(RunResult::Waiting.is_waiting());
}

#[test]
fn waiting_is_not_yielded() {
    assert!(!RunResult::Waiting.is_yielded());
}

// --- Invalid opcode ---

#[test]
fn invalid_recv_opcode() {
    let (mut mem, mut proc) = make_process();
    let mut x_regs = [Term::NIL; 256];

    let result = super::receive::execute(&mut x_regs, &mut proc, &mut mem, 0, 63, None);
    assert!(matches!(
        result,
        Err(RunResult::Error(RuntimeError::InvalidOpcode(63)))
    ));
}
