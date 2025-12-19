// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Function value type.

use core::fmt::{self, Display};
use core::hash::{Hash, Hasher};

/// A compiled function value.
///
/// Functions are first-class values in Lonala. Each function stores its
/// compiled bytecode chunk directly (via an `Arc` for cheap cloning), the
/// number of expected parameters, and an optional name for debugging.
///
/// Note: In Phase 3.3, closures are not supported - functions cannot capture
/// variables from enclosing scopes.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Function {
    /// The compiled bytecode chunk for this function.
    /// Uses Arc for cheap cloning when passing functions around.
    chunk: alloc::sync::Arc<crate::chunk::Chunk>,
    /// Number of fixed parameters this function expects.
    arity: u8,
    /// Whether this function accepts rest arguments.
    has_rest: bool,
    /// Optional function name for debugging and error messages.
    name: Option<alloc::string::String>,
}

impl Function {
    /// Creates a new function value from a chunk.
    #[inline]
    #[must_use]
    pub const fn new(
        chunk: alloc::sync::Arc<crate::chunk::Chunk>,
        arity: u8,
        has_rest: bool,
        name: Option<alloc::string::String>,
    ) -> Self {
        Self {
            chunk,
            arity,
            has_rest,
            name,
        }
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

    /// Returns the number of fixed parameters this function expects.
    #[inline]
    #[must_use]
    pub const fn arity(&self) -> u8 {
        self.arity
    }

    /// Returns whether this function accepts rest arguments.
    #[inline]
    #[must_use]
    pub const fn has_rest(&self) -> bool {
        self.has_rest
    }

    /// Returns the function name, if any.
    #[inline]
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl PartialEq for Function {
    /// Two functions are equal if they have the same chunk (by Arc pointer equality).
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        alloc::sync::Arc::ptr_eq(&self.chunk, &other.chunk)
    }
}

impl Eq for Function {}

impl Hash for Function {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash by pointer address for consistency with PartialEq
        alloc::sync::Arc::as_ptr(&self.chunk).hash(state);
    }
}

impl Display for Function {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rest_indicator = if self.has_rest { "+" } else { "" };
        match self.name {
            Some(ref func_name) => {
                write!(f, "#<function {func_name}/{}{rest_indicator}>", self.arity)
            }
            None => write!(f, "#<function/{}{rest_indicator}>", self.arity),
        }
    }
}
