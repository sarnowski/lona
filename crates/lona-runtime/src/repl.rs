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

use alloc::format;
use alloc::string::String;
#[cfg(not(feature = "integration-test"))]
use alloc::string::ToString;
use lona_core::source;
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
use lona_kernel::vm::{
    Globals, MacroExpander, Vm, intern_arithmetic_primitives, intern_comparison_primitives,
    intern_metadata_primitives, intern_type_predicates, register_arithmetic_primitives,
    register_comparison_primitives, register_metadata_primitives, register_type_predicates,
};
use lonala_compiler::{MacroRegistry, compile_with_expansion};
use lonala_human::{Config as FormatConfig, render as render_error};

// =============================================================================
// Core REPL (Always Available - Used by Both Tests and Production)
// =============================================================================

/// REPL evaluation engine with persistent state.
///
/// Maintains state across evaluations including:
/// - Symbol interner for sharing symbols between compiler and VM
/// - Global variables defined with `def`
/// - Macro definitions that persist across sessions
/// - Source registry for error reporting with context
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
    /// Source registry for error reporting with context.
    sources: source::Registry,
    /// Counter for naming REPL inputs (incremented per evaluation).
    input_counter: u32,
}

/// The core standard library source, embedded at compile time.
#[cfg(not(feature = "integration-test"))]
const CORE_LIBRARY: &str = include_str!("../../../lona/core.lona");

impl Repl {
    /// Creates a new REPL instance with empty state.
    ///
    /// The REPL starts with an empty symbol interner, no global variables,
    /// no macro definitions, and an empty source registry. State defined
    /// via `def` and `defmacro` during evaluation will persist across
    /// subsequent calls to [`eval`].
    pub const fn new() -> Self {
        Self {
            interner: Interner::new(),
            globals: Globals::new(),
            macros: MacroRegistry::new(),
            sources: source::Registry::new(),
            input_counter: 0_u32,
        }
    }

    /// Loads the core standard library (`lona/core.lona`).
    ///
    /// This defines essential macros like `defn` and `when` that form the
    /// foundation of Lonala programming. Call this before interactive use
    /// or when testing code that depends on core macros.
    ///
    /// # Errors
    ///
    /// Returns an error string if the core library fails to load (which
    /// indicates a bug in the core library).
    #[cfg(not(feature = "integration-test"))]
    pub fn load_core(&mut self) -> Result<(), String> {
        // Skip if core.lona is empty (shouldn't happen in practice)
        if CORE_LIBRARY.trim().is_empty() {
            return Ok(());
        }

        // Register the core library source
        let source_id = self
            .sources
            .add(String::from("<core>"), String::from(CORE_LIBRARY))
            .ok_or_else(|| String::from("source registry full"))?;

        // Create a macro expander
        let mut expander = MacroExpander::new();

        // Compile the core library with macro expansion
        let chunk = compile_with_expansion(
            CORE_LIBRARY,
            source_id,
            &mut self.interner,
            &mut self.macros,
            &mut expander,
        )
        .map_err(|err| format!("Core library error: {err}"))?;

        // Pre-intern primitive symbols before creating VM
        let collection_symbols = intern_collection_primitives(&mut self.interner);
        let introspection_symbols = intern_introspection_primitives(&mut self.interner);
        let arithmetic_symbols = intern_arithmetic_primitives(&mut self.interner);
        let comparison_symbols = intern_comparison_primitives(&mut self.interner);
        let type_predicate_symbols = intern_type_predicates(&mut self.interner);
        let metadata_symbols = intern_metadata_primitives(&mut self.interner);

        // Create a VM for core library initialization
        let mut vm = Vm::new(&self.interner);
        vm.set_source(source_id);

        // Restore persistent globals into the VM
        *vm.globals_mut() = self.globals.clone();

        // Register collection primitives with pre-interned symbols
        register_collection_primitives(&mut vm, &collection_symbols);

        // Register arithmetic primitives (first-class +, -, *, /, mod)
        register_arithmetic_primitives(&mut vm, &arithmetic_symbols);

        // Register comparison primitives (first-class =, <, >, <=, >=)
        register_comparison_primitives(&mut vm, &comparison_symbols);

        // Register type predicate primitives (keyword?, etc.)
        register_type_predicates(&mut vm, &type_predicate_symbols);

        // Register metadata primitives (meta, with-meta)
        register_metadata_primitives(&mut vm, &metadata_symbols);

        // Set up macro introspection functions
        vm.set_macro_registry(&self.macros);
        register_introspection_primitives(&mut vm, &introspection_symbols);

        // Execute the core library
        vm.execute(&chunk)
            .map_err(|err| format!("Core library execution error: {err:?}"))?;

        // Save globals back to persistent storage
        self.globals = vm.globals().clone();

        Ok(())
    }

