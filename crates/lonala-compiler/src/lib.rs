// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Bytecode compiler for the Lonala language.
//!
//! This crate compiles Lonala AST into executable bytecode for the Lonala
//! virtual machine. It includes the bytecode format definition, instruction
//! encoding/decoding, and the AST-to-bytecode compiler.
//!
//! # Architecture
//!
//! Lonala uses a register-based virtual machine inspired by Lua 5.x and BEAM.
//! For design rationale and specification, see the architecture document at
//! `docs/architecture/register-based-vm.md` (from the repository root).
//!
//! # Modules
//!
//! - [`opcode`] - Opcode enum and instruction encoding/decoding (re-exported from lona-core)
//! - [`chunk`] - Bytecode chunk and constant pool structures (re-exported from lona-core)
//! - [`error`] - Compilation error types
//! - [`compiler`] - AST to bytecode compiler
//!
//! # Example
//!
//! ```
//! use lona_core::symbol::Interner;
//! use lonala_compiler::compile;
//!
//! let mut interner = Interner::new();
//! let chunk = compile("(+ 1 2)", &mut interner).unwrap();
//!
//! // The chunk can now be executed by the VM (Phase 2.5)
//! println!("{}", chunk.disassemble());
//! ```

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod compiler;
pub mod error;

// Re-export bytecode types from lona-core
#[cfg(feature = "alloc")]
pub use lona_core::chunk::{Chunk, Constant, ConstantPoolFullError};
pub use lona_core::opcode::{self, Opcode};
pub use lona_core::span::Span;

#[cfg(feature = "alloc")]
pub use compiler::{
    CompileError, Compiler, MacroDefinition, MacroExpander, MacroExpansionError, MacroRegistry,
    compile, compile_with_expansion, compile_with_registry,
};
pub use error::Error;
