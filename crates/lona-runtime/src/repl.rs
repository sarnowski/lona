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
use lona_kernel::namespace::Registry as NamespaceRegistry;
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
    intern_metadata_primitives, intern_namespace_primitives, intern_symbol_primitives,
    intern_type_predicates, intern_var_primitives, register_arithmetic_primitives,
    register_comparison_primitives, register_metadata_primitives, register_namespace_primitives,
    register_symbol_primitives, register_type_predicates, register_var_primitives,
};
use lonala_compiler::{MacroRegistry, compile_with_expansion_in_ns};
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
    /// Namespace registry that persists between evaluations.
    namespace_registry: Option<NamespaceRegistry>,
    /// Current namespace for compilation (tracks `(ns ...)` changes).
    current_namespace: Option<lona_core::symbol::Id>,
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
            namespace_registry: None,
            current_namespace: None,
        }
    }

    /// Returns a reference to the symbol interner.
    #[cfg(not(feature = "integration-test"))]
    pub const fn interner(&self) -> &Interner {
        &self.interner
    }

    /// Formats a `CompileError` for display using `lonala-human`.
    fn format_compile_error(
        &self,
        err: &lonala_compiler::CompileError,
        source_id: source::Id,
    ) -> String {
        use lonala_compiler::CompileError;
        let config = FormatConfig::new();
        match *err {
            CompileError::Parse(ref parse_err) => {
                let err_with_source = lonala_parser::Error::new(
                    parse_err.kind.clone(),
                    source::Location::new(source_id, parse_err.location.span),
                );
                render_error(&err_with_source, &self.sources, &self.interner, &config)
            }
            CompileError::Compile(ref compile_err) => {
                let err_with_source = lonala_compiler::error::Error::new(
                    compile_err.kind.clone(),
                    source::Location::new(source_id, compile_err.location.span),
                );
                render_error(&err_with_source, &self.sources, &self.interner, &config)
            }
            // Future variants (non_exhaustive) - use Debug
            _ => format!("{err:?}"),
        }
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

        // Default to "user" namespace for core library initialization
        let user_ns = self.interner.intern("user");

        // Compile the core library with macro expansion
        let chunk = match compile_with_expansion_in_ns(
            CORE_LIBRARY,
            source_id,
            &self.interner,
            &mut self.macros,
            &mut expander,
            user_ns,
        ) {
            Ok(chunk) => chunk,
            Err(err) => return Err(self.format_compile_error(&err, source_id)),
        };

        // Pre-intern primitive symbols before creating VM
        let collection_symbols = intern_collection_primitives(&self.interner);
        let introspection_symbols = intern_introspection_primitives(&self.interner);
        let arithmetic_symbols = intern_arithmetic_primitives(&self.interner);
        let comparison_symbols = intern_comparison_primitives(&self.interner);
        let type_predicate_symbols = intern_type_predicates(&self.interner);
        let metadata_symbols = intern_metadata_primitives(&self.interner);
        let symbol_symbols = intern_symbol_primitives(&self.interner);
        let var_symbols = intern_var_primitives(&self.interner);
        let namespace_symbols = intern_namespace_primitives(&self.interner);

        // Create a VM for core library initialization
        let mut vm = Vm::new(&self.interner);
        vm.set_source(source_id);

        // Restore persistent globals into the VM
        *vm.globals_mut() = self.globals.clone();

        // Restore namespace registry if we have one
        if let Some(ref registry) = self.namespace_registry {
            *vm.namespace_registry_mut() = registry.clone();
        }

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

        // Register var primitives (var-get, var-set!)
        register_var_primitives(&mut vm, &var_symbols);

        // Register namespace primitives (require, namespace-add-alias, etc.)
        register_namespace_primitives(&mut vm, &namespace_symbols);

        // Set up macro introspection functions
        vm.set_macro_registry(&self.macros);
        register_introspection_primitives(&mut vm, &introspection_symbols);

        // Propagate lona.core vars to all namespaces (auto-refer)
        vm.namespace_registry_mut().refer_core_to_all();

        // Execute the core library
        vm.execute(&chunk)
            .map_err(|err| format!("Core library execution error: {err:?}"))?;

        // Save globals and namespace registry back to persistent storage
        self.globals = vm.globals().clone();
        self.namespace_registry = Some(vm.namespace_registry().clone());

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

        // Get the current namespace for compilation (default to "user")
        let current_ns = self
            .current_namespace
            .unwrap_or_else(|| self.interner.intern("user"));

        // Compile the source with macro expansion in the current namespace
        let chunk = match compile_with_expansion_in_ns(
            source_text,
            source_id,
            &self.interner,
            &mut self.macros,
            &mut expander,
            current_ns,
        ) {
            Ok(chunk) => chunk,
            Err(err) => return Err(self.format_compile_error(&err, source_id)),
        };

        // Pre-intern primitive symbols before creating VM
        let collection_symbols = intern_collection_primitives(&self.interner);
        let introspection_symbols = intern_introspection_primitives(&self.interner);
        let arithmetic_symbols = intern_arithmetic_primitives(&self.interner);
        let comparison_symbols = intern_comparison_primitives(&self.interner);
        let type_predicate_symbols = intern_type_predicates(&self.interner);
        let metadata_symbols = intern_metadata_primitives(&self.interner);
        let symbol_symbols = intern_symbol_primitives(&self.interner);
        let var_symbols = intern_var_primitives(&self.interner);
        let namespace_symbols = intern_namespace_primitives(&self.interner);

        // Create a VM for this evaluation
        let mut vm = Vm::new(&self.interner);

        // Set the source ID for error reporting
        vm.set_source(source_id);

        // Restore persistent globals into the VM
        *vm.globals_mut() = self.globals.clone();

        // Restore namespace registry if we have one
        if let Some(ref registry) = self.namespace_registry {
            *vm.namespace_registry_mut() = registry.clone();
        }

        // Set the VM's current namespace to match the saved namespace
        vm.set_current_namespace(current_ns);

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

        // Register var primitives (var-get, var-set!)
        register_var_primitives(&mut vm, &var_symbols);

        // Register namespace primitives (require, namespace-add-alias, etc.)
        register_namespace_primitives(&mut vm, &namespace_symbols);

        // Set up macro introspection functions
        vm.set_macro_registry(&self.macros);
        register_introspection_primitives(&mut vm, &introspection_symbols);

        // Propagate lona.core vars to all namespaces (auto-refer)
        vm.namespace_registry_mut().refer_core_to_all();

        // Execute
        let result = vm.execute(&chunk).map_err(|err| {
            // Format the VM error using lonala-human for Rust-style error messages
            let config = FormatConfig::new();
            render_error(&err, &self.sources, &self.interner, &config)
        })?;

        // Save globals, namespace registry, and current namespace back to persistent storage
        self.globals = vm.globals().clone();
        self.namespace_registry = Some(vm.namespace_registry().clone());
        self.current_namespace = Some(vm.current_namespace());

        Ok(result)
    }
}

// Re-export interactive types for use in main.rs
#[cfg(not(feature = "integration-test"))]
pub use crate::interactive::{InteractiveRepl, UartConsole};
