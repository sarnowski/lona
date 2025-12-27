// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Interactive REPL console for production use.
//!
//! This module handles the interactive portion of the REPL:
//! - Byte-by-byte input with backspace, Ctrl+C handling
//! - Value formatting and output to UART
//! - The main loop waiting for user input
//!
//! This code is not tested via integration tests because:
//! 1. Byte-by-byte input simulation is impractical
//! 2. The core `eval()` function is what matters for correctness
//! 3. These are straightforward I/O wrappers around the tested core

use crate::repl::Repl;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;
use lona_core::source;
use lona_core::value::Value;
use lona_kernel::vm::MacroExpander;
use lonala_compiler::{CompileError, compile_with_expansion};
use lonala_human::{Config as FormatConfig, render as render_error};
use lonala_parser::error::Kind as ParseErrorKind;

/// Control character for backspace (DEL key).
const BACKSPACE: u8 = 0x7F;

/// Control character for backspace (Ctrl+H).
const CTRL_H: u8 = 0x08;

/// Control character for carriage return (Enter key on terminals).
const CARRIAGE_RETURN: u8 = 0x0D;

/// Control character for line feed (newline in piped/non-TTY input).
const LINE_FEED: u8 = 0x0A;

/// Control character for Ctrl+C (cancel input).
const CTRL_C: u8 = 0x03;

/// Maximum line length in bytes.
const MAX_LINE_LENGTH: usize = 1024;

/// Buffer for accumulating a single line of input.
///
/// Handles UTF-8 input by accepting raw bytes and properly handling
/// backspace for multi-byte characters.
struct LineBuffer {
    buffer: Vec<u8>,
}

impl LineBuffer {
    const fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    fn push(&mut self, byte: u8) -> bool {
        if self.buffer.len() < MAX_LINE_LENGTH {
            self.buffer.push(byte);
            true
        } else {
            false
        }
    }

    /// Removes the last UTF-8 character from the buffer.
    ///
    /// UTF-8 continuation bytes start with `10xxxxxx` (0x80-0xBF).
    /// We keep popping until we remove a non-continuation byte (the start byte).
    fn backspace(&mut self) -> bool {
        if self.buffer.is_empty() {
            return false;
        }

        // Pop bytes until we've removed a complete UTF-8 character
        // UTF-8 continuation bytes have the pattern 10xxxxxx (0x80-0xBF)
        loop {
            let Some(byte) = self.buffer.pop() else {
                return true; // Buffer became empty
            };

            // If this is NOT a continuation byte, we've removed the start of the character
            // ASCII bytes (0x00-0x7F) and UTF-8 lead bytes (0xC0-0xFF) are not continuation bytes
            if !is_utf8_continuation_byte(byte) {
                return true;
            }

            // If buffer is now empty, we're done
            if self.buffer.is_empty() {
                return true;
            }
        }
    }

    /// Returns the buffer content as a string, replacing invalid UTF-8 with the replacement character.
    fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.buffer).into_owned()
    }
}

/// Returns true if the byte is a UTF-8 continuation byte (10xxxxxx pattern).
const fn is_utf8_continuation_byte(byte: u8) -> bool {
    // Continuation bytes have the pattern 10xxxxxx
    // This means the top two bits are 10, i.e., byte & 0xC0 == 0x80
    (byte & 0xC0) == 0x80
}

/// Result of reading a line from the console.
enum LineResult {
    Line(String),
    Cancelled,
}

/// I/O trait for the interactive console.
pub trait ConsoleIo {
    /// Reads a single byte from input, blocking until available.
    fn read_byte(&mut self) -> Option<u8>;

    /// Writes a string to output.
    fn write_str(&mut self, text: &str);

    /// Writes a single byte to output.
    ///
    /// This is used for echoing raw bytes, including UTF-8 lead and continuation bytes.
    /// The terminal accumulates bytes and renders complete UTF-8 characters.
    fn write_byte(&mut self, byte: u8);

    /// Writes a formatted string to output.
    fn write_fmt(&mut self, args: core::fmt::Arguments<'_>) {
        struct FmtAdapter<'io, T: ConsoleIo + ?Sized>(&'io mut T);
        impl<T: ConsoleIo + ?Sized> Write for FmtAdapter<'_, T> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                self.0.write_str(s);
                Ok(())
            }
        }
        // Ignore formatting errors since we can't do anything about them.
        if FmtAdapter(self).write_fmt(args).is_err() {
            // Nothing we can do about formatting errors in a no_std environment
        }
    }
}

