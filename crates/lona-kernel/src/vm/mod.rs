// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Bytecode virtual machine for the Lonala language.
//!
//! The VM executes compiled bytecode from `Chunk` objects. It is a
//! register-based VM (like Lua) with up to 256 registers per frame.
//!
//! # Architecture
//!
//! - **Register-based**: Instructions reference registers directly via A, B, C fields
//! - **RK encoding**: Constants can be used directly in instruction operands
//! - **Global storage**: Symbol-to-value mapping for global variables
//!
//! # Example
//!
//! ```ignore
//! use lona_core::symbol::Interner;
//! use lonala_compiler::compile;
//! use lona_kernel::vm::Vm;
//!
//! let mut interner = Interner::new();
//! let chunk = compile("(+ 1 2)", &mut interner).unwrap();
//! let mut vm = Vm::new(&interner);
//! let result = vm.execute(&chunk).unwrap();
//! // result == Value::Integer(3)
//! ```

mod error;
mod frame;
mod globals;
mod interpreter;

pub use error::Error;
pub use frame::Frame;
pub use globals::Globals;
pub use interpreter::Vm;
