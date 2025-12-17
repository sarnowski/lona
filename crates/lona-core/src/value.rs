// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Core value representation for the Lonala language.
//!
//! Values are the fundamental data units in Lonala. This module provides
//! the initial set of primitive types. Future phases will extend this
//! with heap-allocated types like strings, lists, and maps.

use core::fmt::{self, Display};
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

/// A compiled function value.
///
/// Functions are first-class values in Lonala. Each function stores its
/// compiled bytecode chunk directly (via an `Arc` for cheap cloning), the
/// number of expected parameters, and an optional name for debugging.
///
/// Note: In Phase 3.3, closures are not supported - functions cannot capture
/// variables from enclosing scopes.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Function {
    /// The compiled bytecode chunk for this function.
    /// Uses Arc for cheap cloning when passing functions around.
    chunk: alloc::sync::Arc<crate::chunk::Chunk>,
    /// Number of parameters this function expects.
    arity: u8,
    /// Optional function name for debugging and error messages.
    name: Option<alloc::string::String>,
}

#[cfg(feature = "alloc")]
impl Function {
    /// Creates a new function value from a chunk.
    #[inline]
    #[must_use]
    pub const fn new(
        chunk: alloc::sync::Arc<crate::chunk::Chunk>,
        arity: u8,
        name: Option<alloc::string::String>,
    ) -> Self {
        Self { chunk, arity, name }
    }

    /// Returns a reference to the function's bytecode chunk.
    #[inline]
    #[must_use]
    pub fn chunk(&self) -> &crate::chunk::Chunk {
        &self.chunk
    }

    /// Returns the Arc containing the function's chunk (for cloning).
    #[inline]
    #[must_use]
    pub const fn chunk_arc(&self) -> &alloc::sync::Arc<crate::chunk::Chunk> {
        &self.chunk
    }

    /// Returns the number of parameters this function expects.
    #[inline]
    #[must_use]
    pub const fn arity(&self) -> u8 {
        self.arity
    }

    /// Returns the function name, if any.
    #[inline]
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

#[cfg(feature = "alloc")]
impl PartialEq for Function {
    /// Two functions are equal if they have the same chunk (by Arc pointer equality).
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        alloc::sync::Arc::ptr_eq(&self.chunk, &other.chunk)
    }
}

#[cfg(feature = "alloc")]
impl Eq for Function {}

#[cfg(feature = "alloc")]
impl Hash for Function {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash by pointer address for consistency with PartialEq
        alloc::sync::Arc::as_ptr(&self.chunk).hash(state);
    }
}

#[cfg(feature = "alloc")]
impl Display for Function {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(ref func_name) => write!(f, "#<function {func_name}>"),
            None => write!(f, "#<function>"),
        }
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
        Displayable {
            value: self,
            interner,
        }
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

impl Display for Value {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Nil => write!(f, "nil"),
            Self::Bool(true) => write!(f, "true"),
            Self::Bool(false) => write!(f, "false"),
            Self::Integer(ref value) => write!(f, "{value}"),
            Self::Float(value) => format_float(value, f),
            #[cfg(feature = "alloc")]
            Self::Ratio(ref value) => write!(f, "{value}"),
            Self::Symbol(id) => write!(f, "#<symbol:{}>", id.as_u32()),
            #[cfg(feature = "alloc")]
            Self::String(ref string) => write!(f, "\"{string}\""),
            #[cfg(feature = "alloc")]
            Self::List(ref list) => write!(f, "{list}"),
            #[cfg(feature = "alloc")]
            Self::Vector(ref vector) => write!(f, "{vector}"),
            #[cfg(feature = "alloc")]
            Self::Map(ref map) => write!(f, "{map}"),
            #[cfg(feature = "alloc")]
            Self::Function(ref func) => write!(f, "{func}"),
        }
    }
}