    /// Evaluates a source string and returns the result.
    ///
    /// This is the core REPL function used by both integration tests and the
    /// interactive console. It compiles and executes the source, maintaining
    /// persistent state for globals defined with `def` and macros defined
    /// with `defmacro`.
    ///
    /// Each evaluation registers its source in the source registry with a
    /// unique name like `<repl:1>`, `<repl:2>`, etc. This enables error
    /// messages to include source context.
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
    pub fn eval(&mut self, source_text: &str) -> Result<Value, String> {
        // Increment input counter and register the source
        self.input_counter = self.input_counter.saturating_add(1_u32);
        let source_name = format!("<repl:{}>", self.input_counter);
        let source_id = self
            .sources
            .add(source_name, String::from(source_text))
            .ok_or_else(|| String::from("source registry full"))?;

        // Create a macro expander
        let mut expander = MacroExpander::new();

        // Compile the source with macro expansion
        let chunk = compile_with_expansion(
            source_text,
            source_id,
            &mut self.interner,
            &mut self.macros,
            &mut expander,
        )
        .map_err(|err| format!("Compile error: {err}"))?;

        // Pre-intern primitive symbols before creating VM
        let collection_symbols = intern_collection_primitives(&mut self.interner);
        let introspection_symbols = intern_introspection_primitives(&mut self.interner);
        let arithmetic_symbols = intern_arithmetic_primitives(&mut self.interner);
        let comparison_symbols = intern_comparison_primitives(&mut self.interner);
        let type_predicate_symbols = intern_type_predicates(&mut self.interner);
        let metadata_symbols = intern_metadata_primitives(&mut self.interner);

        // Create a VM for this evaluation
        let mut vm = Vm::new(&self.interner);

        // Set the source ID for error reporting
        vm.set_source(source_id);

        // Restore persistent globals into the VM
        *vm.globals_mut() = self.globals.clone();

        // Register collection primitives with pre-interned symbols
        register_collection_primitives(&mut vm, &collection_symbols);

        // Register arithmetic primitives (first-class +, -, *, /, mod)
        register_arithmetic_primitives(&mut vm, &arithmetic_symbols);

        // Register comparison primitives (first-class =, <, >, <=, >=)
        register_comparison_primitives(&mut vm, &comparison_symbols);

        // Register type predicate primitives (keyword?, etc.)
        register_type_predicates(&mut vm, &type_predicate_symbols);

        // Register metadata primitives (meta, with-meta)
        register_metadata_primitives(&mut vm, &metadata_symbols);

        // Set up macro introspection functions
        vm.set_macro_registry(&self.macros);
        register_introspection_primitives(&mut vm, &introspection_symbols);

        // Execute
        let result = vm.execute(&chunk).map_err(|err| {
            // Format the VM error using lonala-human for Rust-style error messages
            let config = FormatConfig::new();
            render_error(&err, &self.sources, &self.interner, &config)
        })?;

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
    use super::{MacroExpander, Repl, String, ToString, Value, compile_with_expansion, format};
    use alloc::vec::Vec;
    use core::fmt::Write;
    use lona_core::source;
    use lonala_compiler::CompileError;
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
            crate::platform::arch::read_byte()
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

            // Use a temporary source ID (0) for the completeness check.
            // The actual source will be registered when eval() is called.
            let temp_source_id = source::Id::new(0_u32);

            // First, try to compile to check if input is complete
            let mut expander = MacroExpander::new();
            match compile_with_expansion(
                &self.accumulated,
                temp_source_id,
                &mut self.repl.interner,
                &mut self.repl.macros,
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

        /// Formats and displays a parse error using `lonala_human`.
        fn display_parse_error(&mut self, source_text: &str, error: &lonala_parser::Error) {
            // Register source for error formatting
            let source_name = format!("<repl:{}>", self.repl.input_counter.saturating_add(1_u32));
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
                    &self.repl.interner,
                    &config,
                );
                self.io.write_str(&formatted);
            } else {
                // Fallback if registry is full
                self.io.write_fmt(format_args!("Error: {}\n", error.kind));
            }
        }

        /// Formats and displays a compile error using `lonala_human`.
        fn display_compile_error(
            &mut self,
            source_text: &str,
            error: &lonala_compiler::error::Error,
        ) {
            // Register source for error formatting
            let source_name = format!("<repl:{}>", self.repl.input_counter.saturating_add(1_u32));
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
                    &self.repl.interner,
                    &config,
                );
                self.io.write_str(&formatted);
            } else {
                // Fallback if registry is full
                self.io.write_fmt(format_args!("Error: {}\n", error.kind));
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
}

// Re-export interactive types for use in main.rs
#[cfg(not(feature = "integration-test"))]
pub use interactive::{InteractiveRepl, UartConsole};
