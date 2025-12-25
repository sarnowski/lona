// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Core value representation for the Lonala language.
//!
//! Values are the fundamental data units in Lonala. This module provides
//! all value types including primitives (nil, bool, integer, float, ratio)
//! and heap-allocated types (string, symbol, list, vector, map, function).

use core::hash::{Hash, Hasher};

#[cfg(feature = "alloc")]
use crate::binary::Binary;
#[cfg(feature = "alloc")]
use crate::integer::Integer;
#[cfg(feature = "alloc")]
use crate::list::List;
#[cfg(feature = "alloc")]
use crate::map::Map;
#[cfg(feature = "alloc")]
use crate::ratio::Ratio;
#[cfg(feature = "alloc")]
use crate::set::Set;
#[cfg(feature = "alloc")]
use crate::string::HeapStr;
use crate::symbol;
#[cfg(feature = "alloc")]
use crate::vector::Vector;

mod accessors;
mod conversions;
mod display;
#[cfg(feature = "alloc")]
mod function;
#[cfg(feature = "alloc")]
mod symbol_value;
#[cfg(feature = "alloc")]
mod var;

#[cfg(test)]
mod tests;

#[cfg(feature = "alloc")]
pub use display::Displayable;
#[cfg(feature = "alloc")]
pub use function::{Function, FunctionBody};
#[cfg(feature = "alloc")]
pub use symbol_value::Symbol;
#[cfg(feature = "alloc")]
pub use var::Var;

/// Runtime value type classification.
///
/// Mirrors the variants of [`Value`] but contains no data, enabling efficient
/// type checking and type-based error messages without cloning values.
///
/// Use as `value::Kind` for clear code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// The nil type.
    Nil,
    /// Boolean type.
    Bool,
    /// Integer type (arbitrary precision with `alloc`, i64 without).
    Integer,
    /// 64-bit floating point type.
    Float,
    /// Exact rational number type.
    #[cfg(feature = "alloc")]
    Ratio,
    /// Interned symbol type.
    Symbol,
    /// Interned keyword type.
    Keyword,
    /// Immutable string type.
    #[cfg(feature = "alloc")]
    String,
    /// Cons-cell linked list type.
    #[cfg(feature = "alloc")]
    List,
    /// Immutable vector type.
    #[cfg(feature = "alloc")]
    Vector,
    /// Immutable map type.
    #[cfg(feature = "alloc")]
    Map,
    /// Immutable set type.
    #[cfg(feature = "alloc")]
    Set,
    /// Mutable binary buffer type.
    #[cfg(feature = "alloc")]
    Binary,
    /// Compiled function type.
    #[cfg(feature = "alloc")]
    Function,
    /// Native function type (first-class reference to a native function).
    NativeFunction,
    /// Var type (mutable binding with metadata).
    #[cfg(feature = "alloc")]
    Var,
}

