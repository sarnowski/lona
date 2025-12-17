// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Error types for bytecode compilation.
//!
//! This module defines errors that can occur during compilation of Lonala
//! source code to bytecode.

use core::fmt;

use lona_core::chunk::ConstantPoolFullError;
use lona_core::span::Span;

/// Errors that can occur during compilation.
///
/// These errors represent limits exceeded during bytecode generation,
/// semantic errors, or features not yet implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Too many constants in a single chunk (> 65535).
    ///
    /// The constant pool uses 16-bit indices, limiting each chunk
    /// to 65536 constants maximum.
    TooManyConstants {
        /// Source location where the error occurred.
        span: Span,
    },

    /// Too many registers needed (> 255).
    ///
    /// Register indices use 8-bit fields, limiting each function
    /// to 256 registers maximum.
    TooManyRegisters {
        /// Source location where the error occurred.
        span: Span,
    },

    /// Jump offset too large to encode.
    ///
    /// Jump offsets use signed 16-bit fields, limiting jumps to
    /// -32768 to +32767 instructions. This error is effectively unreachable
    /// in practice since it would require a single branch body to generate
    /// over 32,000 bytecode instructions, which exceeds realistic program sizes.
    JumpTooLarge {
        /// Source location where the error occurred.
        span: Span,
    },

    /// Empty list cannot be compiled as a function call.
    ///
    /// A list like `()` has no function to call.
    EmptyCall {
        /// Source location of the empty list.
        span: Span,
    },

    /// Feature not yet implemented.
    ///
    /// Indicates that a language feature is planned but not available
    /// in this compiler phase.
    NotImplemented {
        /// Description of the unimplemented feature.
        feature: &'static str,
        /// Source location where the feature was used.
        span: Span,
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
        /// Source location where the error occurred.
        span: Span,
    },
}

impl Error {
    /// Returns the source span where this error occurred.
    #[inline]
    #[must_use]
    pub const fn span(&self) -> Span {
        match *self {
            Self::TooManyConstants { span }
            | Self::TooManyRegisters { span }
            | Self::JumpTooLarge { span }
            | Self::EmptyCall { span }
            | Self::NotImplemented { span, .. }
            | Self::InvalidSpecialForm { span, .. } => span,
        }
    }
}

impl From<ConstantPoolFullError> for Error {
    #[inline]
    fn from(err: ConstantPoolFullError) -> Self {
        Self::TooManyConstants { span: err.span }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TooManyConstants { span } => {
                write!(f, "too many constants in chunk (maximum 65535) at {span}")
            }
            Self::TooManyRegisters { span } => {
                write!(f, "too many registers needed (maximum 255) at {span}")
            }
            Self::JumpTooLarge { span } => {
                write!(f, "jump offset too large (maximum +/- 32767) at {span}")
            }
            Self::EmptyCall { span } => {
                write!(f, "empty list cannot be called as function at {span}")
            }
            Self::NotImplemented { feature, span } => {
                write!(f, "not implemented: {feature} at {span}")
            }
            Self::InvalidSpecialForm {
                form,
                message,
                span,
            } => {
                write!(f, "invalid '{form}' form: {message} at {span}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

    fn test_span() -> Span {
        Span::new(10_usize, 20_usize)
    }

    #[test]
    fn error_display() {
        assert_eq!(
            format!("{}", Error::TooManyConstants { span: test_span() }),
            "too many constants in chunk (maximum 65535) at 10..20"
        );
        assert_eq!(
            format!("{}", Error::TooManyRegisters { span: test_span() }),
            "too many registers needed (maximum 255) at 10..20"
        );
        assert_eq!(
            format!("{}", Error::JumpTooLarge { span: test_span() }),
            "jump offset too large (maximum +/- 32767) at 10..20"
        );
        assert_eq!(
            format!("{}", Error::EmptyCall { span: test_span() }),
            "empty list cannot be called as function at 10..20"
        );
        assert_eq!(
            format!(
                "{}",
                Error::NotImplemented {
                    feature: "closures",
                    span: test_span()
                }
            ),
            "not implemented: closures at 10..20"
        );
        assert_eq!(
            format!(
                "{}",
                Error::InvalidSpecialForm {
                    form: "if",
                    message: "expected (if test then) or (if test then else)",
                    span: test_span()
                }
            ),
            "invalid 'if' form: expected (if test then) or (if test then else) at 10..20"
        );
    }

    #[test]
    fn error_span_accessor() {
        let span = Span::new(5_usize, 15_usize);
        assert_eq!(Error::TooManyConstants { span }.span(), span);
        assert_eq!(Error::TooManyRegisters { span }.span(), span);
        assert_eq!(Error::JumpTooLarge { span }.span(), span);
        assert_eq!(Error::EmptyCall { span }.span(), span);
        assert_eq!(
            Error::NotImplemented {
                feature: "test",
                span
            }
            .span(),
            span
        );
        assert_eq!(
            Error::InvalidSpecialForm {
                form: "if",
                message: "test",
                span
            }
            .span(),
            span
        );
    }

    #[test]
    fn error_equality() {
        let span = test_span();
        assert_eq!(
            Error::TooManyConstants { span },
            Error::TooManyConstants { span }
        );
        assert_ne!(
            Error::TooManyConstants { span },
            Error::TooManyRegisters { span }
        );
        assert_ne!(
            Error::TooManyRegisters { span },
            Error::JumpTooLarge { span }
        );
    }

    #[test]
    fn error_clone() {
        let error = Error::TooManyConstants { span: test_span() };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }
}
