// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Macro expansion support for the VM.
//!
//! This module provides a [`MacroExpander`] implementation that uses the VM
//! to execute macro transformers at compile time.

use alloc::format;
use alloc::vec::Vec;

use lona_core::symbol::Interner;
use lona_core::value::Value;
use lonala_compiler::{MacroDefinition, MacroExpander, MacroExpansionError};

use super::Vm;
use super::collections::{intern_primitives, register_primitives};

/// A macro expander that uses the VM to execute macro transformers.
///
/// This struct implements the [`MacroExpander`] trait, allowing the compiler
/// to expand macros at compile time by executing their transformer functions.
///
/// # Example
///
/// ```ignore
/// use lona_core::symbol::Interner;
/// use lona_kernel::vm::MacroExpander;
/// use lonala_compiler::{compile_with_expansion, MacroRegistry};
///
/// let mut interner = Interner::new();
/// let mut registry = MacroRegistry::new();
/// let mut expander = MacroExpander::new();
///
/// let chunk = compile_with_expansion(
///     "(defmacro double [x] `(+ ~x ~x))",
///     &mut interner,
///     &mut registry,
///     &mut expander
/// ).unwrap();
/// ```
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Expander;

impl Expander {
    /// Creates a new macro expander.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl MacroExpander for Expander {
    /// Executes a macro transformer with the given arguments.
    ///
    /// Creates a fresh VM instance with collection primitives registered,
    /// sets up the macro arguments as registers, executes the macro's
    /// compiled chunk, and returns the result.
    ///
    /// The collection primitives (`list`, `concat`, `vec`, etc.) are required
    /// for quasiquote expansion to work properly.
    #[inline]
    fn expand(
        &mut self,
        definition: &MacroDefinition,
        args: Vec<Value>,
        interner: &mut Interner,
    ) -> Result<Value, MacroExpansionError> {
        // Verify arity
        let expected_arity = usize::from(definition.arity());
        if args.len() != expected_arity {
            return Err(MacroExpansionError::new(format!(
                "macro '{}' expects {} arguments, got {}",
                definition.name(),
                expected_arity,
                args.len()
            )));
        }

        // Intern the collection primitive symbols (required for quasiquote)
        let collection_symbols = intern_primitives(interner);

        // Create a fresh VM for macro expansion
        let mut vm = Vm::new(interner);

        // Register collection primitives (list, concat, vec, etc.)
        // These are needed for quasiquote expansion
        register_primitives(&mut vm, &collection_symbols);

        // Execute the macro's chunk with the arguments as initial register values
        let result = vm
            .execute_with_args(definition.chunk(), &args)
            .map_err(|vm_err| MacroExpansionError::new(format!("{vm_err:?}")))?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use alloc::vec;
    use lona_core::chunk::Chunk;
    use lona_core::integer::Integer;
    use lona_core::opcode::{Opcode, encode_abc};
    use lona_core::span::Span;

    /// Creates a simple macro chunk that returns its first argument.
    fn make_identity_chunk() -> Chunk {
        let mut chunk = Chunk::with_name(alloc::string::String::from("test-macro"));
        chunk.set_arity(1_u8);
        // Return R[0] - the first argument
        chunk.emit(
            encode_abc(Opcode::Return, 0_u8, 1_u8, 0_u8),
            Span::new(0_usize, 1_usize),
        );
        chunk.set_max_registers(1_u8);
        chunk
    }

    #[test]
    fn expander_returns_argument() {
        let mut interner = Interner::new();
        let mut expander = Expander::new();

        let chunk = Arc::new(make_identity_chunk());
        let definition = MacroDefinition::new(chunk, 1_u8, alloc::string::String::from("identity"));

        let args = vec![Value::Integer(Integer::from_i64(42_i64))];
        let result = expander.expand(&definition, args, &mut interner).unwrap();

        assert_eq!(result, Value::Integer(Integer::from_i64(42_i64)));
    }

    #[test]
    fn expander_arity_mismatch() {
        let mut interner = Interner::new();
        let mut expander = Expander::new();

        let chunk = Arc::new(make_identity_chunk());
        let definition = MacroDefinition::new(chunk, 1_u8, alloc::string::String::from("identity"));

        // Pass 2 arguments when 1 is expected
        let args = vec![
            Value::Integer(Integer::from_i64(1_i64)),
            Value::Integer(Integer::from_i64(2_i64)),
        ];
        let result = expander.expand(&definition, args, &mut interner);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("expects 1 arguments"));
    }

    #[test]
    fn expander_returns_nil_for_empty_args() {
        let mut interner = Interner::new();
        let mut expander = Expander::new();

        // Create a macro that takes no arguments and returns nil
        let mut chunk = Chunk::with_name(alloc::string::String::from("nil-macro"));
        chunk.set_arity(0_u8);
        chunk.emit(
            encode_abc(Opcode::LoadNil, 0_u8, 0_u8, 0_u8),
            Span::new(0_usize, 1_usize),
        );
        chunk.emit(
            encode_abc(Opcode::Return, 0_u8, 1_u8, 0_u8),
            Span::new(1_usize, 2_usize),
        );
        chunk.set_max_registers(1_u8);

        let definition = MacroDefinition::new(
            Arc::new(chunk),
            0_u8,
            alloc::string::String::from("nil-macro"),
        );

        let result = expander.expand(&definition, vec![], &mut interner).unwrap();
        assert_eq!(result, Value::Nil);
    }
}
