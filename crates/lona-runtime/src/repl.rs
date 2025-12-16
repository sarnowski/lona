// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Read-Eval-Print Loop (REPL) for interactive Lonala development.
//!
//! Provides an interactive console for evaluating Lonala expressions.
//! Supports multi-line input with continuation prompts when parentheses
//! are unbalanced.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_kernel::vm::Vm;
use lonala_compiler::{CompileError, compile};
use lonala_parser::error::Kind as ParseErrorKind;

use crate::platform::uart;
use crate::{print, println};

/// Control character for backspace (DEL key).
const BACKSPACE: u8 = 0x7F;

/// Control character for backspace (Ctrl+H).
const CTRL_H: u8 = 0x08;

/// Control character for carriage return (Enter key).
const ENTER: u8 = 0x0D;

/// Control character for Ctrl+C (cancel input).
const CTRL_C: u8 = 0x03;

/// Maximum line length in bytes.
const MAX_LINE_LENGTH: usize = 1024;

/// Buffer for accumulating a single line of input.
struct LineBuffer {
    /// Character buffer.
    buffer: Vec<u8>,
}

impl LineBuffer {
    /// Creates a new empty line buffer.
    const fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Attempts to push a character to the buffer.
    ///
    /// Returns `true` if the character was added, `false` if buffer is full.
    fn push(&mut self, byte: u8) -> bool {
        if self.buffer.len() < MAX_LINE_LENGTH {
            self.buffer.push(byte);
            true
        } else {
            false
        }
    }

    /// Removes the last character from the buffer.
    ///
    /// Returns `true` if a character was removed, `false` if buffer was empty.
    fn backspace(&mut self) -> bool {
        self.buffer.pop().is_some()
    }

    /// Clears the buffer.
    #[cfg(test)]
    fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Returns `true` if the buffer is empty.
    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns the buffer contents as a string slice, or `None` if not valid UTF-8.
    fn as_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.buffer).ok()
    }

    /// Converts the buffer to a String, returning empty string if invalid UTF-8.
    fn to_string_lossy(&self) -> String {
        self.as_str().map_or_else(String::new, ToString::to_string)
    }
}

/// Result of reading a line from the console.
enum LineResult {
    /// A complete line was read.
    Line(String),
    /// Input was cancelled (Ctrl+C).
    Cancelled,
}

/// Interactive REPL for Lonala evaluation.
///
/// Maintains persistent state across evaluations including the symbol
/// interner. Note: In Phase 3.1, globals do not persist between evaluations
/// since we don't have `def` yet (that's Phase 3.3).
pub struct Repl {
    /// Symbol interner shared between compiler and VM.
    interner: Interner,
    /// Accumulated source for multi-line input.
    accumulated: String,
}

impl Repl {
    /// Creates a new REPL instance with fresh state.
    pub const fn new() -> Self {
        Self {
            interner: Interner::new(),
            accumulated: String::new(),
        }
    }

    /// Runs the REPL loop forever.
    ///
    /// This function never returns - it continuously reads and evaluates
    /// expressions until the system is shut down.
    pub fn run(&mut self) -> ! {
        println!("Lona REPL v0.1.0");
        println!("Type expressions to evaluate. Ctrl+C to cancel input.");
        println!();

        loop {
            self.print_prompt();

            match Self::read_line() {
                LineResult::Line(line) => {
                    self.process_line(&line);
                }
                LineResult::Cancelled => {
                    // Reset accumulated input on Ctrl+C
                    if !self.accumulated.is_empty() {
                        self.accumulated.clear();
                        println!();
                    }
                }
            }
        }
    }

    /// Prints the appropriate prompt based on input state.
    fn print_prompt(&self) {
        if self.accumulated.is_empty() {
            print!("lona> ");
        } else {
            print!("...> ");
        }
    }

