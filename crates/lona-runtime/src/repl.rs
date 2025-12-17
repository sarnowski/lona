// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Read-Eval-Print Loop (REPL) for interactive Lonala development.
//!
//! This module provides:
//! - [`Repl`]: The core evaluation engine with persistent state
//! - Interactive console support (production only)
//!
//! # Architecture
//!
//! The REPL is split into tested and untested components:
//!
//! **Tested Core** (via integration tests):
//! - [`Repl::eval`]: Compiles and executes a source string, returns the result
//! - State management: Symbol interning and global variables persist across calls
//!
//! **Untested Glue** (interactive mode only):
//! - Byte-by-byte input handling (backspace, Ctrl+C, etc.)
//! - Value-to-string output formatting
//! - The main loop waiting for user input

use alloc::string::String;
#[cfg(not(feature = "integration-test"))]
use alloc::string::ToString;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lona_kernel::vm::collections::{
    intern_primitives as intern_collection_primitives,
    register_primitives as register_collection_primitives,
};
use lona_kernel::vm::introspection::{
    intern_primitives as intern_introspection_primitives,
    register_primitives as register_introspection_primitives,
};
use lona_kernel::vm::{Globals, MacroExpander, Vm};
use lonala_compiler::{MacroRegistry, compile_with_expansion};

// =============================================================================
// Core REPL (Always Available - Used by Both Tests and Production)
// =============================================================================

/// REPL evaluation engine with persistent state.
///
/// Maintains state across evaluations including:
/// - Symbol interner for sharing symbols between compiler and VM
/// - Global variables defined with `def`
/// - Macro definitions that persist across sessions
///
/// # Macro Support
///
/// Macros can be defined with `defmacro` and are stored in a persistent registry.
/// When macro calls are encountered during compilation, they are expanded at
/// compile time using the VM-based `MacroExpander`. The macro transformer
/// function is executed to produce the expanded code, which is then compiled
/// in place of the macro call.
///
/// # Example
///
/// ```ignore
/// let mut repl = Repl::new();
/// let result = repl.eval("(def x 42)").unwrap();
/// // x is now defined
/// let result = repl.eval("x").unwrap();
/// // result is Integer(42)
/// ```
pub struct Repl {
    /// Symbol interner shared between compiler and VM.
    interner: Interner,
    /// Global variables that persist between evaluations.
    globals: Globals,
    /// Macro definitions that persist between evaluations.
    macros: MacroRegistry,
}

impl Repl {
    /// Creates a new REPL instance with empty state.
    ///
    /// The REPL starts with an empty symbol interner, no global variables,
    /// and no macro definitions. State defined via `def` and `defmacro`
    /// during evaluation will persist across subsequent calls to [`eval`].
    pub const fn new() -> Self {
        Self {
            interner: Interner::new(),
            globals: Globals::new(),
            macros: MacroRegistry::new(),
        }
    }

    /// Evaluates a source string and returns the result.
    ///
    /// This is the core REPL function used by both integration tests and the
    /// interactive console. It compiles and executes the source, maintaining
    /// persistent state for globals defined with `def` and macros defined
    /// with `defmacro`.
    ///
    /// # Macro Expansion
    ///
    /// Macros are expanded at compile time using the VM-based macro expander.
    /// When a macro is called, its transformer function is executed by a
    /// temporary VM instance to produce the expanded code.
    ///
    /// # Errors
    ///
    /// Returns an error string if compilation or execution fails.
    pub fn eval(&mut self, source: &str) -> Result<Value, String> {
        // Create a macro expander
        let mut expander = MacroExpander::new();

        // Compile the source with macro expansion
        let chunk =
            compile_with_expansion(source, &mut self.interner, &mut self.macros, &mut expander)
                .map_err(|err| alloc::format!("Compile error: {err}"))?;

        // Pre-intern primitive symbols before creating VM
        let collection_symbols = intern_collection_primitives(&mut self.interner);
        let introspection_symbols = intern_introspection_primitives(&mut self.interner);

        // Create a VM for this evaluation
        let mut vm = Vm::new(&self.interner);

        // Restore persistent globals into the VM
        *vm.globals_mut() = self.globals.clone();

        // Register the print function
        if let Some(print_sym) = self.interner.get("print") {
            vm.update_print_symbol(print_sym);
            vm.set_global(print_sym, Value::Symbol(print_sym));
        }

        // Register collection primitives with pre-interned symbols
        register_collection_primitives(&mut vm, &collection_symbols);

        // Set up macro introspection functions
        vm.set_macro_registry(&self.macros);
        register_introspection_primitives(&mut vm, &introspection_symbols);

        // Execute
        let result = vm
            .execute(&chunk)
            .map_err(|err| alloc::format!("Runtime error: {err:?}"))?;

        // Save globals back to persistent storage
        self.globals = vm.globals().clone();

        Ok(result)
    }
}

