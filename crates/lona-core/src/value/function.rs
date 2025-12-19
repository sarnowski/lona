// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Function value type.

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{self, Display};
use core::hash::{Hash, Hasher};

use crate::chunk::Chunk;

/// Runtime representation of a single function arity body.
///
/// Each body represents one arity variant of a (potentially multi-arity)
/// function. Contains the compiled bytecode, fixed parameter count, and
/// whether this arity accepts rest arguments.
#[derive(Debug, Clone)]
pub struct FunctionBody {
    /// Compiled bytecode for this arity.
    chunk: Arc<Chunk>,
    /// Number of fixed parameters.
    arity: u8,
    /// Whether this arity accepts rest arguments.
    has_rest: bool,
}

impl FunctionBody {
    /// Creates a new function body.
    #[inline]
    #[must_use]
    pub const fn new(chunk: Arc<Chunk>, arity: u8, has_rest: bool) -> Self {
        Self {
            chunk,
            arity,
            has_rest,
        }
    }

    /// Returns a reference to the body's bytecode chunk.
    #[inline]
    #[must_use]
    pub fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    /// Returns the Arc containing the body's chunk (for cloning).
    #[inline]
    #[must_use]
    pub const fn chunk_arc(&self) -> &Arc<Chunk> {
        &self.chunk
    }

    /// Returns the number of fixed parameters this body expects.
    #[inline]
    #[must_use]
    pub const fn arity(&self) -> u8 {
        self.arity
    }

    /// Returns whether this body accepts rest arguments.
    #[inline]
    #[must_use]
    pub const fn has_rest(&self) -> bool {
        self.has_rest
    }
}

/// A compiled function value.
///
/// Functions are first-class values in Lonala. Each function stores one or
/// more arity bodies (for multi-arity functions) and an optional name for
/// debugging.
///
/// Note: In Phase 3.3, closures are not supported - functions cannot capture
/// variables from enclosing scopes.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Function {
    /// All arity bodies for this function, sorted by arity for efficient dispatch.
    bodies: Vec<FunctionBody>,
    /// Optional function name for debugging and error messages.
    name: Option<String>,
}

impl Function {
    /// Creates a new function value from bodies.
    #[inline]
    #[must_use]
    pub const fn new(bodies: Vec<FunctionBody>, name: Option<String>) -> Self {
        Self { bodies, name }
    }

    /// Creates a single-arity function (convenience constructor).
    #[inline]
    #[must_use]
    pub fn single_arity(
        chunk: Arc<Chunk>,
        arity: u8,
        has_rest: bool,
        name: Option<String>,
    ) -> Self {
        let body = FunctionBody::new(chunk, arity, has_rest);
        Self {
            bodies: alloc::vec![body],
            name,
        }
    }

    /// Returns all arity bodies for this function.
    #[inline]
    #[must_use]
    pub fn bodies(&self) -> &[FunctionBody] {
        &self.bodies
    }

    /// Returns the function name, if any.
    #[inline]
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the first body (for single-arity functions).
    ///
    /// Returns `None` if the function has no bodies.
    #[inline]
    #[must_use]
    pub fn first_body(&self) -> Option<&FunctionBody> {
        self.bodies.first()
    }

    /// Finds the matching body for the given argument count.
    ///
    /// Dispatch priority:
    /// 1. Exact fixed arity match (no rest args)
    /// 2. Variadic match (`has_rest` and argc >= fixed arity)
    ///
    /// Returns `None` if no arity matches.
    #[inline]
    #[must_use]
    pub fn find_body(&self, argc: usize) -> Option<&FunctionBody> {
        // First try exact fixed match
        self.bodies
            .iter()
            .find(|body| !body.has_rest && usize::from(body.arity) == argc)
            .or_else(|| {
                // Then try variadic match
                self.bodies
                    .iter()
                    .find(|body| body.has_rest && argc >= usize::from(body.arity))
            })
    }
}

impl PartialEq for Function {
    /// Two functions are equal if they have the same first body's chunk (by Arc pointer equality).
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Compare by first body's chunk pointer (if both have bodies)
        match (self.bodies.first(), other.bodies.first()) {
            (Some(self_body), Some(other_body)) => Arc::ptr_eq(&self_body.chunk, &other_body.chunk),
            (None, None) => true,
            _ => false,
        }
    }
}

impl Eq for Function {}

impl Hash for Function {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash by first body's pointer address for consistency with PartialEq
        if let Some(body) = self.bodies.first() {
            Arc::as_ptr(&body.chunk).hash(state);
        }
    }
}

impl Display for Function {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bodies.is_empty() {
            match self.name {
                Some(ref func_name) => write!(f, "#<function {func_name}/?>"),
                None => write!(f, "#<function/?>"),
            }
        } else if let &[ref body] = self.bodies.as_slice() {
            // Single arity - simpler format
            let rest_indicator = if body.has_rest { "+" } else { "" };
            match self.name {
                Some(ref func_name) => {
                    write!(f, "#<function {func_name}/{}{rest_indicator}>", body.arity)
                }
                None => write!(f, "#<function/{}{rest_indicator}>", body.arity),
            }
        } else {
            // Multi-arity - list all arities
            match self.name {
                Some(ref func_name) => write!(f, "#<function {func_name}/")?,
                None => write!(f, "#<function/")?,
            }
            for (idx, body) in self.bodies.iter().enumerate() {
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
