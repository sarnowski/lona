// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Core value representation for the Lonala language.
//!
//! Values are the fundamental data units in Lonala. This module provides
//! all value types including primitives (nil, bool, integer, float, ratio)
//! and heap-allocated types (string, symbol, list, vector, map, function).

use core::hash::{Hash, Hasher};

#[cfg(feature = "alloc")]
use crate::integer::Integer;
#[cfg(feature = "alloc")]
use crate::list::List;
#[cfg(feature = "alloc")]
use crate::map::Map;
#[cfg(feature = "alloc")]
use crate::ratio::Ratio;
#[cfg(feature = "alloc")]
use crate::string::HeapStr;
use crate::symbol;
#[cfg(feature = "alloc")]
use crate::vector::Vector;

#[cfg(feature = "alloc")]
use crate::symbol::Interner;

mod conversions;
mod display;
#[cfg(feature = "alloc")]
mod function;

#[cfg(test)]
mod tests;

#[cfg(feature = "alloc")]
pub use display::Displayable;
#[cfg(feature = "alloc")]
pub use function::{Function, FunctionBody};

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
    /// Compiled function type.
    #[cfg(feature = "alloc")]
    Function,
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
            #[cfg(feature = "alloc")]
            Self::String => "string",
            #[cfg(feature = "alloc")]
            Self::List => "list",
            #[cfg(feature = "alloc")]
            Self::Vector => "vector",
            #[cfg(feature = "alloc")]
            Self::Map => "map",
            #[cfg(feature = "alloc")]
            Self::Function => "function",
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
                        | (Self::Function, Self::Function)
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
            Self::Nil | Self::Bool | Self::Symbol => false,
            #[cfg(feature = "alloc")]
            Self::String | Self::List | Self::Vector | Self::Map | Self::Function => false,
        }
    }

    /// Returns true if this is a sequence type (list, vector, string, or map).
    ///
    /// Maps are sequences of `[key, value]` pairs, following Clojure semantics.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_sequence(self) -> bool {
        matches!(self, Self::List | Self::Vector | Self::String | Self::Map)
    }

    /// Returns true if this is a callable type (function).
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_callable(self) -> bool {
        matches!(self, Self::Function)
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
    /// Interned symbol (identifier or keyword).
    Symbol(symbol::Id),
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
    /// Compiled function (requires `alloc` feature).
    #[cfg(feature = "alloc")]
    Function(Function),
}

impl Value {
    /// Returns the type of this value.
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> Kind {
        match *self {
            Self::Nil => Kind::Nil,
            Self::Bool(_) => Kind::Bool,
            Self::Integer(_) => Kind::Integer,
            Self::Float(_) => Kind::Float,
            #[cfg(feature = "alloc")]
            Self::Ratio(_) => Kind::Ratio,
            Self::Symbol(_) => Kind::Symbol,
            #[cfg(feature = "alloc")]
            Self::String(_) => Kind::String,
            #[cfg(feature = "alloc")]
            Self::List(_) => Kind::List,
            #[cfg(feature = "alloc")]
            Self::Vector(_) => Kind::Vector,
            #[cfg(feature = "alloc")]
            Self::Map(_) => Kind::Map,
            #[cfg(feature = "alloc")]
            Self::Function(_) => Kind::Function,
        }
    }

