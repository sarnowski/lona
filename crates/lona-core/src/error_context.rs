// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Shared types for structured error messages.
//!
//! This module provides types that capture structured context for error
//! messages, enabling the `lonala-human` crate to generate helpful,
//! Rust-style error output.
//!
//! # Types
//!
//! - [`TypeExpectation`] - What type(s) were expected in an operation
//! - [`ArityExpectation`] - How many arguments were expected

use crate::value;

/// Expected type(s) for an operation.
///
/// Used to provide structured "expected X, got Y" error messages.
/// The formatting layer (`lonala-human`) uses this to generate
/// helpful type error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypeExpectation {
    /// A single specific type is required.
    Single(value::Kind),
    /// One of two specific types is required.
    Either(value::Kind, value::Kind),
    /// Any numeric type (integer, float, or ratio).
    Numeric,
    /// Integer or float (not ratio), for operations like modulo.
    IntegerOrFloat,
    /// Any sequence type (list, vector, or string).
    Sequence,
    /// Any callable type (function or native function).
    Callable,
    /// Boolean type specifically required.
    Boolean,
    /// Symbol type specifically required.
    Symbol,
    /// Any type that supports ordering (numeric types or string).
    Comparable,
    /// Any collection type that supports `conj` (list, vector, set, or nil).
    #[cfg(feature = "alloc")]
    Collection,
    /// Any type that supports metadata (symbol, list, vector, map, set).
    #[cfg(feature = "alloc")]
    MetaSupporting,
}

impl TypeExpectation {
    /// Returns a human-readable description of this expectation.
    ///
    /// Used by the formatting layer for generating error messages.
    #[inline]
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match *self {
            Self::Single(kind) => kind.name(),
            Self::Either(_, _) => "one of two types",
            Self::Numeric => "numeric type",
            Self::IntegerOrFloat => "integer or float",
            Self::Sequence => "sequence type",
            Self::Callable => "callable",
            Self::Boolean => "boolean",
            Self::Symbol => "symbol",
            Self::Comparable => "comparable type (number or string)",
            #[cfg(feature = "alloc")]
            Self::Collection => "collection (list, vector, set, or nil)",
            #[cfg(feature = "alloc")]
            Self::MetaSupporting => "metadata-supporting type (symbol, list, vector, map, or set)",
        }
    }

    /// Checks if the given value kind matches this expectation.
    #[inline]
    #[must_use]
    pub const fn matches(&self, kind: value::Kind) -> bool {
        match *self {
            Self::Single(expected) => expected.eq_const(kind),
            Self::Either(first, second) => first.eq_const(kind) || second.eq_const(kind),
            Self::Numeric => kind.is_numeric(),
            Self::IntegerOrFloat => kind.is_integer_or_float(),
            #[cfg(feature = "alloc")]
            Self::Sequence => kind.is_sequence(),
            #[cfg(not(feature = "alloc"))]
            Self::Sequence => false,
            #[cfg(feature = "alloc")]
            Self::Callable => kind.is_callable(),
            #[cfg(not(feature = "alloc"))]
            Self::Callable => false,
            Self::Boolean => kind.eq_const(value::Kind::Bool),
            Self::Symbol => kind.eq_const(value::Kind::Symbol),
            #[cfg(feature = "alloc")]
            Self::Comparable => kind.is_numeric() || kind.eq_const(value::Kind::String),
            #[cfg(not(feature = "alloc"))]
            Self::Comparable => kind.is_numeric(),
            #[cfg(feature = "alloc")]
            Self::Collection => matches!(
                kind,
                value::Kind::List | value::Kind::Vector | value::Kind::Set | value::Kind::Nil
            ),
            #[cfg(feature = "alloc")]
            Self::MetaSupporting => matches!(
                kind,
                value::Kind::Symbol
                    | value::Kind::List
                    | value::Kind::Vector
                    | value::Kind::Map
                    | value::Kind::Set
            ),
        }
    }
}

/// Expected number of arguments for a function call.
///
/// Used to provide structured arity mismatch error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArityExpectation {
    /// Exactly N arguments required.
    Exact(u8),
    /// At least N arguments required (variadic).
    AtLeast(u8),
    /// Between min and max arguments (inclusive).
    Range {
        /// Minimum number of arguments.
        min: u8,
        /// Maximum number of arguments.
        max: u8,
    },
}

