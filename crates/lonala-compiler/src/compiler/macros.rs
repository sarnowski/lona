// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Macro registry for persistent macro storage.
//!
//! This module provides a shareable registry of macro definitions that can
//! be persisted across compilation sessions. This is essential for REPL usage
//! where macros defined in one evaluation should be available in subsequent
//! evaluations.
//!
//! # Example
//!
//! ```ignore
//! let mut registry = MacroRegistry::new();
//!
//! // First REPL eval: define a macro
//! let chunk1 = compile_with_registry("(defmacro when [test & body] ...)", &mut interner, &mut registry)?;
//!
//! // Second REPL eval: use the macro (it persists in the registry)
//! let chunk2 = compile_with_registry("(when true (print \"hi\"))", &mut interner, &mut registry)?;
//! ```

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lona_core::chunk::Chunk;
use lona_core::symbol;

/// A compiled macro definition.
///
/// Macros are compile-time functions that transform AST. The macro body
/// is compiled to bytecode and stored here for later expansion.
#[derive(Debug, Clone)]
pub struct MacroDefinition {
    /// Compiled bytecode for the macro transformer function.
    /// When called, receives unevaluated arguments and returns transformed AST.
    chunk: Arc<Chunk>,
    /// Number of parameters the macro expects.
    arity: u8,
    /// Macro name for error messages and debugging.
    name: String,
}

impl MacroDefinition {
    /// Creates a new macro definition.
    #[inline]
    #[must_use]
    pub const fn new(chunk: Arc<Chunk>, arity: u8, name: String) -> Self {
        Self { chunk, arity, name }
    }

    /// Returns a reference to the macro's compiled chunk.
    #[inline]
    #[must_use]
    pub const fn chunk(&self) -> &Arc<Chunk> {
        &self.chunk
    }

    /// Returns the macro's arity.
    #[inline]
    #[must_use]
    pub const fn arity(&self) -> u8 {
        self.arity
    }

    /// Returns the macro's name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Registry of macro definitions.
///
/// Stores macro definitions by symbol ID, allowing them to persist across
/// compilation sessions. This is designed to be shared between the REPL
/// and the compiler.
///
/// # Thread Safety
///
/// The registry is not thread-safe. For concurrent access, wrap it in
/// appropriate synchronization primitives.
#[derive(Debug, Clone, Default)]
pub struct MacroRegistry {
    /// Macro definitions keyed by symbol ID.
    macros: BTreeMap<symbol::Id, MacroDefinition>,
}

impl MacroRegistry {
    /// Creates a new empty macro registry.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            macros: BTreeMap::new(),
        }
    }

    /// Registers a macro definition.
    ///
    /// If a macro with the same name already exists, it is replaced.
    #[inline]
    pub fn register(&mut self, name: symbol::Id, definition: MacroDefinition) {
        let _previous: Option<MacroDefinition> = self.macros.insert(name, definition);
    }

    /// Returns the macro definition for a symbol, if it exists.
    #[inline]
    #[must_use]
    pub fn get(&self, name: symbol::Id) -> Option<&MacroDefinition> {
        self.macros.get(&name)
    }

    /// Returns `true` if a macro with the given name is registered.
    #[inline]
    #[must_use]
    pub fn contains(&self, name: symbol::Id) -> bool {
        self.macros.contains_key(&name)
    }

    /// Returns the number of macros in the registry.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.macros.len()
    }

    /// Returns `true` if the registry contains no macros.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.macros.is_empty()
    }

    /// Returns an iterator over all macro names in the registry.
    #[inline]
    pub fn names(&self) -> impl Iterator<Item = symbol::Id> + '_ {
        self.macros.keys().copied()
    }

    /// Returns an iterator over all macro definitions in the registry.
    #[inline]
    pub fn definitions(&self) -> impl Iterator<Item = &MacroDefinition> {
        self.macros.values()
    }

    /// Returns an iterator over (name, definition) pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (symbol::Id, &MacroDefinition)> {
        self.macros.iter().map(|(&key, value)| (key, value))
    }

    /// Removes a macro from the registry.
    ///
    /// Returns the removed definition, or `None` if the macro was not found.
    #[inline]
    pub fn remove(&mut self, name: symbol::Id) -> Option<MacroDefinition> {
        self.macros.remove(&name)
    }

    /// Clears all macros from the registry.
    #[inline]
    pub fn clear(&mut self) {
        self.macros.clear();
    }

    /// Merges another registry into this one.
    ///
    /// Macros from the other registry overwrite existing macros with the
    /// same name.
    #[inline]
    pub fn merge(&mut self, other: &Self) {
        for (name, definition) in &other.macros {
            let _previous: Option<MacroDefinition> = self.macros.insert(*name, definition.clone());
        }
    }

    /// Returns a vector of all macro names for introspection.
    #[inline]
    #[must_use]
    pub fn all_names(&self) -> Vec<symbol::Id> {
        self.macros.keys().copied().collect()
    }
}

/// Error returned by macro expansion.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MacroExpansionError {
    /// Error message describing what went wrong.
    pub message: String,
}

impl MacroExpansionError {
    /// Creates a new macro expansion error.
    #[inline]
    #[must_use]
    pub const fn new(message: String) -> Self {
        Self { message }
    }
}