// =============================================================================
// Interactive Console (Production Only - Untested Glue Code)
// =============================================================================
//
// The following code handles the interactive REPL experience:
// - Byte-by-byte input with backspace, Ctrl+C handling
// - Value formatting and output to UART
// - The main loop waiting for user input
//
// This code is not tested via integration tests because:
// 1. Byte-by-byte input simulation is impractical
// 2. The core eval() function is what matters for correctness
// 3. These are straightforward I/O wrappers around the tested core

#[cfg(not(feature = "integration-test"))]
mod interactive {
    use super::{MacroExpander, Repl, String, ToString, Value, compile_with_expansion};
    use alloc::vec::Vec;
    use core::fmt::Write;
    use lonala_compiler::CompileError;
    use lonala_parser::error::Kind as ParseErrorKind;

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

        fn backspace(&mut self) -> bool {
            self.buffer.pop().is_some()
        }

        fn as_str(&self) -> Option<&str> {
            core::str::from_utf8(&self.buffer).ok()
        }

        fn to_string_lossy(&self) -> String {
            self.as_str().map_or_else(String::new, ToString::to_string)
        }
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
            crate::platform::uart::read_byte()
        }

        fn write_str(&mut self, text: &str) {
            crate::print!("{text}");
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
            self.writeln("Lona REPL v0.1.0");
            self.writeln("Type expressions to evaluate. Ctrl+C to cancel input.");
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
                    ENTER => {
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
                            self.write("\x08 \x08");
                        }
                    }
                    ch_byte if is_printable_ascii(ch_byte) => {
                        if buffer.push(ch_byte) {
                            let ch = char::from(ch_byte);
                            self.io.write_fmt(format_args!("{ch}"));
                        }
                    }
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

            // First, try to compile to check if input is complete
            let mut expander = MacroExpander::new();
            match compile_with_expansion(
                &self.accumulated,
                &mut self.repl.interner,
                &mut self.repl.macros,
                &mut expander,
            ) {
                Ok(_chunk) => {
                    // Input is complete - evaluate using the core eval() function
                    // This ensures the same code path is used as in tests
                    let source = core::mem::take(&mut self.accumulated);
                    self.execute_and_print(&source);
                }
                Err(CompileError::Parse(ref parse_err)) => {
                    if needs_more_input(&parse_err.kind) {
                        return;
                    }
                    let source = core::mem::take(&mut self.accumulated);
                    self.display_error(&source, parse_err.span.start, &parse_err.kind.to_string());
                }
                Err(CompileError::Compile(ref compile_err)) => {
                    let source = core::mem::take(&mut self.accumulated);
                    self.display_error(&source, compile_err.span().start, &compile_err.to_string());
                }
                Err(ref err) => {
                    self.accumulated.clear();
                    self.io.write_fmt(format_args!("Error: {err}\n"));
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
            let displayable = value.display(&self.repl.interner);
            self.io.write_fmt(format_args!("{displayable}\n"));
        }

        fn display_error(&mut self, source: &str, position: usize, message: &str) {
            self.io.write_fmt(format_args!("Error: {message}\n"));

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

            let line_end = source
                .get(line_start..)
                .and_then(|rest| rest.find('\n'))
                .map_or(source.len(), |offset| line_start.saturating_add(offset));

            if let Some(line) = source.get(line_start..line_end) {
                self.io.write_fmt(format_args!("  {line_num} | {line}\n"));

                let col = position.saturating_sub(line_start);
                let prefix_width = format_line_prefix_width(line_num);
                self.write("  ");
                for _ in 0..prefix_width {
                    self.write(" ");
                }
                self.write(" | ");
                for _ in 0..col {
                    self.write(" ");
                }
                self.writeln("^");
            }
        }
    }

    const fn is_printable_ascii(byte: u8) -> bool {
        byte >= 0x20 && byte < 0x7F
    }

    const fn needs_more_input(kind: &ParseErrorKind) -> bool {
        matches!(
            *kind,
            ParseErrorKind::UnexpectedEof { .. } | ParseErrorKind::UnterminatedString
        )
    }

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
}

// Re-export interactive types for use in main.rs
#[cfg(not(feature = "integration-test"))]
pub use interactive::{InteractiveRepl, UartConsole};
