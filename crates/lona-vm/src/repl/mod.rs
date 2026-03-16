// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! REPL (Read-Eval-Print Loop) for Lonala.
//!
//! This module provides a REPL that reads Lonala expressions,
//! compiles them to bytecode, executes them, and prints the results.

#[cfg(test)]
mod mod_test;

use crate::compiler::{self, CompileError};
use crate::intrinsics::IntrinsicError;
use crate::platform::MemorySpace;
#[cfg(test)]
use crate::process::WorkerId;
use crate::process::{Process, ProcessId, ProcessStatus};
use crate::reader::{ReadError, read};
use crate::realm::Realm;
use crate::scheduler::{Scheduler, Worker};
use crate::sync::SpinRwLock;
use crate::term::printer::print_term;
use crate::uart::{Uart, UartExt};
#[cfg(test)]
use crate::vm;
use crate::vm::{RuntimeError, Vm};

/// Maximum line buffer size.
const LINE_BUFFER_SIZE: usize = 256;

/// Run the scheduler-integrated REPL loop.
///
/// The REPL process stays "taken" from the `ProcessTable` at all times.
/// This prevents background workers from picking it up if a message or
/// exit signal wakes it. Messages targeting the REPL while it is taken
/// go to the slot's fragment inbox and are drained before each expression.
///
/// Worker 0 manages the REPL directly and also ticks other processes
/// via `tick_worker` while waiting for UART input.
///
/// This function never returns under normal operation.
pub fn run_scheduled<M: MemorySpace, U: Uart>(
    worker: &mut Worker,
    repl_pid: ProcessId,
    mem: &mut M,
    realm: &'static SpinRwLock<Realm>,
    scheduler: &'static Scheduler,
    uart: &mut U,
) -> ! {
    // Take the REPL process once — it stays taken for the REPL's lifetime.
    // The slot remains allocated (generation preserved), so is_alive(repl_pid)
    // returns true via the is_taken check. Messages go to fragment inbox.
    // The process was just inserted by the caller, so take always succeeds.
    let Some(mut proc) = scheduler.with_process_table_mut(|pt| pt.take(repl_pid)) else {
        loop {
            core::hint::spin_loop();
        }
    };

    let mut line_buf = [0u8; LINE_BUFFER_SIZE];

    loop {
        // Drain fragments and pending exit signals accumulated while REPL was taken
        drain_repl_fragments(repl_pid, &mut proc, mem, scheduler);
        drain_repl_pending_signals(repl_pid, &mut proc, realm, mem, scheduler);

        // Print prompt
        uart.write_str("lona> ");

        // Read line, ticking background processes while waiting for input
        let len = read_line_with_ticks(uart, &mut line_buf, worker, realm, scheduler, mem);

        if len == 0 {
            continue;
        }

        let Ok(line) = core::str::from_utf8(&line_buf[..len]) else {
            uart.write_line("Error: invalid UTF-8");
            continue;
        };

        // Drain fragments and pending signals (may have arrived during input)
        drain_repl_fragments(repl_pid, &mut proc, mem, scheduler);
        drain_repl_pending_signals(repl_pid, &mut proc, realm, mem, scheduler);

        // Parse (needs realm write lock for interning)
        let expr = {
            let mut realm_guard = realm.write();
            match read(line, &mut proc, &mut realm_guard, mem) {
                Ok(Some(v)) => v,
                Ok(None) => continue,
                Err(e) => {
                    uart.write_str("Error: ");
                    print_read_error(&e, uart);
                    uart.write_byte(b'\n');
                    continue;
                }
            }
        };

        // Compile (needs realm write lock)
        let chunk = {
            let mut realm_guard = realm.write();
            match compiler::compile(expr, &mut proc, mem, &mut realm_guard) {
                Ok(c) => c,
                Err(e) => {
                    uart.write_str("Error: ");
                    print_compile_error(e, uart);
                    uart.write_byte(b'\n');
                    worker.reset_x_regs();
                    proc.reset();
                    continue;
                }
            }
        };

        // Serialize chunk to heap (try GC on failure)
        if !proc.write_chunk_to_heap(mem, &chunk) {
            let _ = crate::gc::minor_gc(&mut proc, worker, mem);
            if !proc.write_chunk_to_heap(mem, &chunk) {
                {
                    let mut realm_guard = realm.write();
                    let _ = crate::gc::major_gc(&mut proc, worker, realm_guard.pool_mut(), mem);
                }
                if !proc.write_chunk_to_heap(mem, &chunk) {
                    uart.write_line("Error: out of memory");
                    worker.reset_x_regs();
                    proc.reset();
                    continue;
                }
            }
        }

        // Execute with scheduler (handles yielding internally)
        proc.reset_reductions();
        let result = execute_repl_expression(worker, &mut proc, mem, realm, scheduler);

        match result {
            Ok(value) => {
                let realm_guard = realm.read();
                print_term(value, &proc, &realm_guard, mem, uart);
                uart.write_byte(b'\n');
            }
            Err(e) => {
                uart.write_str("Error: ");
                print_runtime_error(&e, uart);
                uart.write_byte(b'\n');
            }
        }

        // Reset for next expression (REPL stays taken — not put back)
        worker.reset_x_regs();
        proc.reset();
    }
}

