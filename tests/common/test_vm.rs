// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Test VM for integration testing.
//!
//! Provides a stateful VM that uses the same code paths as the UART REPL,
//! enabling end-to-end testing of language features.

#![expect(
    dead_code,
    reason = "test infrastructure used via macros in test files"
)]

use lona_vm::platform::MockVSpace;
use lona_vm::reader::{ReadError, read};
use lona_vm::uart::MockUart;
use lona_vm::value::print_value;
use lona_vm::{Heap, Vaddr, Value};

/// Default heap size for test VMs (64 KB).
const DEFAULT_HEAP_SIZE: usize = 64 * 1024;

/// Default heap base address.
const DEFAULT_HEAP_BASE: Vaddr = Vaddr::new(0x0100_0000);

/// A test VM with its own heap and memory space.
///
/// Each test should create its own `TestVm` to ensure isolation.
/// The VM is stateful, allowing multiple operations to build on each other.
///
/// # Example
///
/// ```ignore
/// let mut vm = TestVm::new();
/// let value = vm.read_and_eval("42")?;
/// assert_eq!(vm.print(value), "42");
/// ```
pub struct TestVm {
    heap: Heap,
    mem: MockVSpace,
}

impl Default for TestVm {
    fn default() -> Self {
        Self::new()
    }
}

impl TestVm {
    /// Create a new test VM with default heap size (64 KB).
    #[must_use]
    pub fn new() -> Self {
        Self::with_heap_size(DEFAULT_HEAP_SIZE)
    }

    /// Create a new test VM with a specific heap size.
    ///
    /// # Arguments
    ///
    /// * `size` - Heap size in bytes
    #[must_use]
    pub fn with_heap_size(size: usize) -> Self {
        let base = DEFAULT_HEAP_BASE;
        Self {
            heap: Heap::new(base, size),
            mem: MockVSpace::new(size, base.sub(size as u64)),
        }
    }

    /// Read a string into a value.
    ///
    /// Uses the same `read` function as the UART REPL.
    ///
    /// # Errors
    ///
    /// Returns `ReadError` if parsing fails.
    pub fn read(&mut self, input: &str) -> Result<Option<Value>, ReadError> {
        read(input, &mut self.heap, &mut self.mem)
    }

    /// Read a string and return the value.
    ///
    /// # Errors
    ///
    /// Returns `TestVmError` if reading fails or input is empty.
    pub fn read_and_eval(&mut self, input: &str) -> Result<Value, TestVmError> {
        self.read(input)?.ok_or(TestVmError::EmptyInput)
    }

    /// Print a value to a string.
    ///
    /// Uses the same `print_value` function as the UART REPL.
    #[must_use]
    pub fn print(&self, value: Value) -> String {
        let mut uart = MockUart::new();
        print_value(value, &self.heap, &self.mem, &mut uart);
        String::from_utf8_lossy(uart.output()).into_owned()
    }

    /// Read, eval, and print in one step.
    ///
    /// Convenience method for quick round-trip tests.
    ///
    /// # Errors
    ///
    /// Returns `TestVmError` if reading fails.
    pub fn rep(&mut self, input: &str) -> Result<String, TestVmError> {
        let result = self.read_and_eval(input)?;
        Ok(self.print(result))
    }

    /// Get remaining heap space in bytes.
    #[must_use]
    pub const fn heap_remaining(&self) -> usize {
        self.heap.remaining()
    }

    /// Get used heap space in bytes.
    #[must_use]
    pub const fn heap_used(&self) -> usize {
        self.heap.used()
    }

    /// Get a reference to the heap (for matchers that need direct access).
    #[must_use]
    pub const fn heap(&self) -> &Heap {
        &self.heap
    }

    /// Get a reference to the memory space (for matchers that need direct access).
    #[must_use]
    pub const fn mem(&self) -> &MockVSpace {
        &self.mem
    }
}

/// Error type for test VM operations.
#[derive(Debug)]
pub enum TestVmError {
    /// Error during reading/parsing.
    Read(ReadError),
    /// Input was empty or whitespace-only.
    EmptyInput,
}

impl core::fmt::Display for TestVmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Read(e) => write!(f, "read error: {e:?}"),
            Self::EmptyInput => write!(f, "empty input"),
        }
    }
}

impl std::error::Error for TestVmError {}

impl From<ReadError> for TestVmError {
    fn from(e: ReadError) -> Self {
        Self::Read(e)
    }
}
