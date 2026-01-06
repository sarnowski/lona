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
use crate::process::Process;
use crate::reader::{ReadError, read};
use crate::uart::{Uart, UartExt};
use crate::value::print_value;
use crate::vm::{self, RuntimeError};

/// Maximum line buffer size.
const LINE_BUFFER_SIZE: usize = 256;

/// Run the REPL loop.
///
/// This function never returns under normal operation.
pub fn run<M: MemorySpace, U: Uart>(proc: &mut Process, mem: &mut M, uart: &mut U) -> ! {
    let mut line_buf = [0u8; LINE_BUFFER_SIZE];

    loop {
        // Print prompt
        uart.write_str("lona> ");

        // Read line
        let len = uart.read_line(&mut line_buf);

        // Skip empty lines
        if len == 0 {
            continue;
        }

        // Convert to str
        let Ok(line) = core::str::from_utf8(&line_buf[..len]) else {
            uart.write_line("Error: invalid UTF-8");
            continue;
        };

        // Parse
        let expr = match read(line, proc, mem) {
            Ok(Some(v)) => v,
            Ok(None) => continue, // Empty input
            Err(e) => {
                uart.write_str("Error: ");
                print_read_error(&e, uart);
                uart.write_byte(b'\n');
                continue;
            }
        };

        // Compile
        let chunk = match compiler::compile(expr, proc, mem) {
            Ok(c) => c,
            Err(e) => {
                uart.write_str("Error: ");
                print_compile_error(e, uart);
                uart.write_byte(b'\n');
                continue;
            }
        };

        // Set chunk and execute
        proc.set_chunk(chunk);
        let result = match vm::execute(proc, mem) {
            Ok(v) => v,
            Err(e) => {
                uart.write_str("Error: ");
                print_runtime_error(&e, uart);
                uart.write_byte(b'\n');
                proc.reset();
                continue;
            }
        };

        // Print result
        print_value(result, proc, mem, uart);
        uart.write_byte(b'\n');

        // Reset for next expression
        proc.reset();
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
                ParseError::OutOfMemory => uart.write_str("out of memory"),
                ParseError::ListTooLong => uart.write_str("list too long"),
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

/// Run the REPL for a limited number of iterations (for testing).
#[cfg(test)]
pub fn run_limited<M: MemorySpace, U: Uart>(
    proc: &mut Process,
    mem: &mut M,
    uart: &mut U,
    max_iterations: usize,
) {
    let mut line_buf = [0u8; LINE_BUFFER_SIZE];

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
        let expr = match read(line, proc, mem) {
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
        let chunk = match compiler::compile(expr, proc, mem) {
            Ok(c) => c,
            Err(e) => {
                uart.write_str("Error: ");
                print_compile_error(e, uart);
                uart.write_byte(b'\n');
                continue;
            }
        };

        // Set chunk and execute
        proc.set_chunk(chunk);
        let result = match vm::execute(proc, mem) {
            Ok(v) => v,
            Err(e) => {
                uart.write_str("Error: ");
                print_runtime_error(&e, uart);
                uart.write_byte(b'\n');
                proc.reset();
                continue;
            }
        };

        // Print result
        print_value(result, proc, mem, uart);
        uart.write_byte(b'\n');

        // Reset for next expression
        proc.reset();
    }
}
