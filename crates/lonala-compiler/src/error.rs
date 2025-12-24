// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Error types for bytecode compilation.
//!
//! This module provides error types and location information for reporting
//! issues encountered during compilation of Lonala source code to bytecode.
//!
//! # Design Principles
//!
//! - **Structured data, not strings**: Errors carry typed data; formatting happens in `lonala-human`
//! - **Source locations always**: Every error includes `source::Location`
//! - **No Display on Error**: Formatting is centralized in `lonala-human` crate

extern crate alloc;

use alloc::string::String;

// Re-export source types from lona-core for consistency.
pub use lona_core::source::{self, Id as SourceId, Location as SourceLocation};
pub use lona_core::span::Span;

/// Kinds of errors that can occur during compilation.
///
/// Each variant captures the specific nature of the error with all context
/// needed for formatting. NO human-readable strings should be stored here
/// except for dynamic messages from macro expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    // ========== Resource limit errors ==========
    /// Too many constants in a single chunk (> 65535).
    ///
    /// The constant pool uses 16-bit indices, limiting each chunk
    /// to 65536 constants maximum.
    TooManyConstants,

    /// Too many registers needed (> 255).
    ///
    /// Register indices use 8-bit fields, limiting each function
    /// to 256 registers maximum.
    TooManyRegisters,

    /// Jump offset too large to encode.
    ///
    /// Jump offsets use signed 16-bit fields, limiting jumps to
    /// -32768 to +32767 instructions. This error is effectively unreachable
    /// in practice since it would require a single branch body to generate
    /// over 32,000 bytecode instructions, which exceeds realistic program sizes.
    JumpTooLarge,

    // ========== Semantic errors ==========
    /// Empty list cannot be compiled as a function call.
    ///
    /// A list like `()` has no function to call.
    EmptyCall,

    /// Feature not yet implemented.
    ///
    /// Indicates that a language feature is planned but not available
    /// in this compiler phase.
    NotImplemented {
        /// Description of the unimplemented feature.
        feature: &'static str,
    },

    /// Invalid special form syntax.
    ///
    /// Indicates that a special form (like `def`, `if`, `let`, etc.)
    /// was used with incorrect syntax.
    InvalidSpecialForm {
        /// Name of the special form.
        form: &'static str,
        /// Description of what went wrong.
        message: &'static str,
    },

    // ========== Macro expansion errors ==========
    /// Invalid macro expansion result.
    ///
    /// The macro returned a value that cannot be converted back to AST
    /// for further compilation (e.g., a function value or ratio).
    InvalidMacroResult {
        /// Description of why the result is invalid.
        message: String,
    },

    /// Macro expansion failed at runtime.
    ///
    /// The macro transformer function threw an error during execution.
    MacroExpansionFailed {
        /// The error message from the macro.
        message: String,
    },

    /// Macro expansion exceeded maximum depth.
    ///
    /// This typically indicates infinite macro recursion where a macro
    /// expands to code that calls itself (directly or indirectly).
    MacroExpansionDepthExceeded {
        /// The depth at which expansion was stopped.
        depth: usize,
    },

    // ========== Destructuring errors ==========
    /// Invalid destructuring pattern syntax.
    ///
    /// Indicates that a destructuring pattern (like `[a b & rest]`)
    /// has invalid syntax.
    InvalidDestructuringPattern {
        /// Description of what went wrong.
        message: &'static str,
    },

    /// Destructuring pattern nesting exceeded maximum depth.
    ///
    /// This prevents stack overflow from deeply nested patterns.
    /// The limit is generous (1024) to allow legitimate complex patterns
    /// while preventing runaway recursion.
    RecursionDepthExceeded {
        /// The maximum allowed depth.
        max_depth: usize,
    },

    // ========== Internal errors ==========
    /// Internal compiler error.
    ///
    /// Indicates a bug in the compiler - a condition that should never
    /// occur if the compiler logic is correct. This is used for invariant
    /// violations rather than user errors.
    InternalError {
        /// Description of the internal error.
        message: &'static str,
    },
}

impl Kind {
    /// Returns the variant name for error identification.
    ///
    /// Used as a stable error identifier in formatted output (e.g., `error[TooManyConstants]`).
    #[inline]
    #[must_use]
    pub const fn variant_name(&self) -> &'static str {
        match *self {
            Self::TooManyConstants => "TooManyConstants",
            Self::TooManyRegisters => "TooManyRegisters",
            Self::JumpTooLarge => "JumpTooLarge",
            Self::EmptyCall => "EmptyCall",
            Self::NotImplemented { .. } => "NotImplemented",
            Self::InvalidSpecialForm { .. } => "InvalidSpecialForm",
            Self::InvalidMacroResult { .. } => "InvalidMacroResult",
            Self::MacroExpansionFailed { .. } => "MacroExpansionFailed",
            Self::MacroExpansionDepthExceeded { .. } => "MacroExpansionDepthExceeded",
            Self::InvalidDestructuringPattern { .. } => "InvalidDestructuringPattern",
            Self::RecursionDepthExceeded { .. } => "RecursionDepthExceeded",
            Self::InternalError { .. } => "InternalError",
        }
    }
}

