// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the REPL.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::run_limited;
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::uart::MockUart;

fn setup() -> (Process, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(128 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let proc = Process::new(1, young_base, young_size, old_base, old_size);
    (proc, mem)
}

#[test]
fn repl_empty_line() {
    let (mut proc, mut mem) = setup();
    // Empty line: just CR
    let mut uart = MockUart::with_input(b"\r");

    run_limited(&mut proc, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    // Should just show prompt and echo the newline
    assert!(output.contains("lona> "));
}

#[test]
fn repl_nil() {
    let (mut proc, mut mem) = setup();
    let mut uart = MockUart::with_input(b"nil\r");

    run_limited(&mut proc, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("lona> "));
    assert!(output.contains("nil\n"));
}

#[test]
fn repl_integer() {
    let (mut proc, mut mem) = setup();
    let mut uart = MockUart::with_input(b"42\r");

    run_limited(&mut proc, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("42\n"));
}

#[test]
fn repl_intrinsic_call() {
    let (mut proc, mut mem) = setup();
    // Test intrinsic call: (+ 1 2) → 3
    let mut uart = MockUart::with_input(b"(+ 1 2)\r");

    run_limited(&mut proc, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("3\n"));
}

#[test]
fn repl_not_callable() {
    let (mut proc, mut mem) = setup();
    // (1 2 3) compiles but fails at runtime - 1 is not callable
    let mut uart = MockUart::with_input(b"(1 2 3)\r");

    run_limited(&mut proc, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("Error: value is not callable"));
}

#[test]
fn repl_quote() {
    let (mut proc, mut mem) = setup();
    // 'x => (quote x) returns x unevaluated
    let mut uart = MockUart::with_input(b"'(1 2)\r");

    run_limited(&mut proc, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("(1 2)\n"));
}

#[test]
fn repl_error() {
    let (mut proc, mut mem) = setup();
    let mut uart = MockUart::with_input(b")\r");

    run_limited(&mut proc, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("Error: unmatched )"));
}

#[test]
fn repl_multiple_lines() {
    let (mut proc, mut mem) = setup();
    let mut uart = MockUart::with_input(b"1\r2\r3\r");

    run_limited(&mut proc, &mut mem, &mut uart, 3);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("1\n"));
    assert!(output.contains("2\n"));
    assert!(output.contains("3\n"));
}

#[test]
fn repl_string() {
    let (mut proc, mut mem) = setup();
    let mut uart = MockUart::with_input(b"\"hello\"\r");

    run_limited(&mut proc, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("\"hello\"\n"));
}