impl Kind {
    /// Returns the type name for display in error messages.
    #[inline]
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Bool => "boolean",
            Self::Integer => "integer",
            Self::Float => "float",
            #[cfg(feature = "alloc")]
            Self::Ratio => "ratio",
            Self::Symbol => "symbol",
            Self::Keyword => "keyword",
            #[cfg(feature = "alloc")]
            Self::String => "string",
            #[cfg(feature = "alloc")]
            Self::List => "list",
            #[cfg(feature = "alloc")]
            Self::Vector => "vector",
            #[cfg(feature = "alloc")]
            Self::Map => "map",
            #[cfg(feature = "alloc")]
            Self::Set => "set",
            #[cfg(feature = "alloc")]
            Self::Binary => "binary",
            #[cfg(feature = "alloc")]
            Self::Function => "function",
            Self::NativeFunction => "native-function",
            #[cfg(feature = "alloc")]
            Self::Var => "var",
        }
    }

    /// Const-compatible equality check.
    ///
    /// Used by [`TypeExpectation`] for const matching.
    ///
    /// [`TypeExpectation`]: crate::error_context::TypeExpectation
    #[inline]
    #[must_use]
    pub const fn eq_const(self, other: Self) -> bool {
        // Compare discriminants directly since Kind variants have no data
        // (except for cfg-gated ones which are still unit variants)
        matches!(
            (self, other),
            (Self::Nil, Self::Nil)
                | (Self::Bool, Self::Bool)
                | (Self::Integer, Self::Integer)
                | (Self::Float, Self::Float)
                | (Self::Symbol, Self::Symbol)
                | (Self::Keyword, Self::Keyword)
                | (Self::NativeFunction, Self::NativeFunction)
        ) || {
            #[cfg(feature = "alloc")]
            {
                matches!(
                    (self, other),
                    (Self::Ratio, Self::Ratio)
                        | (Self::String, Self::String)
                        | (Self::List, Self::List)
                        | (Self::Vector, Self::Vector)
                        | (Self::Map, Self::Map)
                        | (Self::Set, Self::Set)
                        | (Self::Binary, Self::Binary)
                        | (Self::Function, Self::Function)
                        | (Self::Var, Self::Var)
                )
            }
            #[cfg(not(feature = "alloc"))]
            {
                false
            }
        }
    }

    /// Returns true if this is a numeric type (integer, float, or ratio).
    #[inline]
    #[must_use]
    pub const fn is_numeric(self) -> bool {
        match self {
            Self::Integer | Self::Float => true,
            #[cfg(feature = "alloc")]
            Self::Ratio => true,
            Self::Nil | Self::Bool | Self::Symbol | Self::Keyword | Self::NativeFunction => false,
            #[cfg(feature = "alloc")]
            Self::String
            | Self::List
            | Self::Vector
            | Self::Map
            | Self::Set
            | Self::Binary
            | Self::Function
            | Self::Var => false,
        }
    }

    /// Returns true if this is an integer or float (not ratio).
    ///
    /// Used for operations like modulo that don't support ratios.
    #[inline]
    #[must_use]
    pub const fn is_integer_or_float(self) -> bool {
        matches!(self, Self::Integer | Self::Float)
    }

    /// Returns true if this is a sequence type (list, vector, string, map, or set).
    ///
    /// Maps are sequences of `[key, value]` pairs, following Clojure semantics.
    /// Sets are sequences of their elements.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_sequence(self) -> bool {
        matches!(
            self,
            Self::List | Self::Vector | Self::String | Self::Map | Self::Set
        )
    }

    /// Returns true if this is a callable type (function or native function).
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_callable(self) -> bool {
        matches!(self, Self::Function | Self::NativeFunction)
    }
}

impl core::fmt::Display for Kind {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name())
    }
}

