// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Runtime error types for the virtual machine.
//!
//! This module follows the standardized error pattern: a `Kind` enum for
//! error classification and an `Error` struct combining kind with source
//! location. Formatting is handled by `lonala-human`, not here.
//!
//! # Design Principles
//!
//! - **Structured data, not strings**: Errors carry typed data; formatting happens in `lonala-human`
//! - **Symbol IDs, not names**: Store `symbol::Id`, resolve to names during formatting
//! - **Typed context**: Use `value::Kind` and `TypeExpectation` instead of `&'static str`
//! - **Source locations always**: Every error includes `SourceLocation` (aliased from `source::Location`)

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::source::{self, Location as SourceLocation};
use lona_core::symbol;
use lona_core::value;

use super::natives::NativeError;

/// Kinds of errors that can occur during VM execution.
///
/// Each variant captures the specific nature of the error with all context
/// needed for formatting. NO human-readable strings should be stored here
/// except for operation names (which are stable identifiers like "+", "if").
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    /// Invalid opcode byte encountered.
    InvalidOpcode {
        /// The invalid opcode byte value.
        byte: u8,
        /// Program counter where the error occurred.
        pc: usize,
    },

    /// Attempted to access an undefined global variable.
    UndefinedGlobal {
        /// The symbol ID of the undefined global.
        symbol: symbol::Id,
        /// A similar symbol that might be what the user meant (for "did you mean?").
        suggestion: Option<symbol::Id>,
    },

    /// Attempted to call an undefined function.
    UndefinedFunction {
        /// The symbol ID of the undefined function.
        symbol: symbol::Id,
        /// A similar symbol that might be what the user meant (for "did you mean?").
        suggestion: Option<symbol::Id>,
    },

    /// Type mismatch in operation.
    TypeError {
        /// The operation being performed ("+", "-", "if", etc.).
        operation: &'static str,
        /// The type(s) that were expected.
        expected: TypeExpectation,
        /// The type that was actually found.
        got: value::Kind,
        /// Which operand had the wrong type (0-indexed), if applicable.
        operand: Option<u8>,
    },

    /// Division by zero.
    DivisionByZero,

    /// Call stack overflow.
    StackOverflow {
        /// Maximum allowed stack depth.
        max_depth: usize,
    },

    /// Attempted to call a non-callable value.
    NotCallable {
        /// The type of value that was called.
        got: value::Kind,
    },

    /// Invalid constant pool index.
    InvalidConstant {
        /// The invalid index.
        index: u16,
    },

    /// Invalid register index.
    InvalidRegister {
        /// The invalid register index.
        index: u8,
    },

    /// Error occurred in a native function.
    Native {
        /// The native function error.
        error: NativeError,
    },

    /// Function was called with wrong number of arguments.
    ArityMismatch {
        /// The function/symbol being called, if known.
        callable: Option<symbol::Id>,
        /// Expected number of arguments.
        expected: ArityExpectation,
        /// Actual number of arguments provided.
        got: u8,
    },

    /// Attempted to access an upvalue index that doesn't exist.
    InvalidUpvalue {
        /// The requested upvalue index.
        index: u8,
        /// How many upvalues the closure actually has.
        available: usize,
    },

    /// A feature is not yet implemented.
    ///
    /// Used for placeholder code that will be completed in later phases.
    NotImplemented {
        /// Description of the unimplemented feature.
        feature: &'static str,
    },

    /// No matching clause in a `case` expression.
    ///
    /// Occurs when a `case` expression has no matching pattern and no
    /// `:else` default clause is provided.
    NoMatchingCase {
        /// The type of the value that failed to match.
        value_type: value::Kind,
    },
}

impl Kind {
    /// Returns the variant name for error identification.
    ///
    /// Used as a stable error identifier in formatted output (e.g., `error[TypeError]`).
    #[inline]
    #[must_use]
    pub const fn variant_name(&self) -> &'static str {
        match *self {
            Self::InvalidOpcode { .. } => "InvalidOpcode",
            Self::UndefinedGlobal { .. } => "UndefinedGlobal",
            Self::UndefinedFunction { .. } => "UndefinedFunction",
            Self::TypeError { .. } => "TypeError",
            Self::DivisionByZero => "DivisionByZero",
            Self::StackOverflow { .. } => "StackOverflow",
            Self::NotCallable { .. } => "NotCallable",
            Self::InvalidConstant { .. } => "InvalidConstant",
            Self::InvalidRegister { .. } => "InvalidRegister",
            Self::Native { .. } => "NativeError",
            Self::ArityMismatch { .. } => "ArityMismatch",
            Self::InvalidUpvalue { .. } => "InvalidUpvalue",
            Self::NotImplemented { .. } => "NotImplemented",
            Self::NoMatchingCase { .. } => "NoMatchingCase",
        }
    }
}