/// Formats a float in Lonala syntax.
///
/// Ensures that whole numbers still show as floats (e.g., "1.0" not "1").
fn format_float(value: f64, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    if value.is_nan() {
        write!(formatter, "##NaN")
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            write!(formatter, "##Inf")
        } else {
            write!(formatter, "##-Inf")
        }
    } else {
        // Check if the number is a whole number within i64 range
        // A float is whole if converting to i64 and back gives the same value
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::float_cmp,
            reason = "checking if float is representable as i64 - exact equality is intentional"
        )]
        let is_whole = value >= (i64::MIN as f64)
            && value <= (i64::MAX as f64)
            && (value as i64 as f64) == value;

        if is_whole {
            // Whole number - ensure decimal point is shown
            write!(formatter, "{value:.1}")
        } else {
            // Has fractional part or is very large - use default formatting
            write!(formatter, "{value}")
        }
    }
}

/// A wrapper for displaying a [`Value`] with symbol name resolution.
///
/// Created via [`Value::display`].
#[cfg(feature = "alloc")]
pub struct Displayable<'interner> {
    value: &'interner Value,
    interner: &'interner Interner,
}

#[cfg(feature = "alloc")]
impl Display for Displayable<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.value {
            Value::Symbol(id) => write!(f, "{}", self.interner.resolve(id)),
            Value::Nil => write!(f, "nil"),
            Value::Bool(true) => write!(f, "true"),
            Value::Bool(false) => write!(f, "false"),
            Value::Integer(ref value) => write!(f, "{value}"),
            Value::Float(value) => format_float(value, f),
            Value::Ratio(ref value) => write!(f, "{value}"),
            Value::String(ref string) => write!(f, "{string}"),
            Value::List(ref list) => write!(f, "{}", list.display(self.interner)),
            Value::Vector(ref vector) => write!(f, "{}", vector.display(self.interner)),
            Value::Map(ref map) => write!(f, "{}", map.display(self.interner)),
            Value::Function(ref func) => write!(f, "{func}"),
        }
    }
}

// Convenience conversions

impl From<bool> for Value {
    #[inline]
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[cfg(feature = "alloc")]
impl From<i64> for Value {
    #[inline]
    fn from(value: i64) -> Self {
        Self::Integer(Integer::from_i64(value))
    }
}

#[cfg(not(feature = "alloc"))]
impl From<i64> for Value {
    #[inline]
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

#[cfg(feature = "alloc")]
impl From<i32> for Value {
    #[inline]
    fn from(value: i32) -> Self {
        Self::Integer(Integer::from(value))
    }
}

#[cfg(not(feature = "alloc"))]
impl From<i32> for Value {
    #[inline]
    fn from(value: i32) -> Self {
        Self::Integer(i64::from(value))
    }
}

#[cfg(feature = "alloc")]
impl From<Integer> for Value {
    #[inline]
    fn from(value: Integer) -> Self {
        Self::Integer(value)
    }
}

#[cfg(feature = "alloc")]
impl From<Ratio> for Value {
    #[inline]
    fn from(value: Ratio) -> Self {
        Self::Ratio(value)
    }
}

impl From<f64> for Value {
    #[inline]
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<symbol::Id> for Value {
    #[inline]
    fn from(id: symbol::Id) -> Self {
        Self::Symbol(id)
    }
}

#[cfg(feature = "alloc")]
impl From<HeapStr> for Value {
    #[inline]
    fn from(string: HeapStr) -> Self {
        Self::String(string)
    }
}

#[cfg(feature = "alloc")]
impl From<&str> for Value {
    #[inline]
    fn from(text: &str) -> Self {
        Self::String(HeapStr::new(text))
    }
}

#[cfg(feature = "alloc")]
impl From<List> for Value {
    #[inline]
    fn from(list: List) -> Self {
        Self::List(list)
    }
}

#[cfg(feature = "alloc")]
impl From<Vector> for Value {
    #[inline]
    fn from(vector: Vector) -> Self {
        Self::Vector(vector)
    }
}

#[cfg(feature = "alloc")]
impl From<Map> for Value {
    #[inline]
    fn from(map: Map) -> Self {
        Self::Map(map)
    }
}

#[cfg(feature = "alloc")]
impl From<Function> for Value {
    #[inline]
    fn from(func: Function) -> Self {
        Self::Function(func)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "alloc")]
    use crate::symbol::Interner;
    #[cfg(feature = "alloc")]
    use alloc::string::ToString;

