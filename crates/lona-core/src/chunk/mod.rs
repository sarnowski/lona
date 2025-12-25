// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Bytecode chunk and constant pool structures.
//!
//! A `Chunk` represents a compiled function body or top-level expression.
//! It contains the bytecode instructions, a constant pool, and metadata
//! for debugging and execution.
//!
//! See `docs/architecture/register-based-vm.md` (from the repository root) for design rationale.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Display};

use crate::span::Span;
use crate::symbol;

mod disassemble;

#[cfg(test)]
mod tests;

/// Describes how to capture a single upvalue at closure creation time.
///
/// When a closure is instantiated, the VM reads values from the parent
/// scope according to these descriptors and copies them into the new
/// closure's upvalue array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpvalueSource {
    /// Capture from a local variable register in the immediately enclosing scope.
    /// The `u8` is the register index in the parent frame.
    Local(u8),
    /// Capture from an upvalue in the immediately enclosing scope.
    /// The `u8` is the upvalue index in the parent's upvalue array.
    /// Used for nested closures capturing from grandparent+ scopes.
    ParentUpvalue(u8),
}

/// Error when the constant pool exceeds its maximum size.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConstantPoolFullError {
    /// Source location where the error occurred.
    pub span: Span,
}

impl Display for ConstantPoolFullError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "too many constants in chunk (maximum 65535) at {}",
            self.span
        )
    }
}

/// Compile-time representation of a single function arity body.
///
/// Each body represents one arity variant of a (potentially multi-arity)
/// function. Contains the compiled bytecode, fixed parameter count,
/// whether this arity accepts rest arguments, and upvalue sources for closures.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FunctionBodyData {
    /// Compiled bytecode for this arity.
    pub chunk: Box<Chunk>,
    /// Number of fixed parameters.
    pub arity: u8,
    /// Whether this arity accepts rest arguments.
    pub has_rest: bool,
    /// Describes how to capture each upvalue when creating a closure.
    /// Empty for functions that don't close over any variables.
    pub upvalue_sources: Vec<UpvalueSource>,
}

impl FunctionBodyData {
    /// Creates a new function body data.
    #[inline]
    #[must_use]
    pub const fn new(
        chunk: Box<Chunk>,
        arity: u8,
        has_rest: bool,
        upvalue_sources: Vec<UpvalueSource>,
    ) -> Self {
        Self {
            chunk,
            arity,
            has_rest,
            upvalue_sources,
        }
    }
}

/// A constant value stored in a chunk's constant pool.
///
/// Constants are referenced by index from `LoadK` instructions and
/// from the high bits of RK operands in arithmetic instructions.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Constant {
    /// The nil value.
    Nil,
    /// A boolean value.
    Bool(bool),
    /// A 64-bit signed integer.
    Integer(i64),
    /// A 64-bit floating-point number.
    Float(f64),
    /// A string value.
    String(String),
    /// An interned symbol identifier.
    Symbol(symbol::Id),
    /// An interned keyword identifier.
    Keyword(symbol::Id),
    /// A list of constants (for quoted lists).
    List(Vec<Self>),
    /// A vector of constants (for quoted vectors).
    Vector(Vec<Self>),
    /// A map of key-value pairs (for quoted maps and metadata).
    ///
    /// Each pair is (key, value). Keys and values can be any constant type.
    Map(Vec<(Self, Self)>),
    /// A compiled function.
    ///
    /// Contains all arity bodies for this function. Single-arity functions
    /// have exactly one body. Multi-arity functions have multiple bodies,
    /// each with different parameter counts.
    Function {
        /// All arity bodies for this function.
        bodies: Vec<FunctionBodyData>,
        /// Optional function name for debugging and error messages.
        name: Option<String>,
    },
}

impl fmt::Display for Constant {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Nil => write!(f, "nil"),
            Self::Bool(val) => write!(f, "{val}"),
            Self::Integer(num) => write!(f, "{num}"),
            Self::Float(num) => write!(f, "{num}"),
            Self::String(ref text) => write!(f, "\"{text}\""),
            Self::Symbol(id) => write!(f, "sym#{}", id.as_u32()),
            Self::Keyword(id) => write!(f, "kw#{}", id.as_u32()),
            Self::List(ref elements) => {
                write!(f, "(")?;
                for (idx, elem) in elements.iter().enumerate() {
                    if idx > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, ")")
            }
            Self::Vector(ref elements) => {
                write!(f, "[")?;
                for (idx, elem) in elements.iter().enumerate() {
                    if idx > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "]")
            }
            Self::Map(ref pairs) => {
                write!(f, "{{")?;
                for (idx, &(ref key, ref val)) in pairs.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{key} {val}")?;
                }
                write!(f, "}}")
            }
            Self::Function {
                ref bodies,
                ref name,
            } => {
                // Format: #<fn/1,2+> or #<fn name/1,2+>
                // The + indicates the last body (if variadic) has rest params
                if bodies.is_empty() {
                    match *name {
                        Some(ref func_name) => write!(f, "#<fn {func_name}/?>"),
                        None => write!(f, "#<fn/?>"),
                    }
                } else if let &[ref body] = bodies.as_slice() {
                    // Single arity - simpler format
                    let rest_indicator = if body.has_rest { "+" } else { "" };
                    match *name {
                        Some(ref func_name) => {
                            write!(f, "#<fn {func_name}/{}{rest_indicator}>", body.arity)
                        }
                        None => write!(f, "#<fn/{}{rest_indicator}>", body.arity),
                    }
                } else {
                    // Multi-arity - list all arities
                    match *name {
                        Some(ref func_name) => write!(f, "#<fn {func_name}/")?,
                        None => write!(f, "#<fn/")?,
                    }
                    for (idx, body) in bodies.iter().enumerate() {
                        if idx > 0 {
                            write!(f, ",")?;
                        }
                        write!(f, "{}", body.arity)?;
                        if body.has_rest {
                            write!(f, "+")?;
                        }
                    }
                    write!(f, ">")
                }
            }
        }
    }
}

/// A compiled bytecode chunk.
///
/// Represents a function body or top-level expression. Contains all the
/// information needed for the VM to execute the code.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Chunk {
    /// Bytecode instructions.
    code: Vec<u32>,
    /// Constant pool.
    constants: Vec<Constant>,
    /// Maximum registers used by this chunk.
    max_registers: u8,
    /// Number of fixed parameters (0 for top-level code).
    arity: u8,
    /// Whether this chunk uses rest parameters.
    has_rest: bool,
    /// Source spans for each instruction (parallel to `code`).
    spans: Vec<Span>,
    /// Function name for debugging (empty for anonymous/top-level).
    name: String,
}
