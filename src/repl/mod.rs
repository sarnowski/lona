// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! REPL (Read-Eval-Print Loop) for Lonala.
//!
//! This module provides a minimal REPL that reads Lonala expressions,
//! and prints them back (no evaluation yet).

#![allow(
    clippy::manual_let_else,    // match is clearer in loop context
    clippy::single_match_else   // match is clearer in loop context
)]

#[cfg(test)]
mod mod_test;

use crate::heap::Heap;
use crate::platform::MemorySpace;
use crate::reader::{ReadError, read};
use crate::uart::{Uart, UartExt};
use crate::value::print_value;

/// Maximum line buffer size.
const LINE_BUFFER_SIZE: usize = 256;

/// Run the REPL loop.
///
/// This function never returns under normal operation.
pub fn run<M: MemorySpace, U: Uart>(heap: &mut Heap, mem: &mut M, uart: &mut U) -> ! {
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
        let line = match core::str::from_utf8(&line_buf[..len]) {
            Ok(s) => s,
            Err(_) => {
                uart.write_line("Error: invalid UTF-8");
                continue;
            }
        };

        // Parse and print
        match read(line, heap, mem) {
            Ok(Some(value)) => {
                print_value(value, heap, mem, uart);
                uart.write_byte(b'\n');
            }
            Ok(None) => {
                // Empty input after whitespace stripping
            }
            Err(e) => {
                uart.write_str("Error: ");
                print_error(&e, uart);
                uart.write_byte(b'\n');
            }
        }
    }
}

fn print_error<U: Uart>(e: &ReadError, uart: &mut U) {
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

/// Run the REPL for a limited number of iterations (for testing).
#[cfg(test)]
pub fn run_limited<M: MemorySpace, U: Uart>(
    heap: &mut Heap,
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

        let line = match core::str::from_utf8(&line_buf[..len]) {
            Ok(s) => s,
            Err(_) => {
                uart.write_line("Error: invalid UTF-8");
                continue;
            }
        };

        match read(line, heap, mem) {
            Ok(Some(value)) => {
                print_value(value, heap, mem, uart);
                uart.write_byte(b'\n');
            }
            Ok(None) => {}
            Err(e) => {
                uart.write_str("Error: ");
                print_error(&e, uart);
                uart.write_byte(b'\n');
            }
        }
    }
}
