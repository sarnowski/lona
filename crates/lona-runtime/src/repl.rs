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
    intern_metadata_primitives, intern_symbol_primitives, intern_type_predicates,
    register_arithmetic_primitives, register_comparison_primitives, register_metadata_primitives,
    register_symbol_primitives, register_type_predicates,
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

    /// Returns a reference to the symbol interner.
    #[cfg(not(feature = "integration-test"))]
    pub const fn interner(&self) -> &Interner {
        &self.interner
    }

    /// Returns both the interner and macros for simultaneous access.
    ///
    /// This is needed when calling functions that require both an immutable
    /// reference to the interner and a mutable reference to macros.
    #[cfg(not(feature = "integration-test"))]
    pub const fn interner_and_macros(&mut self) -> (&Interner, &mut MacroRegistry) {
        (&self.interner, &mut self.macros)
    }

    /// Returns the current input counter value.
    #[cfg(not(feature = "integration-test"))]
    pub const fn input_counter(&self) -> u32 {
        self.input_counter
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
            &self.interner,
            &mut self.macros,
            &mut expander,
        )
        .map_err(|err| format!("Core library error: {err}"))?;

        // Pre-intern primitive symbols before creating VM
        let collection_symbols = intern_collection_primitives(&self.interner);
        let introspection_symbols = intern_introspection_primitives(&self.interner);
        let arithmetic_symbols = intern_arithmetic_primitives(&self.interner);
        let comparison_symbols = intern_comparison_primitives(&self.interner);
        let type_predicate_symbols = intern_type_predicates(&self.interner);
        let metadata_symbols = intern_metadata_primitives(&self.interner);
        let symbol_symbols = intern_symbol_primitives(&self.interner);

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

        // Register symbol primitives (symbol, gensym)
        register_symbol_primitives(&mut vm, &symbol_symbols);

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
            &self.interner,
            &mut self.macros,
            &mut expander,
        )
        .map_err(|err| format!("Compile error: {err}"))?;

        // Pre-intern primitive symbols before creating VM
        let collection_symbols = intern_collection_primitives(&self.interner);
        let introspection_symbols = intern_introspection_primitives(&self.interner);
        let arithmetic_symbols = intern_arithmetic_primitives(&self.interner);
        let comparison_symbols = intern_comparison_primitives(&self.interner);
        let type_predicate_symbols = intern_type_predicates(&self.interner);
        let metadata_symbols = intern_metadata_primitives(&self.interner);
        let symbol_symbols = intern_symbol_primitives(&self.interner);

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

        // Register symbol primitives (symbol, gensym)
        register_symbol_primitives(&mut vm, &symbol_symbols);

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

// Re-export interactive types for use in main.rs
#[cfg(not(feature = "integration-test"))]
pub use crate::interactive::{InteractiveRepl, UartConsole};