/// Read a line from UART, ticking background processes while waiting.
///
/// Polls UART character-by-character. When no data is available, runs
/// one `tick_worker` to service background processes.
fn read_line_with_ticks<M: MemorySpace, U: Uart>(
    uart: &mut U,
    buf: &mut [u8],
    worker: &mut Worker,
    realm: &'static SpinRwLock<Realm>,
    scheduler: &'static Scheduler,
    mem: &mut M,
) -> usize {
    let mut pos = 0;

    loop {
        if uart.can_read() {
            let byte = uart.read_byte();

            // Echo character
            if byte == b'\r' || byte == b'\n' {
                uart.write_byte(b'\n');
                return pos;
            }

            // Backspace handling
            if byte == 0x7F || byte == 0x08 {
                if pos > 0 {
                    pos -= 1;
                    uart.write_str("\x08 \x08");
                }
                continue;
            }

            // Store if buffer has space
            if pos < buf.len() {
                buf[pos] = byte;
                pos += 1;
                uart.write_byte(byte);
            }
        } else {
            // No UART data — tick background processes
            let _ = scheduler.tick_worker(worker, realm, mem);
        }
    }
}

/// Execute a REPL expression, handling yield and wait.
///
/// Runs `Vm::run` in a loop, resetting reductions on yield. The realm
/// write lock is held per `Vm::run` call (not batched) since the REPL
/// is the primary Worker 0 activity.
///
/// **Limitation:** When `Waiting` (receive with no matching message), this
/// loops without ticking background processes. Self-send works (message
/// is already in mailbox), but receiving from a spawned process will
/// busy-wait until the timeout expires. Background workers still run
/// spawned processes independently; messages arrive via fragments which
/// are drained between expressions, not mid-expression.
fn execute_repl_expression<M: MemorySpace>(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &'static SpinRwLock<Realm>,
    scheduler: &'static Scheduler,
) -> Result<crate::term::Term, RuntimeError> {
    use crate::vm::RunResult;

    proc.status = ProcessStatus::Running;

    loop {
        proc.reset_reductions();

        let result = {
            let mut realm_guard = realm.write();
            Vm::run(worker, proc, mem, &mut realm_guard, Some(scheduler))
        };

        match result {
            RunResult::Completed(value) | RunResult::Exited(value) => return Ok(value),
            RunResult::Error(e) => return Err(e),
            RunResult::Yielded | RunResult::Waiting => {}
        }
    }
}

/// Drain heap fragments from the REPL's slot inbox into its mailbox.
///
/// When the REPL is taken (always, in this design), messages sent to it
/// via `send` go through the fragment path. This drains those fragments
/// into the process mailbox so `receive` can find them.
fn drain_repl_fragments<M: MemorySpace>(
    repl_pid: ProcessId,
    proc: &mut Process,
    mem: &mut M,
    scheduler: &'static Scheduler,
) {
    let fragments = scheduler.with_process_table_mut(|pt| pt.take_fragments(repl_pid));
    if let Some(head) = fragments {
        // Reverse the LIFO fragment list to restore FIFO send order
        let mut reversed = None;
        let mut current = Some(head);
        while let Some(mut frag) = current {
            current = frag.next.take();
            frag.next = reversed;
            reversed = Some(frag);
        }
        // Drain reversed list into mailbox
        let mut frag = reversed;
        while let Some(mut f) = frag {
            let next = f.next.take();
            let msg = f.message();
            if let Some(copied) =
                crate::process::deep_copy::deep_copy_message_to_process(msg, proc, mem)
            {
                proc.mailbox.push(copied);
            }
            frag = next;
        }
    }
}