/// UART-based console I/O.
pub struct UartConsole;

impl ConsoleIo for UartConsole {
    fn read_byte(&mut self) -> Option<u8> {
        crate::platform::arch::read_byte()
    }

    fn write_str(&mut self, text: &str) {
        crate::print!("{text}");
    }

    fn write_byte(&mut self, byte: u8) {
        crate::platform::arch::write_byte(byte);
    }
}

/// Interactive REPL console.
///
/// Wraps the core [`Repl`] with I/O handling for interactive use.
pub struct InteractiveRepl<I: ConsoleIo> {
    repl: Repl,
    io: I,
    accumulated: String,
}

impl<I: ConsoleIo> InteractiveRepl<I> {
    /// Creates a new interactive REPL with the given I/O backend.
    pub const fn new(io: I) -> Self {
        Self {
            repl: Repl::new(),
            io,
            accumulated: String::new(),
        }
    }

    /// Runs the REPL loop forever.
    pub fn run(&mut self) -> ! {
        self.io
            .write_fmt(format_args!("Lona REPL {}\n", env!("LONA_VERSION")));
        self.writeln("Type expressions to evaluate. Ctrl+C to cancel input.");

        // Load the core standard library
        if let Err(err) = self.repl.load_core() {
            self.io.write_fmt(format_args!("Warning: {err}\n"));
        }

        self.writeln("");

        loop {
            self.print_prompt();

            match self.read_line() {
                LineResult::Line(line) => {
                    self.process_line(&line);
                }
                LineResult::Cancelled => {
                    if !self.accumulated.is_empty() {
                        self.accumulated.clear();
                        self.writeln("");
                    }
                }
            }
        }
    }

    fn write(&mut self, text: &str) {
        self.io.write_str(text);
    }

    fn writeln(&mut self, text: &str) {
        self.io.write_str(text);
        self.io.write_str("\n");
    }

    fn print_prompt(&mut self) {
        if self.accumulated.is_empty() {
            self.write("lona> ");
        } else {
            self.write("...> ");
        }
    }

    fn read_line(&mut self) -> LineResult {
        let mut buffer = LineBuffer::new();

        loop {
            let Some(input_byte) = self.io.read_byte() else {
                continue;
            };

            match input_byte {
                // Accept both carriage return (TTY) and line feed (piped input)
                CARRIAGE_RETURN | LINE_FEED => {
                    self.writeln("");
                    return LineResult::Line(buffer.to_string_lossy());
                }
                CTRL_C => {
                    self.write("^C");
                    self.writeln("");
                    return LineResult::Cancelled;
                }
                BACKSPACE | CTRL_H => {
                    if buffer.backspace() {
                        // For multi-byte UTF-8, we need to erase the visual character
                        // This sends: move cursor back, write space, move cursor back
                        self.write("\x08 \x08");
                    }
                }
                // Accept all printable bytes (both ASCII and UTF-8)
                // Control characters (0x00-0x1F except handled above, and 0x7F) are ignored
                byte if is_printable_byte(byte) => {
                    if buffer.push(byte) {
                        // Echo the raw byte to the terminal.
                        // For ASCII bytes (0x20-0x7E), this displays the character.
                        // For UTF-8 multi-byte sequences (lead and continuation bytes >= 0x80),
                        // the terminal accumulates bytes and renders complete characters.
                        // We write using a single-byte slice to preserve the exact byte value.
                        self.io.write_byte(byte);
                    }
                }
                // Ignore control characters
                _ => {}
            }
        }
    }