impl ArityExpectation {
    /// Checks if the given argument count satisfies this expectation.
    #[inline]
    #[must_use]
    pub const fn matches(&self, count: u8) -> bool {
        match *self {
            Self::Exact(expected) => count == expected,
            Self::AtLeast(min) => count >= min,
            Self::Range { min, max } => count >= min && count <= max,
        }
    }

    /// Returns a human-readable description of this expectation.
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub fn description(&self) -> alloc::string::String {
        use alloc::format;
        match *self {
            Self::Exact(num) => format!("{num}"),
            Self::AtLeast(min) => format!("at least {min}"),
            Self::Range { min, max } => format!("{min} to {max}"),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;

    #[test]
    fn type_expectation_single_matches() {
        let exp = TypeExpectation::Single(value::Kind::Integer);
        assert!(exp.matches(value::Kind::Integer));
        assert!(!exp.matches(value::Kind::Float));
        assert!(!exp.matches(value::Kind::Bool));
    }

    #[test]
    fn type_expectation_numeric_matches() {
        let exp = TypeExpectation::Numeric;
        assert!(exp.matches(value::Kind::Integer));
        assert!(exp.matches(value::Kind::Float));
        #[cfg(feature = "alloc")]
        assert!(exp.matches(value::Kind::Ratio));
        assert!(!exp.matches(value::Kind::Bool));
        assert!(!exp.matches(value::Kind::Nil));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn type_expectation_sequence_matches() {
        let exp = TypeExpectation::Sequence;
        assert!(exp.matches(value::Kind::List));
        assert!(exp.matches(value::Kind::Vector));
        assert!(exp.matches(value::Kind::String));
        // Maps are sequences of [key value] pairs (Clojure semantics)
        assert!(exp.matches(value::Kind::Map));
        assert!(!exp.matches(value::Kind::Integer));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn type_expectation_callable_matches() {
        let exp = TypeExpectation::Callable;
        assert!(exp.matches(value::Kind::Function));
        assert!(!exp.matches(value::Kind::Integer));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn type_expectation_comparable_matches() {
        let exp = TypeExpectation::Comparable;
        // Numeric types are comparable
        assert!(exp.matches(value::Kind::Integer));
        assert!(exp.matches(value::Kind::Float));
        assert!(exp.matches(value::Kind::Ratio));
        // String is comparable
        assert!(exp.matches(value::Kind::String));
        // Other types are not comparable
        assert!(!exp.matches(value::Kind::Bool));
        assert!(!exp.matches(value::Kind::Nil));
        assert!(!exp.matches(value::Kind::List));
        assert!(!exp.matches(value::Kind::Vector));
    }

    #[test]
    fn type_expectation_description() {
        assert_eq!(
            TypeExpectation::Single(value::Kind::Integer).description(),
            "integer"
        );
        assert_eq!(TypeExpectation::Numeric.description(), "numeric type");
        assert_eq!(TypeExpectation::Sequence.description(), "sequence type");
        assert_eq!(TypeExpectation::Callable.description(), "callable");
        assert_eq!(
            TypeExpectation::Comparable.description(),
            "comparable type (number or string)"
        );
    }

    #[test]
    fn arity_expectation_exact_matches() {
        let exp = ArityExpectation::Exact(2_u8);
        assert!(!exp.matches(1_u8));
        assert!(exp.matches(2_u8));
        assert!(!exp.matches(3_u8));
    }

    #[test]
    fn arity_expectation_at_least_matches() {
        let exp = ArityExpectation::AtLeast(2_u8);
        assert!(!exp.matches(1_u8));
        assert!(exp.matches(2_u8));
        assert!(exp.matches(3_u8));
        assert!(exp.matches(100_u8));
    }

    #[test]
    fn arity_expectation_range_matches() {
        let exp = ArityExpectation::Range {
            min: 2_u8,
            max: 4_u8,
        };
        assert!(!exp.matches(1_u8));
        assert!(exp.matches(2_u8));
        assert!(exp.matches(3_u8));
        assert!(exp.matches(4_u8));
        assert!(!exp.matches(5_u8));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn arity_expectation_description() {
        assert_eq!(ArityExpectation::Exact(2_u8).description(), "2");
        assert_eq!(ArityExpectation::AtLeast(1_u8).description(), "at least 1");
        assert_eq!(
            ArityExpectation::Range {
                min: 2_u8,
                max: 4_u8
            }
            .description(),
            "2 to 4"
        );
    }
}
