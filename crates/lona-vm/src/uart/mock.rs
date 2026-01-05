// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Mock UART for testing.
//!
//! This provides a UART implementation backed by in-memory buffers,
//! allowing unit tests to verify UART interactions without hardware.

use super::Uart;
use std::collections::VecDeque;
use std::vec::Vec;

/// Mock UART backed by in-memory buffers.
pub struct MockUart {
    /// Input buffer (data to be read)
    input: VecDeque<u8>,
    /// Output buffer (data that was written)
    output: Vec<u8>,
}

impl MockUart {
    /// Create an empty mock UART.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            input: VecDeque::new(),
            output: Vec::new(),
        }
    }

    /// Create a mock UART with pre-loaded input data.
    #[must_use]
    pub fn with_input(input: &[u8]) -> Self {
        Self {
            input: input.iter().copied().collect(),
            output: Vec::new(),
        }
    }

    /// Get the output that has been written.
    #[must_use]
    pub fn output(&self) -> &[u8] {
        &self.output
    }

    /// Clear the output buffer.
    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    /// Add more input data.
    pub fn push_input(&mut self, data: &[u8]) {
        self.input.extend(data);
    }
}

impl Default for MockUart {
    fn default() -> Self {
        Self::new()
    }
}

impl Uart for MockUart {
    fn write_byte(&mut self, byte: u8) {
        self.output.push(byte);
    }

    #[expect(
        clippy::expect_used,
        reason = "test mock panics intentionally on empty input"
    )]
    fn read_byte(&mut self) -> u8 {
        // In tests, if input is empty we panic rather than block forever
        self.input
            .pop_front()
            .expect("MockUart: no input available")
    }

    fn can_read(&self) -> bool {
        !self.input.is_empty()
    }

    fn can_write(&self) -> bool {
        true // Mock UART is always ready to write
    }
}
