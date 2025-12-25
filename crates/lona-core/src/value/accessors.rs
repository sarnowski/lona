// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Accessor methods for Value type inspection.
//!
//! This module provides `is_X()` predicates and `as_X()` extractors for
//! the [`Value`] enum, enabling type checking and value extraction.

#[cfg(feature = "alloc")]
use super::Displayable;
use super::{Kind, Value};
use crate::symbol;
#[cfg(feature = "alloc")]
use crate::symbol::Interner;

#[cfg(feature = "alloc")]
use super::{Function, Symbol, Var};
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
#[cfg(feature = "alloc")]
use crate::vector::Vector;

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
            Self::Keyword(_) => Kind::Keyword,
            #[cfg(feature = "alloc")]
            Self::String(_) => Kind::String,
            #[cfg(feature = "alloc")]
            Self::List(_) => Kind::List,
            #[cfg(feature = "alloc")]
            Self::Vector(_) => Kind::Vector,
            #[cfg(feature = "alloc")]
            Self::Map(_) => Kind::Map,
            #[cfg(feature = "alloc")]
            Self::Set(_) => Kind::Set,
            #[cfg(feature = "alloc")]
            Self::Binary(_) => Kind::Binary,
            #[cfg(feature = "alloc")]
            Self::Function(_) => Kind::Function,
            Self::NativeFunction(_) => Kind::NativeFunction,
            #[cfg(feature = "alloc")]
            Self::Var(_) => Kind::Var,
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
            Self::Nil
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Symbol(_)
            | Self::Keyword(_)
            | Self::NativeFunction(_) => None,
            #[cfg(feature = "alloc")]
            Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
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
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
        }
    }

    /// Returns the contained integer if this is an `Integer` variant.
    #[cfg(not(feature = "alloc"))]
    #[inline]
    #[must_use]
    pub const fn as_integer(&self) -> Option<i64> {
        match *self {
            Self::Integer(value) => Some(value),
            Self::Nil
            | Self::Bool(_)
            | Self::Float(_)
            | Self::Symbol(_)
            | Self::Keyword(_)
            | Self::NativeFunction(_) => None,
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
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Symbol(_)
            | Self::Keyword(_)
            | Self::NativeFunction(_) => None,
            #[cfg(feature = "alloc")]
            Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
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
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
        }
    }

    /// Returns `true` if this value is a `Ratio`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_ratio(&self) -> bool {
        matches!(self, Self::Ratio(_))
    }

    /// Returns the symbol ID if this is a `Symbol` variant.
    ///
    /// This returns only the ID, not the full Symbol with metadata.
    /// Use [`Self::as_symbol_ref`] to get a reference to the full Symbol.
    #[inline]
    #[must_use]
    pub const fn as_symbol(&self) -> Option<symbol::Id> {
        match *self {
            #[cfg(feature = "alloc")]
            Self::Symbol(ref sym) => Some(sym.id()),
            #[cfg(not(feature = "alloc"))]
            Self::Symbol(id) => Some(id),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Keyword(_)
            | Self::NativeFunction(_) => None,
            #[cfg(feature = "alloc")]
            Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
        }
    }

    /// Returns a reference to the Symbol if this is a `Symbol` variant.
    ///
    /// This includes the symbol ID and any attached metadata.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_symbol_ref(&self) -> Option<&Symbol> {
        match *self {
            Self::Symbol(ref sym) => Some(sym),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Ratio(_)
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
        }
    }

    /// Returns `true` if this value is a `Keyword`.
    #[inline]
    #[must_use]
    pub const fn is_keyword(&self) -> bool {
        matches!(self, Self::Keyword(_))
    }

    /// Returns the contained symbol ID if this is a `Keyword` variant.
    #[inline]
    #[must_use]
    pub const fn as_keyword(&self) -> Option<symbol::Id> {
        match *self {
            Self::Keyword(id) => Some(id),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Symbol(_)
            | Self::NativeFunction(_) => None,
            #[cfg(feature = "alloc")]
            Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
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
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
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
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::String(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
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
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
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
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
        }
    }

    /// Returns `true` if this value is a `Set`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_set(&self) -> bool {
        matches!(self, Self::Set(_))
    }

    /// Returns a reference to the contained set if this is a `Set` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_set(&self) -> Option<&Set> {
        match *self {
            Self::Set(ref set) => Some(set),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Ratio(_)
            | Self::Symbol(_)
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
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
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Var(_) => None,
        }
    }

    /// Returns `true` if this value is a `NativeFunction`.
    #[inline]
    #[must_use]
    pub const fn is_native_function(&self) -> bool {
        matches!(self, Self::NativeFunction(_))
    }

    /// Returns the contained symbol ID if this is a `NativeFunction` variant.
    #[inline]
    #[must_use]
    pub const fn as_native_function(&self) -> Option<symbol::Id> {
        match *self {
            Self::NativeFunction(id) => Some(id),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Symbol(_)
            | Self::Keyword(_) => None,
            #[cfg(feature = "alloc")]
            Self::Ratio(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_)
            | Self::Var(_) => None,
        }
    }

    /// Returns `true` if this value is a `Binary`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_binary(&self) -> bool {
        matches!(self, Self::Binary(_))
    }

    /// Returns a reference to the contained binary if this is a `Binary` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_binary(&self) -> Option<&Binary> {
        match *self {
            Self::Binary(ref binary) => Some(binary),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Ratio(_)
            | Self::Symbol(_)
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Function(_)
            | Self::Var(_) => None,
        }
    }

    /// Returns `true` if this value is a `Var`.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn is_var(&self) -> bool {
        matches!(self, Self::Var(_))
    }

    /// Returns a reference to the contained var if this is a `Var` variant.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub const fn as_var(&self) -> Option<&Var> {
        match *self {
            Self::Var(ref var) => Some(var),
            Self::Nil
            | Self::Bool(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Ratio(_)
            | Self::Symbol(_)
            | Self::Keyword(_)
            | Self::NativeFunction(_)
            | Self::String(_)
            | Self::List(_)
            | Self::Vector(_)
            | Self::Map(_)
            | Self::Set(_)
            | Self::Binary(_)
            | Self::Function(_) => None,
        }
    }
}