/// Drain pending exit signals from the REPL's slot.
///
/// Since the REPL is always taken, exit signals from linked/monitored
/// processes are queued as pending signals. This delivers them to the
/// REPL process. For trap-exit processes, signals become messages;
/// otherwise non-normal signals would kill the REPL.
fn drain_repl_pending_signals<M: MemorySpace>(
    repl_pid: ProcessId,
    proc: &mut Process,
    realm: &'static SpinRwLock<Realm>,
    mem: &mut M,
    scheduler: &'static Scheduler,
) {
    let signals = scheduler.with_process_table_mut(|pt| pt.take_pending_signals(repl_pid));
    if signals.is_empty() {
        return;
    }
    // The REPL traps exits by default so it survives linked process crashes.
    // Deliver each signal as a [:EXIT sender reason] message.
    proc.trap_exit = true;
    for (sender, reason) in signals {
        let exit_kw = {
            let mut realm_guard = realm.write();
            realm_guard
                .intern_keyword(mem, "EXIT")
                .unwrap_or(crate::term::Term::NIL)
        };
        let pid_term = proc
            .alloc_term_pid(mem, sender.index() as u32, sender.generation())
            .unwrap_or(crate::term::Term::NIL);
        let copied_reason =
            crate::process::deep_copy::deep_copy_message_to_process(reason, proc, mem)
                .unwrap_or(reason);
        if let Some(msg) = proc.alloc_term_tuple(mem, &[exit_kw, pid_term, copied_reason]) {
            proc.mailbox.push(msg);
        }
    }
}

fn print_read_error<U: Uart>(e: &ReadError, uart: &mut U) {
    match e {
        ReadError::Lex(e) => {
            use crate::reader::LexError;
            match e {
                LexError::UnterminatedString => uart.write_str("unterminated string"),
                LexError::InvalidEscape(c) => {
                    uart.write_str("invalid escape: \\");
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    uart.write_str(s);
                }
                LexError::InvalidNumber => uart.write_str("invalid number"),
                LexError::TooLong => uart.write_str("string or symbol too long"),
                LexError::UnexpectedChar(c) => {
                    uart.write_str("unexpected character: ");
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    uart.write_str(s);
                }
            }
        }
        ReadError::Parse(e) => {
            use crate::reader::ParseError;
            match e {
                ParseError::UnexpectedEof => uart.write_str("unexpected end of input"),
                ParseError::UnexpectedToken(_) => uart.write_str("unexpected token"),
                ParseError::UnmatchedRParen => uart.write_str("unmatched )"),
                ParseError::UnmatchedRBracket => uart.write_str("unmatched ]"),
                ParseError::UnmatchedRBrace => uart.write_str("unmatched }"),
                ParseError::OutOfMemory => uart.write_str("out of memory"),
                ParseError::ListTooLong => uart.write_str("list too long"),
                ParseError::TupleTooLong => uart.write_str("tuple too long"),
                ParseError::VectorTooLong => uart.write_str("vector too long"),
                ParseError::MapTooLong => uart.write_str("map too long"),
                ParseError::MapOddElements => {
                    uart.write_str("map requires even number of elements");
                }
                ParseError::InvalidMetadata => uart.write_str("invalid metadata"),
                ParseError::MissingFormAfterMetadata => {
                    uart.write_str("expected form after metadata");
                }
            }
        }
    }
}

