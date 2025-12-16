// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Bytecode compiler for the Lonala language.
//!
//! This crate defines the bytecode format for the Lonala virtual machine and
//! will provide compilation from AST to bytecode in later phases.
//!
//! # Architecture
//!
//! Lonala uses a register-based virtual machine inspired by Lua 5.x and BEAM.
//! For design rationale and specification, see the architecture document at
//! `docs/architecture/register-based-vm.md` (from the repository root).
//!
//! # Modules
//!
//! - [`opcode`] - Opcode enum and instruction encoding/decoding
//! - [`chunk`] - Bytecode chunk and constant pool structures
//! - [`error`] - Compilation error types

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod chunk;
pub mod error;
pub mod opcode;

#[cfg(feature = "alloc")]
pub use chunk::{Chunk, Constant};
pub use error::Error;
pub use opcode::Opcode;