    /// Returns `true` if this value is `Nil`.
    #[inline]
    #[must_use]
    pub const fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }

    /// Returns `true` if this value is truthy.
    ///
    /// In Lonala, only `nil` and `false` are falsy; everything else is truthy.
    #[inline]
    #[must_use]
    pub const fn is_truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Bool(false))
    }

    /// Returns the contained boolean if this is a `Bool` variant.
    #[inline]
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Bool(value) => Some(value),
            Self::Nil | Self::Integer(_) | Self::Float(_) | Self::Symbol(_) => None,
            #[cfg(feature = "alloc")]
            Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Function(_) => None,
        }
    }

    /// Returns a reference to the contained integer if this is an `Integer` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_integer(&self) -> Option<&Integer> {
        match *self {
            Self::Integer(ref value) => Some(value),
            Self::Nil
            | Self::Bool(_)
            | Self::Float(_)
            | Self::Symbol(_)
            | Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Function(_) => None,
        }
    }

    /// Returns the contained integer if this is an `Integer` variant.
    #[cfg(not(feature = "alloc"))]
    #[inline]
    #[must_use]
    pub const fn as_integer(&self) -> Option<i64> {
        match *self {
            Self::Integer(value) => Some(value),
            Self::Nil | Self::Bool(_) | Self::Float(_) | Self::Symbol(_) => None,
        }
    }

    /// Returns `true` if this value is an `Integer`.
    #[inline]
    #[must_use]
    pub const fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }

    /// Returns the contained float if this is a `Float` variant.
    #[inline]
    #[must_use]
    pub const fn as_float(&self) -> Option<f64> {
        match *self {
            Self::Float(value) => Some(value),
            Self::Nil | Self::Bool(_) | Self::Integer(_) | Self::Symbol(_) => None,
            #[cfg(feature = "alloc")]
            Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Function(_) => None,
        }
    }

    /// Returns a reference to the contained ratio if this is a `Ratio` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_ratio(&self) -> Option<&Ratio> {
        match *self {
            Self::Ratio(ref value) => Some(value),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Symbol(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Function(_) => None,
        }
    }

    /// Returns `true` if this value is a `Ratio`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_ratio(&self) -> bool {
        matches!(self, Self::Ratio(_))
    }

    /// Returns the contained symbol ID if this is a `Symbol` variant.
    #[inline]
    #[must_use]
    pub const fn as_symbol(&self) -> Option<symbol::Id> {
        match *self {
            Self::Symbol(id) => Some(id),
            Self::Nil | Self::Bool(_) | Self::Integer(_) | Self::Float(_) => None,
            #[cfg(feature = "alloc")]
            Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Function(_) => None,
        }
    }

    /// Returns `true` if this value is a `String`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Returns a reference to the contained string if this is a `String` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_string(&self) -> Option<&HeapStr> {
        match *self {
            Self::String(ref string) => Some(string),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Ratio(_)
            | Self::Symbol(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Function(_) => None,
        }
    }

    /// Returns `true` if this value is a `List`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    /// Returns a reference to the contained list if this is a `List` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_list(&self) -> Option<&List> {
        match *self {
            Self::List(ref list) => Some(list),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Ratio(_)
            | Self::Symbol(_)
            | Self::String(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Function(_) => None,
        }
    }

    /// Returns `true` if this value is a `Vector`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_vector(&self) -> bool {
        matches!(self, Self::Vector(_))
    }

    /// Returns a reference to the contained vector if this is a `Vector` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_vector(&self) -> Option<&Vector> {
        match *self {
            Self::Vector(ref vector) => Some(vector),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Ratio(_)
            | Self::Symbol(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Map(_)
            | Self::Function(_) => None,
        }
    }

    /// Returns `true` if this value is a `Map`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    /// Returns a reference to the contained map if this is a `Map` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_map(&self) -> Option<&Map> {
        match *self {
            Self::Map(ref map) => Some(map),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Ratio(_)
            | Self::Symbol(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Function(_) => None,
        }
    }

    /// Returns `true` if this value is a `Function`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_function(&self) -> bool {
        matches!(self, Self::Function(_))
    }

    /// Returns a reference to the contained function if this is a `Function` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_function(&self) -> Option<&Function> {
        match *self {
            Self::Function(ref func) => Some(func),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Ratio(_)
            | Self::Symbol(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_) => None,
        }
    }

    /// Creates a wrapper for displaying this value with symbol resolution.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn display<'interner>(
        &'interner self,
        interner: &'interner Interner,
    ) -> Displayable<'interner> {
        Displayable::new(self, interner)
    }
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
            (&Self::Symbol(ref left), &Self::Symbol(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::String(ref left), &Self::String(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::List(ref left), &Self::List(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Vector(ref left), &Self::Vector(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Map(ref left), &Self::Map(ref right)) => left == right,
            #[cfg(feature = "alloc")]
            (&Self::Function(ref left), &Self::Function(ref right)) => left == right,
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
            Self::Symbol(id) => id.hash(state),
            #[cfg(feature = "alloc")]
            Self::String(ref string) => string.hash(state),
            #[cfg(feature = "alloc")]
            Self::List(ref list) => list.hash(state),
            #[cfg(feature = "alloc")]
            Self::Vector(ref vector) => vector.hash(state),
            #[cfg(feature = "alloc")]
            Self::Map(ref map) => map.hash(state),
            #[cfg(feature = "alloc")]
            Self::Function(ref func) => func.hash(state),
        }
    }
}