fn print_compile_error<U: Uart>(e: CompileError, uart: &mut U) {
    match e {
        CompileError::UnboundSymbol => uart.write_str("unbound symbol"),
        CompileError::InvalidSyntax => uart.write_str("invalid syntax"),
        CompileError::TooManyArguments => uart.write_str("too many arguments"),
        CompileError::IntegerTooLarge => uart.write_str("integer too large"),
        CompileError::ConstantPoolFull => uart.write_str("constant pool full"),
        CompileError::ExpressionTooComplex => uart.write_str("expression too complex"),
        CompileError::InternalError => uart.write_str("internal compiler error"),
    }
}

fn print_runtime_error<U: Uart>(e: &RuntimeError, uart: &mut U) {
    match e {
        RuntimeError::InvalidOpcode(op) => {
            uart.write_str("invalid opcode: ");
            print_u8(*op, uart);
        }
        RuntimeError::IpOutOfBounds => uart.write_str("instruction pointer out of bounds"),
        RuntimeError::ConstantOutOfBounds(idx) => {
            uart.write_str("constant index out of bounds: ");
            print_u32(*idx, uart);
        }
        RuntimeError::Intrinsic(e) => print_intrinsic_error(e, uart),
        RuntimeError::NoCode => uart.write_str("no code to execute"),
        RuntimeError::OutOfMemory => uart.write_str("out of memory"),
        RuntimeError::NotCallable { type_name } => {
            uart.write_str(type_name);
            uart.write_str(" is not callable");
        }
        RuntimeError::ArityMismatch {
            expected,
            got,
            variadic,
        } => {
            uart.write_str("wrong number of arguments: expected ");
            print_u8(*expected, uart);
            if *variadic {
                uart.write_str("+");
            }
            uart.write_str(", got ");
            print_u8(*got, uart);
        }
        RuntimeError::CallableArityError { expected, got } => {
            uart.write_str("wrong number of arguments: expected ");
            uart.write_str(expected);
            uart.write_str(", got ");
            print_u8(*got, uart);
        }
        RuntimeError::CallableTypeError {
            callable,
            arg,
            expected,
        } => {
            uart.write_str(callable);
            uart.write_str(" call: argument ");
            print_u8(*arg, uart);
            uart.write_str(" must be ");
            uart.write_str(expected);
        }
        RuntimeError::StackOverflow => uart.write_str("call stack overflow"),
        RuntimeError::YRegisterOutOfBounds { index, allocated } => {
            uart.write_str("Y register ");
            print_usize(*index, uart);
            uart.write_str(" out of bounds (");
            print_usize(*allocated, uart);
            uart.write_str(" allocated)");
        }
        RuntimeError::FrameMismatch {
            allocated,
            deallocate_count,
        } => {
            uart.write_str("frame mismatch: allocated ");
            print_usize(*allocated, uart);
            uart.write_str(" Y regs, tried to deallocate ");
            print_usize(*deallocate_count, uart);
        }
        RuntimeError::Badmatch { .. } => {
            uart.write_str("badmatch: no clause matched");
        }
        RuntimeError::EvalError => {
            uart.write_str("eval: compilation failed");
        }
        RuntimeError::ProcessLimitReached => {
            uart.write_str("process limit reached");
        }
        RuntimeError::BadArgument { intrinsic, message } => {
            uart.write_str("bad argument in ");
            uart.write_str(intrinsic);
            uart.write_str(": ");
            uart.write_str(message);
        }
    }
}

fn print_intrinsic_error<U: Uart>(e: &IntrinsicError, uart: &mut U) {
    match e {
        IntrinsicError::TypeError {
            intrinsic,
            arg,
            expected,
        } => {
            uart.write_str("type error in intrinsic ");
            if let Some(name) = crate::intrinsics::intrinsic_name(*intrinsic) {
                uart.write_str(name);
            } else {
                print_u8(*intrinsic, uart);
            }
            uart.write_str(": argument ");
            print_u8(*arg, uart);
            uart.write_str(" expected ");
            uart.write_str(expected);
        }
        IntrinsicError::DivisionByZero => uart.write_str("division by zero"),
        IntrinsicError::Overflow => uart.write_str("integer overflow"),
        IntrinsicError::UnknownIntrinsic(id) => {
            uart.write_str("unknown intrinsic: ");
            print_u8(*id, uart);
        }
        IntrinsicError::OutOfMemory => uart.write_str("out of memory"),
        IntrinsicError::IndexOutOfBounds { index, len } => {
            uart.write_str("index out of bounds: ");
            print_i64(*index, uart);
            uart.write_str(" >= ");
            print_usize(*len, uart);
        }
    }
}

