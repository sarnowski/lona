// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Error types for bytecode compilation.
//!
//! This module defines errors that can occur during compilation of Lonala
//! source code to bytecode.

use core::fmt;

/// Errors that can occur during compilation.
///
/// These errors represent limits exceeded during bytecode generation,
/// such as too many constants or registers in a single chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
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
    /// -32768 to +32767 instructions.
    JumpTooLarge,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyConstants => {
                write!(f, "too many constants in chunk (maximum 65535)")
            }
            Self::TooManyRegisters => {
                write!(f, "too many registers needed (maximum 255)")
            }
            Self::JumpTooLarge => {
                write!(f, "jump offset too large (maximum +/- 32767)")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

    #[test]
    fn error_display() {
        assert_eq!(
            format!("{}", Error::TooManyConstants),
            "too many constants in chunk (maximum 65535)"
        );
        assert_eq!(
            format!("{}", Error::TooManyRegisters),
            "too many registers needed (maximum 255)"
        );
        assert_eq!(
            format!("{}", Error::JumpTooLarge),
            "jump offset too large (maximum +/- 32767)"
        );
    }

    #[test]
    fn error_equality() {
        assert_eq!(Error::TooManyConstants, Error::TooManyConstants);
        assert_ne!(Error::TooManyConstants, Error::TooManyRegisters);
        assert_ne!(Error::TooManyRegisters, Error::JumpTooLarge);
    }

    #[test]
    fn error_clone() {
        let error = Error::TooManyConstants;
        let cloned = error;
        assert_eq!(error, cloned);
    }
}