    fn process_line(&mut self, line: &str) {
        if !self.accumulated.is_empty() {
            self.accumulated.push('\n');
        }
        self.accumulated.push_str(line);

        if self.accumulated.trim().is_empty() {
            self.accumulated.clear();
            return;
        }

        // Use a temporary source ID (0) for the completeness check.
        // The actual source will be registered when eval() is called.
        let temp_source_id = source::Id::new(0_u32);

        // First, try to compile to check if input is complete
        let mut expander = MacroExpander::new();
        let (interner, macros) = self.repl.interner_and_macros();
        match compile_with_expansion(
            &self.accumulated,
            temp_source_id,
            interner,
            macros,
            &mut expander,
        ) {
            Ok(_chunk) => {
                // Input is complete - evaluate using the core eval() function
                // This ensures the same code path is used as in tests
                let source_text = core::mem::take(&mut self.accumulated);
                self.execute_and_print(&source_text);
            }
            Err(CompileError::Parse(ref parse_err)) => {
                if needs_more_input(&parse_err.kind) {
                    return;
                }
                // Register source for error formatting
                let source_text = core::mem::take(&mut self.accumulated);
                self.display_parse_error(&source_text, parse_err);
            }
            Err(CompileError::Compile(ref compile_err)) => {
                // Register source for error formatting
                let source_text = core::mem::take(&mut self.accumulated);
                self.display_compile_error(&source_text, compile_err);
            }
            Err(ref err) => {
                // Catch-all for future CompileError variants (non_exhaustive)
                self.accumulated.clear();
                self.io.write_fmt(format_args!("Error: {err:?}\n"));
            }
        }
    }

    /// Evaluates source and prints the result.
    ///
    /// Uses the core `Repl::eval()` function to ensure the same code path
    /// is exercised in both interactive and test modes.
    fn execute_and_print(&mut self, source: &str) {
        match self.repl.eval(source) {
            Ok(result) => {
                if !matches!(result, Value::Nil) {
                    self.print_value(&result);
                }
            }
            Err(err) => {
                self.io.write_fmt(format_args!("{err}\n"));
            }
        }
    }

    fn print_value(&mut self, value: &Value) {
        let displayable = value.display(self.repl.interner());
        self.io.write_fmt(format_args!("{displayable}\n"));
    }

    /// Formats and displays a parse error using `lonala_human`.
    fn display_parse_error(&mut self, source_text: &str, error: &lonala_parser::Error) {
        // Register source for error formatting
        let source_name = format!("<repl:{}>", self.repl.input_counter().saturating_add(1_u32));
        let mut temp_registry = source::Registry::new();
        if let Some(source_id) = temp_registry.add(source_name, String::from(source_text)) {
            // Update the error's source ID to match our temporary registry
            let error_with_correct_source = lonala_parser::Error::new(
                error.kind.clone(),
                source::Location::new(source_id, error.location.span),
            );
            let config = FormatConfig::new();
            let formatted = render_error(
                &error_with_correct_source,
                &temp_registry,
                self.repl.interner(),
                &config,
            );
            self.io.write_str(&formatted);
        } else {
            // Fallback if registry is full
            self.io
                .write_fmt(format_args!("Error: {}\n", error.kind.variant_name()));
        }
    }

    /// Formats and displays a compile error using `lonala_human`.
    fn display_compile_error(&mut self, source_text: &str, error: &lonala_compiler::error::Error) {
        // Register source for error formatting
        let source_name = format!("<repl:{}>", self.repl.input_counter().saturating_add(1_u32));
        let mut temp_registry = source::Registry::new();
        if let Some(source_id) = temp_registry.add(source_name, String::from(source_text)) {
            // Update the error's source ID to match our temporary registry
            let error_with_correct_source = lonala_compiler::error::Error::new(
                error.kind.clone(),
                source::Location::new(source_id, error.location.span),
            );
            let config = FormatConfig::new();
            let formatted = render_error(
                &error_with_correct_source,
                &temp_registry,
                self.repl.interner(),
                &config,
            );
            self.io.write_str(&formatted);
        } else {
            // Fallback if registry is full
            self.io
                .write_fmt(format_args!("Error: {}\n", error.kind.variant_name()));
        }
    }
}

/// Returns true if the byte is a printable character or part of a UTF-8 sequence.
///
/// Accepts:
/// - ASCII printable characters (0x20-0x7E)
/// - All UTF-8 bytes (0x80-0xFF) - lead bytes and continuation bytes
///
/// Rejects:
/// - ASCII control characters (0x00-0x1F and 0x7F)
const fn is_printable_byte(byte: u8) -> bool {
    // ASCII printable range (space through tilde)
    // OR any byte >= 0x80 (UTF-8 multi-byte sequences)
    (byte >= 0x20 && byte < 0x7F) || byte >= 0x80
}

const fn needs_more_input(kind: &ParseErrorKind) -> bool {
    matches!(
        *kind,
        ParseErrorKind::UnexpectedEof { .. } | ParseErrorKind::UnterminatedString
    )
}
