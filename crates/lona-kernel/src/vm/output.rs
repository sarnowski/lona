// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Output abstraction for the Lonala virtual machine.
//!
//! Provides a trait-based output system that enables testable `print`
//! implementations. In tests, output can be captured to a buffer;
//! in the runtime, it writes to UART or debug output.

use core::fmt::{self, Write};

use lona_core::symbol::Interner;
use lona_core::value::Value;

/// Trait for output operations.
///
/// Enables testable print implementations by abstracting over
/// the actual output destination.
pub trait Output {
    /// Writes a string to the output.
    fn write_str(&mut self, text: &str);

    /// Writes a formatted value to the output.
    ///
    /// Uses the interner to resolve symbol names.
    #[inline]
    fn write_value(&mut self, value: Value, interner: &Interner) {
        let displayable = value.display(interner);
        // Use Display trait formatting
        let _err = write!(OutputWriter { output: self }, "{displayable}");
    }
}

/// Adapter to use `Output` trait with `core::fmt::Write`.
struct OutputWriter<'output, O: Output + ?Sized> {
    output: &'output mut O,
}

impl<O: Output + ?Sized> Write for OutputWriter<'_, O> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.output.write_str(s);
        Ok(())
    }
}

/// Output that discards all writes.
///
/// Useful for benchmarks or silent mode where output is not needed.
#[non_exhaustive]
pub struct Discard;

impl Output for Discard {
    #[inline]
    fn write_str(&mut self, _text: &str) {
        // Discard
    }

    #[inline]
    fn write_value(&mut self, _value: Value, _interner: &Interner) {
        // Discard
    }
}

/// Output buffer for capturing output in tests.
///
/// Stores all written content in a string buffer that can be
/// inspected after execution.
#[cfg(test)]
pub struct BufferOutput {
    /// The accumulated output.
    pub buffer: alloc::string::String,
}

#[cfg(test)]
impl BufferOutput {
    /// Creates a new empty buffer output.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: alloc::string::String::new(),
        }
    }

    /// Clears the buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
impl Default for BufferOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Output for BufferOutput {
    fn write_str(&mut self, s: &str) {
        self.buffer.push_str(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::integer::Integer;
    use lona_core::symbol::Interner;

    #[test]
    fn discard_output_discards() {
        let mut output = Discard;
        output.write_str("hello");
        // No assertion - just verifying it doesn't panic
    }

    #[test]
    fn buffer_output_captures_string() {
        let mut output = BufferOutput::new();
        output.write_str("hello");
        output.write_str(" world");
        assert_eq!(output.buffer, "hello world");
    }

    #[test]
    fn buffer_output_clear() {
        let mut output = BufferOutput::new();
        output.write_str("some text");
        output.clear();
        assert!(output.buffer.is_empty());
    }

    #[test]
    fn buffer_output_write_value_integer() {
        let interner = Interner::new();
        let mut output = BufferOutput::new();
        output.write_value(Value::Integer(Integer::from_i64(42)), &interner);
        assert_eq!(output.buffer, "42");
    }

    #[test]
    fn buffer_output_write_value_float() {
        let interner = Interner::new();
        let mut output = BufferOutput::new();
        output.write_value(Value::Float(3.14), &interner);
        assert_eq!(output.buffer, "3.14");
    }

    #[test]
    fn buffer_output_write_value_bool() {
        let interner = Interner::new();
        let mut output = BufferOutput::new();

        output.write_value(Value::Bool(true), &interner);
        assert_eq!(output.buffer, "true");

        output.clear();
        output.write_value(Value::Bool(false), &interner);
        assert_eq!(output.buffer, "false");
    }

    #[test]
    fn buffer_output_write_value_nil() {
        let interner = Interner::new();
        let mut output = BufferOutput::new();
        output.write_value(Value::Nil, &interner);
        assert_eq!(output.buffer, "nil");
    }

    #[test]
    fn buffer_output_write_value_symbol() {
        let mut interner = Interner::new();
        let sym_id = interner.intern("my-symbol");

        let mut output = BufferOutput::new();
        output.write_value(Value::Symbol(sym_id), &interner);
        assert_eq!(output.buffer, "my-symbol");
    }

    #[test]
    fn buffer_output_write_value_float_whole() {
        let interner = Interner::new();
        let mut output = BufferOutput::new();
        output.write_value(Value::Float(1.0), &interner);
        // Whole floats show decimal point
        assert_eq!(output.buffer, "1.0");
    }
}