/// Core value representation for Lonala.
///
/// Values can be stack-allocated primitives or heap-allocated types.
/// The heap types (String, Integer, Ratio, and future List, Vector, Map) use
/// reference counting or boxing, which is why `Value` is `Clone` but not `Copy`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Value {
    /// Nothing - the absence of a value.
    Nil,
    /// Boolean truth value.
    Bool(bool),
    /// Arbitrary-precision integer (hybrid small/big representation).
    #[cfg(feature = "alloc")]
    Integer(Integer),
    /// 64-bit signed integer (without `alloc` feature).
    #[cfg(not(feature = "alloc"))]
    Integer(i64),
    /// 64-bit floating point number.
    Float(f64),
    /// Exact rational number (requires `alloc` feature).
    #[cfg(feature = "alloc")]
    Ratio(Ratio),
    /// Interned symbol with optional metadata (identifier).
    #[cfg(feature = "alloc")]
    Symbol(Symbol),
    /// Interned symbol (identifier) - no metadata support without alloc.
    #[cfg(not(feature = "alloc"))]
    Symbol(symbol::Id),
    /// Interned keyword (self-evaluating, commonly used as map keys).
    Keyword(symbol::Id),
    /// Immutable string (requires `alloc` feature).
    #[cfg(feature = "alloc")]
    String(HeapStr),
    /// Cons-cell linked list (requires `alloc` feature).
    #[cfg(feature = "alloc")]
    List(List),
    /// Immutable vector (requires `alloc` feature).
    #[cfg(feature = "alloc")]
    Vector(Vector),
    /// Immutable map (requires `alloc` feature).
    #[cfg(feature = "alloc")]
    Map(Map),
    /// Immutable set (requires `alloc` feature).
    #[cfg(feature = "alloc")]
    Set(Set),
    /// Mutable binary buffer (requires `alloc` feature).
    #[cfg(feature = "alloc")]
    Binary(Binary),
    /// Compiled function (requires `alloc` feature).
    #[cfg(feature = "alloc")]
    Function(Function),
    /// Native function reference (symbol ID for lookup).
    ///
    /// First-class representation of a native function, enabling `+` and `-`
    /// to be passed to higher-order functions like `map` and `reduce`.
    NativeFunction(symbol::Id),
    /// Mutable binding with metadata (requires `alloc` feature).
    ///
    /// Vars are the building blocks of namespaces, providing mutable storage
    /// for values with associated metadata like docstrings and source locations.
    #[cfg(feature = "alloc")]
    Var(Var),
}

impl PartialEq for Value {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&Self::Nil, &Self::Nil) => true,
            (&Self::Bool(ref left), &Self::Bool(ref right)) => left == right,
            (&Self::Integer(ref left), &Self::Integer(ref right)) => left == right,
            (&Self::Float(ref left), &Self::Float(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Ratio(ref left), &Self::Ratio(ref right)) => left == right,
            // Symbol equality compares by ID only (metadata ignored by Symbol's PartialEq)
            (&Self::Symbol(ref left), &Self::Symbol(ref right)) => left == right,
            (&Self::Keyword(ref left), &Self::Keyword(ref right)) => left == right,
            (&Self::NativeFunction(ref left), &Self::NativeFunction(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::String(ref left), &Self::String(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::List(ref left), &Self::List(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Vector(ref left), &Self::Vector(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Map(ref left), &Self::Map(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Set(ref left), &Self::Set(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Binary(ref left), &Self::Binary(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Function(ref left), &Self::Function(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Var(ref left), &Self::Var(ref right)) => left == right,
            _ => false,
        }
    }
}

impl Hash for Value {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the discriminant first
        core::mem::discriminant(self).hash(state);

        // Then hash the value
        match *self {
            Self::Nil => {}
            Self::Bool(value) => value.hash(state),
            Self::Integer(ref value) => value.hash(state),
            Self::Float(value) => {
                // Use to_bits for consistent float hashing
                value.to_bits().hash(state);
            }
            #[cfg(feature = "alloc")]
            Self::Ratio(ref value) => value.hash(state),
            #[cfg(feature = "alloc")]
            Self::Symbol(ref sym) => sym.hash(state),
            #[cfg(not(feature = "alloc"))]
            Self::Symbol(id) => id.hash(state),
            Self::Keyword(id) => id.hash(state),
            Self::NativeFunction(id) => id.hash(state),
            #[cfg(feature = "alloc")]
            Self::String(ref string) => string.hash(state),
            #[cfg(feature = "alloc")]
            Self::List(ref list) => list.hash(state),
            #[cfg(feature = "alloc")]
            Self::Vector(ref vector) => vector.hash(state),
            #[cfg(feature = "alloc")]
            Self::Map(ref map) => map.hash(state),
            #[cfg(feature = "alloc")]
            Self::Set(ref set) => set.hash(state),
            #[cfg(feature = "alloc")]
            Self::Binary(ref binary) => binary.hash(state),
            #[cfg(feature = "alloc")]
            Self::Function(ref func) => func.hash(state),
            #[cfg(feature = "alloc")]
            Self::Var(ref var) => var.hash(state),
        }
    }
}
