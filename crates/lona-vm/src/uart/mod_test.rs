// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the UART interface.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{MockUart, Uart, UartExt};

#[test]
fn mock_uart_write_byte() {
    let mut uart = MockUart::new();
    uart.write_byte(b'H');
    uart.write_byte(b'i');
    assert_eq!(uart.output(), b"Hi");
}

#[test]
fn mock_uart_read_byte() {
    let mut uart = MockUart::with_input(b"AB");
    assert_eq!(uart.read_byte(), b'A');
    assert_eq!(uart.read_byte(), b'B');
}

#[test]
fn mock_uart_can_read() {
    let mut uart = MockUart::with_input(b"X");
    assert!(uart.can_read());
    uart.read_byte();
    assert!(!uart.can_read());
}

#[test]
fn mock_uart_can_write() {
    let uart = MockUart::new();
    assert!(uart.can_write());
}

#[test]
fn uart_ext_write_str() {
    let mut uart = MockUart::new();
    uart.write_str("Hello");
    assert_eq!(uart.output(), b"Hello");
}

#[test]
fn uart_ext_write_line() {
    let mut uart = MockUart::new();
    uart.write_line("Hello");
    assert_eq!(uart.output(), b"Hello\n");
}

#[test]
fn uart_ext_read_line_simple() {
    let mut uart = MockUart::with_input(b"hello\r");
    let mut buf = [0u8; 64];
    let len = uart.read_line(&mut buf);
    assert_eq!(len, 5);
    assert_eq!(&buf[..len], b"hello");
    // Should echo input plus CR LF
    assert_eq!(uart.output(), b"hello\r\n");
}

#[test]
fn uart_ext_read_line_with_backspace() {
    // Type "helo", backspace, "lo"
    let mut uart = MockUart::with_input(b"helo\x7Flo\r");
    let mut buf = [0u8; 64];
    let len = uart.read_line(&mut buf);
    assert_eq!(len, 5);
    assert_eq!(&buf[..len], b"hello");
}

#[test]
fn uart_ext_read_line_buffer_full() {
    let mut uart = MockUart::with_input(b"abcdefgh\r");
    let mut buf = [0u8; 4];
    let len = uart.read_line(&mut buf);
    assert_eq!(len, 4);
    assert_eq!(&buf[..len], b"abcd");
}

#[test]
fn uart_ext_read_line_empty() {
    let mut uart = MockUart::with_input(b"\r");
    let mut buf = [0u8; 64];
    let len = uart.read_line(&mut buf);
    assert_eq!(len, 0);
}