    /// Helper to create an integer value.
    #[cfg(feature = "alloc")]
    fn int(value: i64) -> Value {
        Value::Integer(Integer::from_i64(value))
    }

    /// Helper to create an integer value (non-alloc).
    #[cfg(not(feature = "alloc"))]
    fn int(value: i64) -> Value {
        Value::Integer(value)
    }

    #[test]
    fn nil_equality() {
        assert_eq!(Value::Nil, Value::Nil);
    }

    #[test]
    fn bool_equality() {
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Bool(false), Value::Bool(false));
        assert_ne!(Value::Bool(true), Value::Bool(false));
    }

    #[test]
    fn integer_equality() {
        assert_eq!(int(42), int(42));
        assert_eq!(int(-1), int(-1));
        assert_ne!(int(1), int(2));
    }

    #[test]
    fn float_equality() {
        assert_eq!(Value::Float(3.14), Value::Float(3.14));
        assert_eq!(Value::Float(-0.5), Value::Float(-0.5));
        assert_ne!(Value::Float(1.0), Value::Float(2.0));
    }

    #[test]
    fn float_nan_not_equal_to_itself() {
        // NaN behavior: NaN != NaN
        let nan = Value::Float(f64::NAN);
        assert_ne!(nan, nan);
    }

    #[test]
    fn different_types_not_equal() {
        assert_ne!(Value::Nil, Value::Bool(false));
        assert_ne!(int(0), Value::Float(0.0));
        assert_ne!(Value::Bool(true), int(1));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_nil() {
        assert_eq!(Value::Nil.to_string(), "nil");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_bool() {
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Bool(false).to_string(), "false");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_integer() {
        assert_eq!(int(42).to_string(), "42");
        assert_eq!(int(-17).to_string(), "-17");
        assert_eq!(int(0).to_string(), "0");
        assert_eq!(int(i64::MAX).to_string(), "9223372036854775807");
        assert_eq!(int(i64::MIN).to_string(), "-9223372036854775808");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_float() {
        assert_eq!(Value::Float(3.14).to_string(), "3.14");
        assert_eq!(Value::Float(-0.5).to_string(), "-0.5");
        // Whole numbers show decimal point
        assert_eq!(Value::Float(1.0).to_string(), "1.0");
        assert_eq!(Value::Float(-42.0).to_string(), "-42.0");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_float_special_values() {
        assert_eq!(Value::Float(f64::NAN).to_string(), "##NaN");
        assert_eq!(Value::Float(f64::INFINITY).to_string(), "##Inf");
        assert_eq!(Value::Float(f64::NEG_INFINITY).to_string(), "##-Inf");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_float_scientific() {
        // Very large numbers use scientific notation
        assert_eq!(Value::Float(1e20).to_string(), "100000000000000000000");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_symbol_without_interner() {
        let mut interner = Interner::new();
        let id = interner.intern("foo");
        let value = Value::Symbol(id);
        // Without interner, shows raw ID
        assert_eq!(value.to_string(), "#<symbol:0>");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_symbol_with_interner() {
        let mut interner = Interner::new();
        let id = interner.intern("my-symbol");
        let value = Value::Symbol(id);
        // With interner, shows symbol name
        assert_eq!(value.display(&interner).to_string(), "my-symbol");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_with_interner_passthrough() {
        let interner = Interner::new();

        // Non-symbol values pass through unchanged
        assert_eq!(Value::Nil.display(&interner).to_string(), "nil");
        assert_eq!(Value::Bool(true).display(&interner).to_string(), "true");
        assert_eq!(int(42).display(&interner).to_string(), "42");
        assert_eq!(Value::Float(3.14).display(&interner).to_string(), "3.14");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_list_with_symbols_resolves_names() {
        use crate::list::List;

        let mut interner = Interner::new();
        let plus_id = interner.intern("+");
        let x_id = interner.intern("x");
        let y_id = interner.intern("y");

        // Create list (+ x y)
        let list = List::empty()
            .cons(Value::Symbol(y_id))
            .cons(Value::Symbol(x_id))
            .cons(Value::Symbol(plus_id));

        let value = Value::List(list);

        // With interner, symbols should show their names
        assert_eq!(value.display(&interner).to_string(), "(+ x y)");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_vector_with_symbols_resolves_names() {
        use crate::vector::Vector;

        let mut interner = Interner::new();
        let a_id = interner.intern("a");
        let b_id = interner.intern("b");

        // Create vector [a b]
        let vector = Vector::empty()
            .push(Value::Symbol(a_id))
            .push(Value::Symbol(b_id));

        let value = Value::Vector(vector);

        // With interner, symbols should show their names
        assert_eq!(value.display(&interner).to_string(), "[a b]");
    }

    #[test]
    fn is_nil() {
        assert!(Value::Nil.is_nil());
        assert!(!Value::Bool(false).is_nil());
        assert!(!int(0).is_nil());
    }

    #[test]
    fn is_truthy() {
        // Only nil and false are falsy
        assert!(!Value::Nil.is_truthy());
        assert!(!Value::Bool(false).is_truthy());

        // Everything else is truthy
        assert!(Value::Bool(true).is_truthy());
        assert!(int(0).is_truthy()); // 0 is truthy!
        assert!(int(42).is_truthy());
        assert!(Value::Float(0.0).is_truthy()); // 0.0 is truthy!
        assert!(Value::Float(3.14).is_truthy());
    }

    #[test]
    fn as_bool() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Bool(false).as_bool(), Some(false));
        assert_eq!(Value::Nil.as_bool(), None);
        assert_eq!(int(1).as_bool(), None);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn as_integer() {
        assert_eq!(int(42).as_integer(), Some(&Integer::from_i64(42)));
        assert_eq!(int(-1).as_integer(), Some(&Integer::from_i64(-1)));
        assert_eq!(Value::Nil.as_integer(), None);
        assert_eq!(Value::Float(42.0).as_integer(), None);
    }

    #[test]
    fn as_float() {
        assert_eq!(Value::Float(3.14).as_float(), Some(3.14));
        assert_eq!(Value::Nil.as_float(), None);
        assert_eq!(int(42).as_float(), None);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn as_symbol() {
        let mut interner = Interner::new();
        let id = interner.intern("test");
        assert_eq!(Value::Symbol(id).as_symbol(), Some(id));
        assert_eq!(Value::Nil.as_symbol(), None);
    }

    #[test]
    fn from_bool() {
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(false), Value::Bool(false));
    }

    #[test]
    fn from_i64() {
        assert_eq!(Value::from(42_i64), int(42));
        assert_eq!(Value::from(-1_i64), int(-1));
    }

    #[test]
    fn from_i32() {
        assert_eq!(Value::from(42_i32), int(42));
    }

    #[test]
    fn from_f64() {
        assert_eq!(Value::from(3.14_f64), Value::Float(3.14));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn from_symbol_id() {
        let mut interner = Interner::new();
        let id = interner.intern("test");
        assert_eq!(Value::from(id), Value::Symbol(id));
    }

    #[test]
    fn value_is_clone() {
        let v1 = int(42);
        let v2 = v1.clone();
        assert_eq!(v1, v2);
    }

    // =========================================================================
    // Ratio Tests
    // =========================================================================

    #[cfg(feature = "alloc")]
    #[test]
    fn ratio_equality() {
        let r1 = Value::Ratio(Ratio::from_i64(1, 2));
        let r2 = Value::Ratio(Ratio::from_i64(1, 2));
        let r3 = Value::Ratio(Ratio::from_i64(1, 3));

        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn ratio_equality_normalized() {
        // 2/4 should equal 1/2 after normalization
        let r1 = Value::Ratio(Ratio::from_i64(2, 4));
        let r2 = Value::Ratio(Ratio::from_i64(1, 2));
        assert_eq!(r1, r2);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_ratio() {
        let ratio = Value::Ratio(Ratio::from_i64(1, 3));
        assert_eq!(ratio.to_string(), "1/3");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_ratio_integer() {
        // Ratio that equals an integer displays as integer
        let ratio = Value::Ratio(Ratio::from_i64(4, 2));
        assert_eq!(ratio.to_string(), "2");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn is_ratio() {
        assert!(Value::Ratio(Ratio::from_i64(1, 2)).is_ratio());
        assert!(!Value::Nil.is_ratio());
        assert!(!int(42).is_ratio());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn as_ratio() {
        let ratio = Ratio::from_i64(1, 2);
        let value = Value::Ratio(ratio.clone());
        assert_eq!(value.as_ratio(), Some(&ratio));
        assert_eq!(Value::Nil.as_ratio(), None);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn from_ratio() {
        let ratio = Ratio::from_i64(1, 2);
        let value = Value::from(ratio.clone());
        assert_eq!(value, Value::Ratio(ratio));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn ratio_is_truthy() {
        // All ratios are truthy, including zero
        assert!(Value::Ratio(Ratio::from_i64(0, 1)).is_truthy());
        assert!(Value::Ratio(Ratio::from_i64(1, 2)).is_truthy());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn ratio_not_equal_to_integer() {
        // Even though 2/1 = 2, they are different types
        let ratio = Value::Ratio(Ratio::from_i64(2, 1));
        let integer = int(2);
        assert_ne!(ratio, integer);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_ratio_with_interner() {
        let interner = Interner::new();
        let ratio = Value::Ratio(Ratio::from_i64(1, 3));
        assert_eq!(ratio.display(&interner).to_string(), "1/3");
    }

    // =========================================================================
    // String Tests
    // =========================================================================

    #[cfg(feature = "alloc")]
    #[test]
    fn string_equality() {
        let s1 = Value::String(HeapStr::new("hello"));
        let s2 = Value::String(HeapStr::new("hello"));
        let s3 = Value::String(HeapStr::new("world"));

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_string() {
        let string = Value::String(HeapStr::new("hello world"));
        assert_eq!(string.to_string(), "\"hello world\"");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_string_empty() {
        let string = Value::String(HeapStr::new(""));
        assert_eq!(string.to_string(), "\"\"");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_string_with_interner() {
        let interner = Interner::new();
        let string = Value::String(HeapStr::new("hello"));
        // With interner, string shows without quotes (raw content)
        assert_eq!(string.display(&interner).to_string(), "hello");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn is_string() {
        assert!(Value::String(HeapStr::new("test")).is_string());
        assert!(!Value::Nil.is_string());
        assert!(!Value::Integer(Integer::from_i64(42)).is_string());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn as_string() {
        let string = HeapStr::new("test");
        let value = Value::String(string.clone());
        assert_eq!(value.as_string(), Some(&string));
        assert_eq!(Value::Nil.as_string(), None);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn from_heap_str() {
        let string = HeapStr::new("test");
        let value = Value::from(string.clone());
        assert_eq!(value, Value::String(string));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn from_str_slice() {
        let value = Value::from("hello");
        assert_eq!(value, Value::String(HeapStr::new("hello")));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn string_is_truthy() {
        // All strings are truthy, even empty ones
        assert!(Value::String(HeapStr::new("")).is_truthy());
        assert!(Value::String(HeapStr::new("hello")).is_truthy());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn string_not_equal_to_other_types() {
        let string = Value::String(HeapStr::new("42"));
        assert_ne!(string, Value::Integer(Integer::from_i64(42)));
        assert_ne!(string, Value::Nil);
        assert_ne!(string, Value::Bool(true));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn string_clone_shares_data() {
        let s1 = Value::String(HeapStr::new("hello"));
        let s2 = s1.clone();
        assert_eq!(s1, s2);
        // Both are still valid after clone
        assert_eq!(s1.as_string().unwrap().as_str(), "hello");
        assert_eq!(s2.as_string().unwrap().as_str(), "hello");
    }
}