// NOTE: No Display impl on Kind. All error formatting is centralized in the
// `lonala-human` crate via the Diagnostic trait, ensuring consistent error
// presentation across REPL and future LSP implementations.

/// An error encountered during compilation.
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
    pub const fn span(&self) -> Span {
        self.location.span
    }

    /// Returns the source ID where the error occurred.
    #[inline]
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.location.source
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::string::String;

    use super::*;

    /// Helper to create a test source location.
    fn test_location(start: usize, end: usize) -> SourceLocation {
        SourceLocation::new(SourceId::new(0_u32), Span::new(start, end))
    }

    // ==================== Kind variant_name() Tests ====================

    #[test]
    fn kind_variant_name() {
        assert_eq!(Kind::TooManyConstants.variant_name(), "TooManyConstants");
        assert_eq!(Kind::TooManyRegisters.variant_name(), "TooManyRegisters");
        assert_eq!(Kind::JumpTooLarge.variant_name(), "JumpTooLarge");
        assert_eq!(Kind::EmptyCall.variant_name(), "EmptyCall");
        assert_eq!(
            Kind::NotImplemented { feature: "test" }.variant_name(),
            "NotImplemented"
        );
        assert_eq!(
            Kind::InvalidSpecialForm {
                form: "if",
                message: "test"
            }
            .variant_name(),
            "InvalidSpecialForm"
        );
        assert_eq!(
            Kind::InvalidMacroResult {
                message: String::from("test")
            }
            .variant_name(),
            "InvalidMacroResult"
        );
        assert_eq!(
            Kind::MacroExpansionFailed {
                message: String::from("test")
            }
            .variant_name(),
            "MacroExpansionFailed"
        );
        assert_eq!(
            Kind::MacroExpansionDepthExceeded { depth: 256_usize }.variant_name(),
            "MacroExpansionDepthExceeded"
        );
        assert_eq!(
            Kind::InvalidDestructuringPattern { message: "test" }.variant_name(),
            "InvalidDestructuringPattern"
        );
        assert_eq!(
            Kind::RecursionDepthExceeded {
                max_depth: 1024_usize
            }
            .variant_name(),
            "RecursionDepthExceeded"
        );
    }

    // ==================== Error Tests ====================

    #[test]
    fn error_new_and_accessors() {
        let location = test_location(5_usize, 15_usize);
        let error = Error::new(Kind::EmptyCall, location);
        assert_eq!(error.kind, Kind::EmptyCall);
        assert_eq!(error.span(), Span::new(5_usize, 15_usize));
        assert_eq!(error.source_id(), SourceId::new(0_u32));
    }

    #[test]
    fn error_location_field() {
        let source_id = SourceId::new(42_u32);
        let span = Span::new(10_usize, 20_usize);
        let location = SourceLocation::new(source_id, span);
        let error = Error::new(Kind::TooManyConstants, location);
        assert_eq!(error.location.source, source_id);
        assert_eq!(error.location.span, span);
    }

    #[test]
    fn error_span_accessor() {
        let location = test_location(5_usize, 15_usize);
        assert_eq!(
            Error::new(Kind::TooManyConstants, location).span(),
            location.span
        );
        assert_eq!(
            Error::new(Kind::TooManyRegisters, location).span(),
            location.span
        );
        assert_eq!(
            Error::new(Kind::JumpTooLarge, location).span(),
            location.span
        );
        assert_eq!(Error::new(Kind::EmptyCall, location).span(), location.span);
        assert_eq!(
            Error::new(Kind::NotImplemented { feature: "test" }, location).span(),
            location.span
        );
        assert_eq!(
            Error::new(
                Kind::InvalidSpecialForm {
                    form: "if",
                    message: "test"
                },
                location
            )
            .span(),
            location.span
        );
        assert_eq!(
            Error::new(
                Kind::InvalidMacroResult {
                    message: String::from("test")
                },
                location
            )
            .span(),
            location.span
        );
        assert_eq!(
            Error::new(
                Kind::MacroExpansionFailed {
                    message: String::from("test")
                },
                location
            )
            .span(),
            location.span
        );
        assert_eq!(
            Error::new(
                Kind::MacroExpansionDepthExceeded { depth: 256_usize },
                location
            )
            .span(),
            location.span
        );
        assert_eq!(
            Error::new(
                Kind::InvalidDestructuringPattern { message: "test" },
                location
            )
            .span(),
            location.span
        );
        assert_eq!(
            Error::new(
                Kind::RecursionDepthExceeded {
                    max_depth: 1024_usize
                },
                location
            )
            .span(),
            location.span
        );
    }

    #[test]
    fn error_equality() {
        let location = test_location(10_usize, 20_usize);
        assert_eq!(
            Error::new(Kind::TooManyConstants, location),
            Error::new(Kind::TooManyConstants, location)
        );
        assert_ne!(
            Error::new(Kind::TooManyConstants, location),
            Error::new(Kind::TooManyRegisters, location)
        );
        assert_ne!(
            Error::new(Kind::TooManyRegisters, location),
            Error::new(Kind::JumpTooLarge, location)
        );
    }

    #[test]
    fn error_clone() {
        let error = Error::new(Kind::TooManyConstants, test_location(10_usize, 20_usize));
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }
}
