// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Type definitions for the VM interpreter.
//!
//! This module contains the types used for control flow and tail call
//! optimization in the interpreter.

use alloc::sync::Arc;
use alloc::vec::Vec;

use lona_core::chunk::Chunk;
use lona_core::source;
use lona_core::value::{Function, Value};

/// Result of dispatching a single opcode.
///
/// Used internally by the interpreter to control execution flow.
pub enum DispatchResult {
    /// Continue to the next instruction.
    Continue,
    /// Return from the current function with a value.
    Return(Value),
    /// Perform a tail call (replace current frame instead of pushing new one).
    ///
    /// When a function call is in tail position, returning this variant
    /// signals to the trampoline to replace the current frame and continue
    /// execution without growing the Rust stack.
    TailCall(TailCallData),
}

/// Data needed to perform a tail call without growing the stack.
///
/// When a function call is in tail position, instead of recursively calling
/// `run()`, we return this data to the trampoline loop which replaces the
/// current frame and continues execution.
pub struct TailCallData {
    /// The function to call.
    pub function: Function,
    /// Arguments to pass to the function.
    pub arguments: Vec<Value>,
    /// Source ID for error reporting.
    pub source: source::Id,
}

impl TailCallData {
    /// Creates new tail call data.
    #[inline]
    pub const fn new(function: Function, arguments: Vec<Value>, source: source::Id) -> Self {
        Self {
            function,
            arguments,
            source,
        }
    }

    /// Returns the source ID.
    #[inline]
    pub const fn source(&self) -> source::Id {
        self.source
    }
}

/// Result of setting up a tail call frame.
///
/// Contains all data needed to create a new frame for a tail call.
pub struct TailCallSetup {
    /// The chunk to execute.
    pub chunk: Arc<Chunk>,
    /// Source ID for error reporting.
    pub source: source::Id,
    /// Captured upvalues for the function.
    pub upvalues: Arc<[Value]>,
}

impl TailCallSetup {
    /// Creates a new tail call setup.
    #[inline]
    pub const fn new(chunk: Arc<Chunk>, source: source::Id, upvalues: Arc<[Value]>) -> Self {
        Self {
            chunk,
            source,
            upvalues,
        }
    }

    /// Returns the chunk Arc, consuming self.
    #[inline]
    pub fn into_parts(self) -> (Arc<Chunk>, source::Id, Arc<[Value]>) {
        (self.chunk, self.source, self.upvalues)
    }
}
