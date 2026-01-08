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
    /// Heap-allocated keyword (pointer to `HeapString`).
    Keyword(Vaddr) = 6,
    /// Heap-allocated tuple (pointer to `HeapTuple`).
    Tuple(Vaddr) = 7,
    /// Heap-allocated map (pointer to `HeapMap`).
    Map(Vaddr) = 8,
    /// Heap-allocated namespace (pointer to `Namespace`).
    Namespace(Vaddr) = 9,
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

    /// Create a keyword value from a heap address.
    #[inline]
    #[must_use]
    pub const fn keyword(addr: Vaddr) -> Self {
        Self::Keyword(addr)
    }

    /// Create a tuple value from a heap address.
    #[inline]
    #[must_use]
    pub const fn tuple(addr: Vaddr) -> Self {
        Self::Tuple(addr)
    }

    /// Create a map value from a heap address.
    #[inline]
    #[must_use]
    pub const fn map(addr: Vaddr) -> Self {
        Self::Map(addr)
    }

    /// Create a namespace value from a heap address.
    #[inline]
    #[must_use]
    pub const fn namespace(addr: Vaddr) -> Self {
        Self::Namespace(addr)
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

    /// Check if this value is a string.
    #[inline]
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
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

    /// Check if this value is a keyword.
    #[inline]
    #[must_use]
    pub const fn is_keyword(&self) -> bool {
        matches!(self, Self::Keyword(_))
    }

    /// Check if this value is a symbol.
    #[inline]
    #[must_use]
    pub const fn is_symbol(&self) -> bool {
        matches!(self, Self::Symbol(_))
    }

    /// Check if this value is a tuple.
    #[inline]
    #[must_use]
    pub const fn is_tuple(&self) -> bool {
        matches!(self, Self::Tuple(_))
    }

    /// Check if this value is a map.
    #[inline]
    #[must_use]
    pub const fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    /// Check if this value is a namespace.
    #[inline]
    #[must_use]
    pub const fn is_namespace(&self) -> bool {
        matches!(self, Self::Namespace(_))
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
            Self::Keyword(addr) => write!(f, "Keyword({addr:?})"),
            Self::Tuple(addr) => write!(f, "Tuple({addr:?})"),
            Self::Map(addr) => write!(f, "Map({addr:?})"),
            Self::Namespace(addr) => write!(f, "Namespace({addr:?})"),
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

/// Heap-allocated tuple header.
///
/// Stored in memory as:
/// - 4 bytes: length (u32)
/// - `len * size_of::<Value>()` bytes: elements (array of Values)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapTuple {
    /// Number of elements in the tuple.
    pub len: u32,
    // Followed by `len` Values (not represented in struct)
}

impl HeapTuple {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a tuple of given length.
    #[inline]
    #[must_use]
    pub const fn alloc_size(len: usize) -> usize {
        Self::HEADER_SIZE + len * core::mem::size_of::<Value>()
    }
}

/// Heap-allocated map header.
///
/// Maps are implemented as association lists: linked lists of `[key value]` tuples.
/// The `entries` field points to a Pair chain where each `first` is a 2-element tuple.
///
/// Stored in memory as:
/// - 16 bytes: entries (`Value::Pair` or `Value::Nil` for empty map)
///
/// Example structure for `%{:a 1 :b 2}`:
/// ```text
/// HeapMap { entries } → Pair([:a 1], Pair([:b 2], nil))
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapMap {
    /// Head of the association list (Pair chain or nil).
    pub entries: Value,
}

impl HeapMap {
    /// Size of the header in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// Heap-allocated namespace header.
///
/// Namespaces are containers for var bindings. The `name` field is a symbol,
/// and `mappings` is a `Value::Map` holding symbol→var mappings.
///
/// Stored in memory as:
/// - 16 bytes: name (`Value::Symbol`)
/// - 16 bytes: mappings (`Value::Map`)
///
/// Example: namespace `my.app` with var `x`:
/// ```text
/// Namespace { name: 'my.app, mappings: %{'x → var-addr} }
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Namespace {
    /// The namespace name (a symbol).
    pub name: Value,
    /// Symbol→Vaddr mappings (a map). In the future, this will map to `VarSlot`s.
    /// For now, this is a map of symbol→value.
    pub mappings: Value,
}

impl Namespace {
    /// Size of the namespace header in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}
