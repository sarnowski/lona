// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Value representation for the Lonala language.
//!
//! Values are the runtime representation of Lonala expressions.
//! Immediate values (nil, bool, int) fit in registers.
//! Compound values (strings, pairs, symbols) are heap-allocated.

#[cfg(test)]
mod mod_test;
#[cfg(test)]
mod printer_test;

mod printer;

pub use printer::print_value;

use crate::Vaddr;
use core::fmt;

/// A Lonala value.
///
/// The value representation uses 16 bytes (tag + payload).
/// Immediate values are stored inline, heap values store a `Vaddr` pointer.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Value {
    /// The nil value (empty list, false-ish).
    #[default]
    Nil = 0,
    /// Boolean true or false.
    Bool(bool) = 1,
    /// 64-bit signed integer.
    Int(i64) = 2,
    /// Heap-allocated string (pointer to `HeapString`).
    String(Vaddr) = 3,
    /// Heap-allocated pair (pointer to `Pair`).
    Pair(Vaddr) = 4,
    /// Heap-allocated symbol (pointer to `HeapString`).
    Symbol(Vaddr) = 5,
}

impl Value {
    /// Create a nil value.
    #[inline]
    #[must_use]
    pub const fn nil() -> Self {
        Self::Nil
    }

    /// Create a boolean value.
    #[inline]
    #[must_use]
    pub const fn bool(b: bool) -> Self {
        Self::Bool(b)
    }

    /// Create an integer value.
    #[inline]
    #[must_use]
    pub const fn int(n: i64) -> Self {
        Self::Int(n)
    }

    /// Create a string value from a heap address.
    #[inline]
    #[must_use]
    pub const fn string(addr: Vaddr) -> Self {
        Self::String(addr)
    }

    /// Create a pair value from a heap address.
    #[inline]
    #[must_use]
    pub const fn pair(addr: Vaddr) -> Self {
        Self::Pair(addr)
    }

    /// Create a symbol value from a heap address.
    #[inline]
    #[must_use]
    pub const fn symbol(addr: Vaddr) -> Self {
        Self::Symbol(addr)
    }

    /// Check if this value is nil.
    #[inline]
    #[must_use]
    pub const fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }

    /// Check if this value is truthy (not nil and not false).
    #[inline]
    #[must_use]
    pub const fn is_truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Bool(false))
    }

    /// Check if this value is a pair.
    #[inline]
    #[must_use]
    pub const fn is_pair(&self) -> bool {
        matches!(self, Self::Pair(_))
    }

    /// Check if this value is a proper list (nil or pair ending in nil).
    /// Note: This doesn't traverse the list, just checks immediate structure.
    #[inline]
    #[must_use]
    pub const fn is_list_head(&self) -> bool {
        matches!(self, Self::Nil | Self::Pair(_))
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "Nil"),
            Self::Bool(b) => write!(f, "Bool({b})"),
            Self::Int(n) => write!(f, "Int({n})"),
            Self::String(addr) => write!(f, "String({addr:?})"),
            Self::Pair(addr) => write!(f, "Pair({addr:?})"),
            Self::Symbol(addr) => write!(f, "Symbol({addr:?})"),
        }
    }
}

/// Heap-allocated string header.
///
/// Stored in memory as:
/// - 4 bytes: length (u32)
/// - `len` bytes: UTF-8 data (no null terminator)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapString {
    /// Length of the string in bytes.
    pub len: u32,
    // Followed by `len` UTF-8 bytes (not represented in struct)
}

impl HeapString {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a string of given length.
    #[inline]
    #[must_use]
    pub const fn alloc_size(len: usize) -> usize {
        Self::HEADER_SIZE + len
    }
}

/// Heap-allocated pair.
///
/// Used to build lists: (1 2 3) = Pair(1, Pair(2, Pair(3, Nil)))
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Pair {
    /// First element of the pair.
    pub first: Value,
    /// Rest of the list (second element of the pair).
    pub rest: Value,
}

impl Pair {
    /// Size of a pair in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Create a new pair.
    #[inline]
    #[must_use]
    pub const fn new(first: Value, rest: Value) -> Self {
        Self { first, rest }
    }
}
