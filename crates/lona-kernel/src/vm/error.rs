// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Runtime error types for the virtual machine.

use core::fmt::{self, Display};

use lona_core::symbol;
use lonala_parser::Span;

/// Runtime errors that can occur during VM execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Invalid opcode byte encountered.
    InvalidOpcode {
        /// The invalid opcode byte.
        byte: u8,
        /// Program counter where the error occurred.
        pc: usize,
        /// Source location of the instruction.
        span: Span,
    },

    /// Attempted to access an undefined global variable.
    UndefinedGlobal {
        /// The symbol ID of the undefined global.
        symbol: symbol::Id,
        /// Source location of the access.
        span: Span,
    },

    /// Type mismatch in operation.
    TypeError {
        /// The expected type.
        expected: &'static str,
        /// The actual type encountered.
        got: &'static str,
        /// Source location of the operation.
        span: Span,
    },

    /// Division by zero.
    DivisionByZero {
        /// Source location of the division.
        span: Span,
    },

    /// Call stack overflow.
    StackOverflow {
        /// Maximum allowed stack depth.
        max_depth: usize,
        /// Source location of the call that caused overflow.
        span: Span,
    },

    /// Attempted to call a non-callable value.
    NotCallable {
        /// Source location of the call.
        span: Span,
    },

    /// Invalid constant pool index.
    InvalidConstant {
        /// The invalid index.
        index: u16,
        /// Source location of the instruction.
        span: Span,
    },

    /// Invalid register index.
    InvalidRegister {
        /// The invalid register index.
        index: u8,
        /// Source location of the instruction.
        span: Span,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOpcode { byte, pc, span } => {
                write!(
                    f,
                    "invalid opcode 0x{byte:02X} at pc={pc} (span {}-{})",
                    span.start, span.end
                )
            }
            Self::UndefinedGlobal { symbol, span } => {
                write!(
                    f,
                    "undefined global variable (symbol #{}) at span {}-{}",
                    symbol.as_u32(),
                    span.start,
                    span.end
                )
            }
            Self::TypeError {
                expected,
                got,
                span,
            } => {
                write!(
                    f,
                    "type error: expected {expected}, got {got} at span {}-{}",
                    span.start, span.end
                )
            }
            Self::DivisionByZero { span } => {
                write!(f, "division by zero at span {}-{}", span.start, span.end)
            }
            Self::StackOverflow { max_depth, span } => {
                write!(
                    f,
                    "stack overflow (max depth {max_depth}) at span {}-{}",
                    span.start, span.end
                )
            }
            Self::NotCallable { span } => {
                write!(
                    f,
                    "attempted to call non-callable value at span {}-{}",
                    span.start, span.end
                )
            }
            Self::InvalidConstant { index, span } => {
                write!(
                    f,
                    "invalid constant pool index {index} at span {}-{}",
                    span.start, span.end
                )
            }
            Self::InvalidRegister { index, span } => {
                write!(
                    f,
                    "invalid register index {index} at span {}-{}",
                    span.start, span.end
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_invalid_opcode() {
        let err = Error::InvalidOpcode {
            byte: 0xFF,
            pc: 10,
            span: Span::new(0_usize, 5_usize),
        };
        let msg = alloc::format!("{err}");
        assert!(msg.contains("invalid opcode"));
        assert!(msg.contains("0xFF"));
        assert!(msg.contains("pc=10"));
    }

    #[test]
    fn display_type_error() {
        let err = Error::TypeError {
            expected: "integer",
            got: "boolean",
            span: Span::new(0_usize, 5_usize),
        };
        let msg = alloc::format!("{err}");
        assert!(msg.contains("type error"));
        assert!(msg.contains("integer"));
        assert!(msg.contains("boolean"));
    }

    #[test]
    fn display_division_by_zero() {
        let err = Error::DivisionByZero {
            span: Span::new(0_usize, 5_usize),
        };
        let msg = alloc::format!("{err}");
        assert!(msg.contains("division by zero"));
    }
}
