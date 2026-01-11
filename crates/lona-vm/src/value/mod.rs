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

mod function;
mod heap;
mod namespace;
mod printer;
mod var;

pub use function::{HeapClosure, HeapCompiledFn};
pub use heap::{HeapMap, HeapString, HeapTuple, Pair};
pub use namespace::Namespace;
pub use printer::print_value;
pub use var::{VarContent, VarSlot, var_flags};

use crate::Vaddr;
use core::fmt;

/// A Lonala value.
///
/// The value representation uses 16 bytes (tag + payload).
/// Immediate values are stored inline, heap values store a `Vaddr` pointer.
///
/// Tags follow the VM specification (see `docs/architecture/virtual-machine.md`):
/// - 0x0-0x8: Basic types (nil, bool, int, string, pair, symbol, keyword, tuple, map)
/// - 0x9-0xB: Callable types (function, closure, native function)
/// - 0xC-0xD: Reference types (var, namespace)
/// - 0xE: Unbound sentinel
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
    /// Heap-allocated vector (pointer to `HeapTuple`, same layout as tuple).
    Vector(Vaddr) = 15,
    /// Compiled function without captures (pointer to `HeapCompiledFn`).
    CompiledFn(Vaddr) = 9,
    /// Function with captured values (pointer to `HeapClosure`).
    Closure(Vaddr) = 10,
    /// Native function (immediate value: intrinsic ID).
    NativeFn(u16) = 11,
    /// Var reference (pointer to `VarSlot` in code region).
    Var(Vaddr) = 12,
    /// Heap-allocated namespace (pointer to `Namespace`).
    Namespace(Vaddr) = 13,
    /// Sentinel for uninitialized vars (immediate, no payload).
    Unbound = 14,
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

    /// Create a vector value from a heap address.
    #[inline]
    #[must_use]
    pub const fn vector(addr: Vaddr) -> Self {
        Self::Vector(addr)
    }

    /// Create a namespace value from a heap address.
    #[inline]
    #[must_use]
    pub const fn namespace(addr: Vaddr) -> Self {
        Self::Namespace(addr)
    }

    /// Create a var value from a code region address.
    #[inline]
    #[must_use]
    pub const fn var(addr: Vaddr) -> Self {
        Self::Var(addr)
    }

    /// Create a compiled function value from a heap address.
    #[inline]
    #[must_use]
    pub const fn compiled_fn(addr: Vaddr) -> Self {
        Self::CompiledFn(addr)
    }

    /// Create a closure value from a heap address.
    #[inline]
    #[must_use]
    pub const fn closure(addr: Vaddr) -> Self {
        Self::Closure(addr)
    }

    /// Create a native function value from an intrinsic ID.
    #[inline]
    #[must_use]
    pub const fn native_fn(id: u16) -> Self {
        Self::NativeFn(id)
    }

    /// Create an unbound sentinel value.
    #[inline]
    #[must_use]
    pub const fn unbound() -> Self {
        Self::Unbound
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

    /// Check if this value is a vector.
    #[inline]
    #[must_use]
    pub const fn is_vector(&self) -> bool {
        matches!(self, Self::Vector(_))
    }

    /// Check if this value is a namespace.
    #[inline]
    #[must_use]
    pub const fn is_namespace(&self) -> bool {
        matches!(self, Self::Namespace(_))
    }

    /// Check if this value is a var.
    #[inline]
    #[must_use]
    pub const fn is_var(&self) -> bool {
        matches!(self, Self::Var(_))
    }

    /// Check if this value is a compiled function.
    #[inline]
    #[must_use]
    pub const fn is_compiled_fn(&self) -> bool {
        matches!(self, Self::CompiledFn(_))
    }

    /// Check if this value is a closure.
    #[inline]
    #[must_use]
    pub const fn is_closure(&self) -> bool {
        matches!(self, Self::Closure(_))
    }

    /// Check if this value is a native function.
    #[inline]
    #[must_use]
    pub const fn is_native_fn(&self) -> bool {
        matches!(self, Self::NativeFn(_))
    }

    /// Check if this value is callable (function, closure, or native function).
    #[inline]
    #[must_use]
    pub const fn is_fn(&self) -> bool {
        matches!(
            self,
            Self::CompiledFn(_) | Self::Closure(_) | Self::NativeFn(_)
        )
    }

    /// Check if this value is the unbound sentinel.
    #[inline]
    #[must_use]
    pub const fn is_unbound(&self) -> bool {
        matches!(self, Self::Unbound)
    }

    /// Get the type name of this value for error messages.
    #[inline]
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::String(_) => "string",
            Self::Pair(_) => "pair",
            Self::Symbol(_) => "symbol",
            Self::Keyword(_) => "keyword",
            Self::Tuple(_) => "tuple",
            Self::Map(_) => "map",
            Self::Vector(_) => "vector",
            Self::CompiledFn(_) => "function",
            Self::Closure(_) => "closure",
            Self::NativeFn(_) => "native-function",
            Self::Var(_) => "var",
            Self::Namespace(_) => "namespace",
            Self::Unbound => "unbound",
        }
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
            Self::Vector(addr) => write!(f, "Vector({addr:?})"),
            Self::CompiledFn(addr) => write!(f, "CompiledFn({addr:?})"),
            Self::Closure(addr) => write!(f, "Closure({addr:?})"),
            Self::NativeFn(id) => write!(f, "NativeFn({id})"),
            Self::Var(addr) => write!(f, "Var({addr:?})"),
            Self::Namespace(addr) => write!(f, "Namespace({addr:?})"),
            Self::Unbound => write!(f, "Unbound"),
        }
    }
}