/// Print a u8 as decimal.
fn print_u8<U: Uart>(n: u8, uart: &mut U) {
    let mut buf = [0u8; 3];
    let mut i = 0;
    let mut val = n;

    if val == 0 {
        uart.write_byte(b'0');
        return;
    }

    while val > 0 {
        buf[i] = b'0' + (val % 10);
        val /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        uart.write_byte(buf[i]);
    }
}

/// Print a u32 as decimal.
fn print_u32<U: Uart>(n: u32, uart: &mut U) {
    let mut buf = [0u8; 10];
    let mut i = 0;
    let mut val = n;

    if val == 0 {
        uart.write_byte(b'0');
        return;
    }

    while val > 0 {
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        uart.write_byte(buf[i]);
    }
}

/// Print an i64 as decimal.
fn print_i64<U: Uart>(n: i64, uart: &mut U) {
    if n < 0 {
        uart.write_byte(b'-');
        // Handle i64::MIN edge case
        if n == i64::MIN {
            uart.write_str("9223372036854775808");
            return;
        }
    }
    print_u64(n.unsigned_abs(), uart);
}

/// Print a u64 as decimal.
fn print_u64<U: Uart>(n: u64, uart: &mut U) {
    let mut buf = [0u8; 20];
    let mut i = 0;
    let mut val = n;

    if val == 0 {
        uart.write_byte(b'0');
        return;
    }

    while val > 0 {
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        uart.write_byte(buf[i]);
    }
}

/// Print a usize as decimal.
fn print_usize<U: Uart>(n: usize, uart: &mut U) {
    print_u64(n as u64, uart);
}

/// Run the REPL for a limited number of iterations (for testing).
#[cfg(test)]
pub fn run_limited<M: MemorySpace, U: Uart>(
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    uart: &mut U,
    max_iterations: usize,
) {
    let mut line_buf = [0u8; LINE_BUFFER_SIZE];

    let mut worker = Worker::new(WorkerId(0));

    for _ in 0..max_iterations {
        uart.write_str("lona> ");

        let len = uart.read_line(&mut line_buf);

        if len == 0 {
            continue;
        }

        let Ok(line) = core::str::from_utf8(&line_buf[..len]) else {
            uart.write_line("Error: invalid UTF-8");
            continue;
        };

        // Parse
        let expr = match read(line, proc, realm, mem) {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => {
                uart.write_str("Error: ");
                print_read_error(&e, uart);
                uart.write_byte(b'\n');
                continue;
            }
        };

        // Compile
        let chunk = match compiler::compile(expr, proc, mem, realm) {
            Ok(c) => c,
            Err(e) => {
                uart.write_str("Error: ");
                print_compile_error(e, uart);
                uart.write_byte(b'\n');
                continue;
            }
        };

        // Serialize chunk to heap (try GC on failure)
        if !proc.write_chunk_to_heap(mem, &chunk) {
            let _ = crate::gc::minor_gc(proc, &mut worker, mem);
            if !proc.write_chunk_to_heap(mem, &chunk) {
                let _ = crate::gc::major_gc(proc, &mut worker, realm.pool_mut(), mem);
                if !proc.write_chunk_to_heap(mem, &chunk) {
                    uart.write_line("Error: out of memory");
                    worker.reset_x_regs();
                    proc.reset();
                    continue;
                }
            }
        }
        let result = match vm::execute(&mut worker, proc, mem, realm) {
            Ok(v) => v,
            Err(e) => {
                uart.write_str("Error: ");
                print_runtime_error(&e, uart);
                uart.write_byte(b'\n');
                worker.reset_x_regs();
                proc.reset();
                continue;
            }
        };

        // Print result
        print_term(result, proc, realm, mem, uart);
        uart.write_byte(b'\n');

        // Reset for next expression
        worker.reset_x_regs();
        proc.reset();
    }
}