/// Trait for executing macro transformers.
///
/// This trait abstracts the VM execution needed to run macro transformers.
/// The compiler uses this trait to expand macros at compile time without
/// directly depending on the VM crate.
///
/// # Implementors
///
/// The REPL or any other compile-time execution environment should implement
/// this trait using their VM infrastructure.
pub trait MacroExpander {
    /// Executes a macro transformer with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `definition` - The compiled macro definition
    /// * `args` - The macro arguments as runtime Values
    /// * `interner` - The symbol interner for symbol resolution (mutable to allow
    ///   interning of primitive function names needed by quasiquote expansion)
    ///
    /// # Returns
    ///
    /// The expanded form as a Value, which will be converted back to AST.
    ///
    /// # Errors
    ///
    /// Returns an error if the macro transformer fails during execution.
    fn expand(
        &mut self,
        definition: &MacroDefinition,
        args: Vec<lona_core::value::Value>,
        interner: &mut symbol::Interner,
    ) -> Result<lona_core::value::Value, MacroExpansionError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::symbol::Interner;

    #[test]
    fn new_registry_is_empty() {
        let registry = MacroRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn register_and_get() {
        let mut interner = Interner::new();
        let mut registry = MacroRegistry::new();

        let name = interner.intern("my-macro");
        let chunk = Arc::new(Chunk::new());
        let def = MacroDefinition::new(chunk, 2, String::from("my-macro"));

        registry.register(name, def);

        assert!(registry.contains(name));
        assert_eq!(registry.len(), 1);

        let retrieved = registry.get(name).unwrap();
        assert_eq!(retrieved.arity(), 2);
        assert_eq!(retrieved.name(), "my-macro");
    }

    #[test]
    fn register_overwrites() {
        let mut interner = Interner::new();
        let mut registry = MacroRegistry::new();

        let name = interner.intern("my-macro");

        let chunk1 = Arc::new(Chunk::new());
        let def1 = MacroDefinition::new(chunk1, 1, String::from("my-macro"));
        registry.register(name, def1);

        let chunk2 = Arc::new(Chunk::new());
        let def2 = MacroDefinition::new(chunk2, 3, String::from("my-macro"));
        registry.register(name, def2);

        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(name).unwrap().arity(), 3);
    }

    #[test]
    fn contains_unknown_returns_false() {
        let mut interner = Interner::new();
        let registry = MacroRegistry::new();

        let unknown = interner.intern("unknown-macro");
        assert!(!registry.contains(unknown));
    }

    #[test]
    fn get_unknown_returns_none() {
        let mut interner = Interner::new();
        let registry = MacroRegistry::new();

        let unknown = interner.intern("unknown-macro");
        assert!(registry.get(unknown).is_none());
    }

    #[test]
    fn remove_macro() {
        let mut interner = Interner::new();
        let mut registry = MacroRegistry::new();

        let name = interner.intern("my-macro");
        let chunk = Arc::new(Chunk::new());
        let def = MacroDefinition::new(chunk, 2, String::from("my-macro"));

        registry.register(name, def);
        assert!(registry.contains(name));

        let removed = registry.remove(name);
        assert!(removed.is_some());
        assert!(!registry.contains(name));
    }

    #[test]
    fn remove_unknown_returns_none() {
        let mut interner = Interner::new();
        let mut registry = MacroRegistry::new();

        let unknown = interner.intern("unknown-macro");
        assert!(registry.remove(unknown).is_none());
    }

    #[test]
    fn clear_removes_all() {
        let mut interner = Interner::new();
        let mut registry = MacroRegistry::new();

        let name1 = interner.intern("macro1");
        let name2 = interner.intern("macro2");

        let chunk1 = Arc::new(Chunk::new());
        let def1 = MacroDefinition::new(chunk1, 1, String::from("macro1"));
        registry.register(name1, def1);

        let chunk2 = Arc::new(Chunk::new());
        let def2 = MacroDefinition::new(chunk2, 2, String::from("macro2"));
        registry.register(name2, def2);

        assert_eq!(registry.len(), 2);

        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn merge_registries() {
        let mut interner = Interner::new();
        let mut registry1 = MacroRegistry::new();
        let mut registry2 = MacroRegistry::new();

        let name1 = interner.intern("macro1");
        let name2 = interner.intern("macro2");

        let chunk1 = Arc::new(Chunk::new());
        let def1 = MacroDefinition::new(chunk1, 1, String::from("macro1"));
        registry1.register(name1, def1);

        let chunk2 = Arc::new(Chunk::new());
        let def2 = MacroDefinition::new(chunk2, 2, String::from("macro2"));
        registry2.register(name2, def2);

        registry1.merge(&registry2);

        assert_eq!(registry1.len(), 2);
        assert!(registry1.contains(name1));
        assert!(registry1.contains(name2));
    }

    #[test]
    fn iterate_names() {
        let mut interner = Interner::new();
        let mut registry = MacroRegistry::new();

        let name1 = interner.intern("macro1");
        let name2 = interner.intern("macro2");

        let chunk1 = Arc::new(Chunk::new());
        let def1 = MacroDefinition::new(chunk1, 1, String::from("macro1"));
        registry.register(name1, def1);

        let chunk2 = Arc::new(Chunk::new());
        let def2 = MacroDefinition::new(chunk2, 2, String::from("macro2"));
        registry.register(name2, def2);

        let names: Vec<_> = registry.names().collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&name1));
        assert!(names.contains(&name2));
    }

    #[test]
    fn clone_registry() {
        let mut interner = Interner::new();
        let mut registry = MacroRegistry::new();

        let name = interner.intern("my-macro");
        let chunk = Arc::new(Chunk::new());
        let def = MacroDefinition::new(chunk, 2, String::from("my-macro"));

        registry.register(name, def);

        let cloned = registry.clone();
        assert_eq!(cloned.len(), 1);
        assert!(cloned.contains(name));
    }

    #[test]
    fn default_is_empty() {
        let registry = MacroRegistry::default();
        assert!(registry.is_empty());
    }
}
