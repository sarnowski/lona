// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the REPL.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::run_limited;
use crate::Vaddr;
use crate::heap::Heap;
use crate::platform::MockVSpace;
use crate::uart::MockUart;

fn setup() -> (MockVSpace, Heap) {
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x10000));
    let heap = Heap::new(Vaddr::new(0x20000), 0x10000);
    (mem, heap)
}

#[test]
fn repl_empty_line() {
    let (mut mem, mut heap) = setup();
    // Empty line: just CR
    let mut uart = MockUart::with_input(b"\r");

    run_limited(&mut heap, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    // Should just show prompt and echo the newline
    assert!(output.contains("lona> "));
}

#[test]
fn repl_nil() {
    let (mut mem, mut heap) = setup();
    let mut uart = MockUart::with_input(b"nil\r");

    run_limited(&mut heap, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("lona> "));
    assert!(output.contains("nil\n"));
}

#[test]
fn repl_integer() {
    let (mut mem, mut heap) = setup();
    let mut uart = MockUart::with_input(b"42\r");

    run_limited(&mut heap, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("42\n"));
}

#[test]
fn repl_list() {
    let (mut mem, mut heap) = setup();
    let mut uart = MockUart::with_input(b"(1 2 3)\r");

    run_limited(&mut heap, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("(1 2 3)\n"));
}

#[test]
fn repl_quote() {
    let (mut mem, mut heap) = setup();
    let mut uart = MockUart::with_input(b"'(1 2)\r");

    run_limited(&mut heap, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    // 'x => (quote x)
    assert!(output.contains("(quote (1 2))\n"));
}

#[test]
fn repl_error() {
    let (mut mem, mut heap) = setup();
    let mut uart = MockUart::with_input(b")\r");

    run_limited(&mut heap, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("Error: unmatched )"));
}

#[test]
fn repl_multiple_lines() {
    let (mut mem, mut heap) = setup();
    let mut uart = MockUart::with_input(b"1\r2\r3\r");

    run_limited(&mut heap, &mut mem, &mut uart, 3);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("1\n"));
    assert!(output.contains("2\n"));
    assert!(output.contains("3\n"));
}

#[test]
fn repl_string() {
    let (mut mem, mut heap) = setup();
    let mut uart = MockUart::with_input(b"\"hello\"\r");

    run_limited(&mut heap, &mut mem, &mut uart, 1);

    let output = std::string::String::from_utf8(uart.output().to_vec()).unwrap();
    assert!(output.contains("\"hello\"\n"));
}