    /// Reads a single line from the console.
    ///
    /// Handles backspace and Ctrl+C.
    fn read_line() -> LineResult {
        let mut buffer = LineBuffer::new();

        loop {
            let Some(input_byte) = uart::read_byte() else {
                // UART not initialized, shouldn't happen in normal operation
                continue;
            };

            match input_byte {
                ENTER => {
                    println!(); // Echo newline
                    let line = buffer.to_string_lossy();
                    return LineResult::Line(line);
                }
                CTRL_C => {
                    print!("^C");
                    println!();
                    return LineResult::Cancelled;
                }
                BACKSPACE | CTRL_H => {
                    if buffer.backspace() {
                        // Echo backspace: move cursor back, overwrite with space, move back again
                        print!("\x08 \x08");
                    }
                }
                // Printable ASCII characters
                ch_byte if is_printable_ascii(ch_byte) => {
                    if buffer.push(ch_byte) {
                        // Echo the character - char::from is safe for any u8
                        let ch = char::from(ch_byte);
                        print!("{ch}");
                    }
                    // Silently ignore if buffer is full
                }
                // Ignore other control characters
                _ => {}
            }
        }
    }

    /// Processes a line of input, accumulating for multi-line or evaluating.
    fn process_line(&mut self, line: &str) {
        // Add line to accumulated input
        if !self.accumulated.is_empty() {
            self.accumulated.push('\n');
        }
        self.accumulated.push_str(line);

        // Skip empty input
        if self.accumulated.trim().is_empty() {
            self.accumulated.clear();
            return;
        }

        // Try to compile the accumulated input
        match compile(&self.accumulated, &mut self.interner) {
            Ok(chunk) => {
                // Successfully compiled - execute it
                self.accumulated.clear();
                self.execute_chunk(&chunk);
            }
            Err(CompileError::Parse(ref parse_err)) => {
                // Check if this is an "incomplete input" error
                if needs_more_input(&parse_err.kind) {
                    // Keep accumulating
                    return;
                }
                // Real parse error - display it and reset
                let source = core::mem::take(&mut self.accumulated);
                Self::display_error(&source, parse_err.span.start, &parse_err.kind.to_string());
            }
            Err(CompileError::Compile(ref compile_err)) => {
                // Compile error - display it and reset
                let source = core::mem::take(&mut self.accumulated);
                Self::display_error(&source, compile_err.span().start, &compile_err.to_string());
            }
            // Handle future error variants (CompileError is non-exhaustive)
            Err(ref err) => {
                self.accumulated.clear();
                println!("Error: {err}");
            }
        }
    }

    /// Executes a compiled chunk and displays the result.
    fn execute_chunk(&self, chunk: &lonala_compiler::Chunk) {
        // Create a fresh VM for this evaluation
        let mut vm = Vm::new(&self.interner);
        vm.set_print_callback(uart_print);

        // Make sure the print symbol is registered
        if let Some(print_sym) = self.interner.get("print") {
            vm.update_print_symbol(print_sym);
            vm.set_global(print_sym, Value::Symbol(print_sym));
        }

        match vm.execute(chunk) {
            Ok(result) => {
                // Print the result unless it's nil
                if !matches!(result, Value::Nil) {
                    self.print_value(&result);
                }
            }
            Err(err) => {
                println!("Runtime error: {err:?}");
            }
        }
    }

    /// Prints a value to the console.
    fn print_value(&self, value: &Value) {
        match *value {
            Value::Nil => println!("nil"),
            Value::Integer(int_val) => println!("{int_val}"),
            Value::Float(float_val) => println!("{float_val}"),
            Value::Bool(bool_val) => println!("{bool_val}"),
            Value::Symbol(sym_id) => {
                let name = self.interner.resolve(sym_id);
                println!("{name}");
            }
            Value::String(ref string) => {
                print!("\"");
                for ch in string.as_str().chars() {
                    match ch {
                        '"' => print!("\\\""),
                        '\\' => print!("\\\\"),
                        '\n' => print!("\\n"),
                        '\t' => print!("\\t"),
                        '\r' => print!("\\r"),
                        other => print!("{other}"),
                    }
                }
                println!("\"");
            }
            // Handle future value variants (Value is non-exhaustive)
            _ => println!("{value:?}"),
        }
    }

