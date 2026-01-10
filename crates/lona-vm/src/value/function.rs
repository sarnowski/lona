// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Function-related heap types.
//!
//! This module contains the heap structures for compiled functions and closures.

use super::Value;
use crate::Vaddr;

/// Heap-allocated compiled function header.
///
/// A compiled function is a pure function (no captures) produced by `(fn* [args] body)`.
/// Contains bytecode and metadata for execution.
///
/// Stored in memory as:
/// - Header: arity, variadic flag, locals count, code length
/// - Followed by: bytecode instructions (array of u32)
/// - Followed by: constants pool (array of Values)
///
/// Note: The constant pool is stored separately to allow variable-length bytecode.
/// Constants follow immediately after bytecode.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapCompiledFn {
    /// Number of required parameters.
    pub arity: u8,
    /// If true, accepts variable arguments (last param collects rest).
    pub variadic: bool,
    /// Number of Y (local) registers needed.
    pub num_locals: u8,
    /// Padding byte for alignment (always 0).
    pub padding: u8,
    /// Length of bytecode in u32 instructions.
    pub code_len: u32,
    /// Number of constants in the constant pool.
    pub constants_len: u32,
    /// Source line number (0 if unknown).
    pub source_line: u32,
    /// Padding for 8-byte alignment.
    pub padding2: u32,
    /// Source file path (Vaddr to string, or `Vaddr::null()` if unknown).
    pub source_file: Vaddr,
    // Followed by:
    // - `code_len` u32 instructions
    // - `constants_len` Values (constant pool)
}

impl HeapCompiledFn {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a compiled function.
    ///
    /// # Arguments
    /// * `code_len` - Number of bytecode instructions (u32)
    /// * `constants_len` - Number of constants in the pool
    #[inline]
    #[must_use]
    pub const fn alloc_size(code_len: usize, constants_len: usize) -> usize {
        Self::HEADER_SIZE
            + code_len * core::mem::size_of::<u32>()
            + constants_len * core::mem::size_of::<Value>()
    }

    /// Offset from header start to the bytecode array.
    #[inline]
    #[must_use]
    pub const fn bytecode_offset() -> usize {
        Self::HEADER_SIZE
    }

    /// Offset from header start to the constants pool.
    #[inline]
    #[must_use]
    pub const fn constants_offset(code_len: usize) -> usize {
        Self::HEADER_SIZE + code_len * core::mem::size_of::<u32>()
    }
}

/// Heap-allocated closure header.
///
/// A closure is a function paired with captured values from its lexical environment.
/// Produced when `fn*` references free variables from enclosing scope.
///
/// Stored in memory as:
/// - Header: function pointer, captures count
/// - Followed by: captured values (array of Values)
///
/// The `function` field points to a `HeapCompiledFn` that contains the bytecode.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapClosure {
    /// Pointer to the underlying `HeapCompiledFn`.
    pub function: Vaddr,
    /// Number of captured values.
    pub captures_len: u32,
    /// Padding for alignment (always 0).
    pub padding: u32,
    // Followed by `captures_len` Values (captured environment)
}

impl HeapClosure {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a closure.
    ///
    /// # Arguments
    /// * `captures_len` - Number of captured values
    #[inline]
    #[must_use]
    pub const fn alloc_size(captures_len: usize) -> usize {
        Self::HEADER_SIZE + captures_len * core::mem::size_of::<Value>()
    }

    /// Offset from header start to the captures array.
    #[inline]
    #[must_use]
    pub const fn captures_offset() -> usize {
        Self::HEADER_SIZE
    }
}