/// A runtime error with its source location.
///
/// Combines an error kind with its full source location, enabling helpful error
/// messages that can point to the exact position in the correct source file.
///
/// # Note
///
/// This type does NOT implement `Display`. All formatting is centralized in
/// the `lonala-human` crate to ensure consistent error presentation across
/// REPL and future LSP implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Error {
    /// The kind of error.
    pub kind: Kind,
    /// Full source location (source ID + byte span).
    pub location: SourceLocation,
}

impl Error {
    /// Creates a new error with the given kind and source location.
    #[inline]
    #[must_use]
    pub const fn new(kind: Kind, location: SourceLocation) -> Self {
        Self { kind, location }
    }

    /// Returns the span within the source where the error occurred.
    #[inline]
    #[must_use]
    pub const fn span(&self) -> lona_core::span::Span {
        self.location.span
    }

    /// Returns the source ID where the error occurred.
    #[inline]
    #[must_use]
    pub const fn source_id(&self) -> source::Id {
        self.location.source
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use lona_core::span::Span;

    use super::*;

    /// Helper to create a test source location.
    fn test_location(start: usize, end: usize) -> SourceLocation {
        SourceLocation::new(source::Id::new(0_u32), Span::new(start, end))
    }

    #[test]
    fn error_new_and_accessors() {
        let location = test_location(5_usize, 10_usize);
        let error = Error::new(
            Kind::UndefinedGlobal {
                symbol: symbol::Id::new(42_u32),
                suggestion: None,
            },
            location,
        );

        assert_eq!(error.span(), Span::new(5_usize, 10_usize));
        assert_eq!(error.source_id(), source::Id::new(0_u32));
    }

    #[test]
    fn kind_variant_name_invalid_opcode() {
        let kind = Kind::InvalidOpcode { byte: 0xFF, pc: 10 };
        assert_eq!(kind.variant_name(), "InvalidOpcode");
    }

    #[test]
    fn kind_variant_name_undefined_global() {
        let kind = Kind::UndefinedGlobal {
            symbol: symbol::Id::new(1_u32),
            suggestion: None,
        };
        assert_eq!(kind.variant_name(), "UndefinedGlobal");
    }

    #[test]
    fn kind_variant_name_type_error() {
        let kind = Kind::TypeError {
            operation: "+",
            expected: TypeExpectation::Numeric,
            got: value::Kind::Bool,
            operand: Some(0_u8),
        };
        assert_eq!(kind.variant_name(), "TypeError");
    }

    #[test]
    fn kind_variant_name_division_by_zero() {
        assert_eq!(Kind::DivisionByZero.variant_name(), "DivisionByZero");
    }

    #[test]
    fn kind_variant_name_stack_overflow() {
        let kind = Kind::StackOverflow { max_depth: 256 };
        assert_eq!(kind.variant_name(), "StackOverflow");
    }

    #[test]
    fn kind_variant_name_not_callable() {
        let kind = Kind::NotCallable {
            got: value::Kind::Integer,
        };
        assert_eq!(kind.variant_name(), "NotCallable");
    }

    #[test]
    fn kind_variant_name_arity_mismatch() {
        let kind = Kind::ArityMismatch {
            callable: None,
            expected: ArityExpectation::Exact(2_u8),
            got: 3_u8,
        };
        assert_eq!(kind.variant_name(), "ArityMismatch");
    }

    #[test]
    fn type_error_with_operand() {
        let kind = Kind::TypeError {
            operation: "-",
            expected: TypeExpectation::Numeric,
            got: value::Kind::Symbol,
            operand: Some(1_u8),
        };
        match kind {
            Kind::TypeError {
                operation,
                expected,
                got,
                operand,
            } => {
                assert_eq!(operation, "-");
                assert_eq!(expected, TypeExpectation::Numeric);
                assert_eq!(got, value::Kind::Symbol);
                assert_eq!(operand, Some(1_u8));
            }
            _ => panic!("Expected TypeError"),
        }
    }

    #[test]
    fn undefined_global_with_suggestion() {
        let kind = Kind::UndefinedGlobal {
            symbol: symbol::Id::new(10_u32),
            suggestion: Some(symbol::Id::new(11_u32)),
        };
        match kind {
            Kind::UndefinedGlobal { symbol, suggestion } => {
                assert_eq!(symbol.as_u32(), 10_u32);
                assert_eq!(suggestion.map(|id| id.as_u32()), Some(11_u32));
            }
            _ => panic!("Expected UndefinedGlobal"),
        }
    }
}