    /// Displays an error with source context and caret pointing to location.
    fn display_error(source: &str, position: usize, message: &str) {
        println!("Error: {message}");

        // Find the line containing the error position
        let mut line_start = 0_usize;
        let mut line_num = 1_usize;

        for (idx, ch) in source.char_indices() {
            if idx >= position {
                break;
            }
            if ch == '\n' {
                line_start = idx.saturating_add(1);
                line_num = line_num.saturating_add(1);
            }
        }

        // Find end of line
        let line_end = source
            .get(line_start..)
            .and_then(|rest| rest.find('\n'))
            .map_or(source.len(), |offset| line_start.saturating_add(offset));

        // Extract the line
        if let Some(line) = source.get(line_start..line_end) {
            println!("  {line_num} | {line}");

            // Calculate column position
            let col = position.saturating_sub(line_start);

            // Print caret at error position
            // Account for line number prefix width
            let prefix_width = format_line_prefix_width(line_num);
            print!("  ");
            for _ in 0..prefix_width {
                print!(" ");
            }
            print!(" | ");
            for _ in 0..col {
                print!(" ");
            }
            println!("^");
        }
    }
}

/// Checks if a byte is a printable ASCII character (0x20-0x7E).
const fn is_printable_ascii(byte: u8) -> bool {
    byte >= 0x20 && byte < 0x7F
}

/// Checks if a parse error indicates incomplete input (need more lines).
const fn needs_more_input(kind: &ParseErrorKind) -> bool {
    matches!(
        *kind,
        ParseErrorKind::UnexpectedEof { .. } | ParseErrorKind::UnterminatedString
    )
}

/// Calculates the width of the line number prefix for alignment.
const fn format_line_prefix_width(line_num: usize) -> usize {
    if line_num == 0 {
        1
    } else {
        let mut n = line_num;
        let mut width = 0_usize;
        while n > 0 {
            width = width.saturating_add(1);
            n /= 10;
        }
        width
    }
}

/// Print callback for the VM that outputs to UART.
fn uart_print(output: &str) {
    print!("{output}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_buffer_push_and_backspace() {
        let mut buf = LineBuffer::new();
        assert!(buf.is_empty());

        assert!(buf.push(b'a'));
        assert!(!buf.is_empty());
        assert_eq!(buf.as_str(), Some("a"));

        assert!(buf.push(b'b'));
        assert_eq!(buf.as_str(), Some("ab"));

        assert!(buf.backspace());
        assert_eq!(buf.as_str(), Some("a"));

        assert!(buf.backspace());
        assert!(buf.is_empty());

        // Backspace on empty buffer
        assert!(!buf.backspace());
    }

    #[test]
    fn line_buffer_clear() {
        let mut buf = LineBuffer::new();
        buf.push(b'x');
        buf.push(b'y');
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn line_buffer_to_string_lossy() {
        let mut buf = LineBuffer::new();
        buf.push(b'h');
        buf.push(b'i');
        assert_eq!(buf.to_string_lossy(), "hi");
    }

    #[test]
    fn format_line_prefix_width_single_digit() {
        assert_eq!(format_line_prefix_width(1), 1);
        assert_eq!(format_line_prefix_width(9), 1);
    }

    #[test]
    fn format_line_prefix_width_multi_digit() {
        assert_eq!(format_line_prefix_width(10), 2);
        assert_eq!(format_line_prefix_width(99), 2);
        assert_eq!(format_line_prefix_width(100), 3);
    }

    #[test]
    fn format_line_prefix_width_zero() {
        assert_eq!(format_line_prefix_width(0), 1);
    }

    #[test]
    fn is_printable_ascii_works() {
        assert!(is_printable_ascii(b' '));
        assert!(is_printable_ascii(b'~'));
        assert!(is_printable_ascii(b'a'));
        assert!(!is_printable_ascii(0x1F));
        assert!(!is_printable_ascii(0x7F));
    }
}
