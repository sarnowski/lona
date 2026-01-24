// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Pattern representation and parsing for `match` expressions.
//!
//! Patterns are used in `match` to destructure values and bind variables.
//! This module provides:
//! - `Pattern` enum representing all pattern forms
//! - Parsing from `Value` to `Pattern`

#[cfg(any(test, feature = "std"))]
use std::{boxed::Box, vec::Vec};

#[cfg(not(any(test, feature = "std")))]
use alloc::{boxed::Box, vec::Vec};

use crate::platform::MemorySpace;
use crate::value::Value;

use super::{CompileError, Compiler, MAX_SYMBOL_NAME_LEN};

/// Maximum number of sub-patterns in a tuple/vector pattern.
const MAX_PATTERN_ELEMENTS: usize = 16;

/// Maximum number of key-value pairs in a map pattern.
const MAX_MAP_PATTERN_PAIRS: usize = 8;

/// A compiled pattern for use in `match` expressions.
///
/// Patterns are matched against values at runtime. Binding patterns
/// capture values into variables, while literal patterns test for equality.
#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    /// Wildcard `_` - matches anything, binds nothing.
    Wildcard,

    /// Binding `x` - matches anything, captures value into named variable.
    Binding {
        /// Symbol name for the binding.
        name: [u8; MAX_SYMBOL_NAME_LEN],
        /// Length of the name in bytes.
        name_len: u8,
    },

    /// Literal - matches exact value (nil, bool, int, keyword, string).
    Literal(Value),

    /// Tuple `[a b c]` - matches tuple of exact length with sub-patterns.
    Tuple(Vec<Self>),

    /// Tuple with rest `[h & t]` - matches non-empty tuple, binding rest.
    TupleRest {
        /// Patterns for fixed head elements.
        head: Vec<Self>,
        /// Pattern for the remaining elements (as a tuple).
        rest: Box<Self>,
    },

    /// Vector `{a b c}` - matches vector of exact length with sub-patterns.
    Vector(Vec<Self>),

    /// Map `%{:k v}` - matches map containing specified keys.
    Map(Vec<(Value, Self)>),
}

impl Pattern {
    /// Create a wildcard pattern.
    #[must_use]
    pub const fn wildcard() -> Self {
        Self::Wildcard
    }

    /// Create a binding pattern from name bytes.
    #[must_use]
    pub fn binding(name: &[u8]) -> Self {
        let mut name_buf = [0u8; MAX_SYMBOL_NAME_LEN];
        let len = name.len().min(MAX_SYMBOL_NAME_LEN);
        name_buf[..len].copy_from_slice(&name[..len]);
        Self::Binding {
            name: name_buf,
            name_len: len as u8,
        }
    }

    /// Create a literal pattern.
    #[must_use]
    pub const fn literal(value: Value) -> Self {
        Self::Literal(value)
    }

    /// Get the name of a binding pattern as a slice.
    #[must_use]
    pub fn binding_name(&self) -> Option<&[u8]> {
        match self {
            Self::Binding { name, name_len } => Some(&name[..*name_len as usize]),
            _ => None,
        }
    }
}

impl<M: MemorySpace> Compiler<'_, M> {
    /// Parse a value as a pattern.
    ///
    /// Converts a runtime `Value` (from parsed source) into a `Pattern`
    /// suitable for compilation into bytecode tests.
    ///
    /// # Errors
    ///
    /// Returns `CompileError::InvalidSyntax` if the value cannot be parsed as a pattern.
    pub fn parse_pattern(&self, pat_val: Value) -> Result<Pattern, CompileError> {
        match pat_val {
            // Symbol: either wildcard `_` or binding `x`
            Value::Symbol(_) => self.parse_symbol_pattern(pat_val),

            // Literals: nil, bool, int, keyword, string
            Value::Nil | Value::Bool(_) | Value::Int(_) | Value::Keyword(_) | Value::String(_) => {
                Ok(Pattern::Literal(pat_val))
            }

            // Tuple pattern: `[a b]` or `[h & t]`
            Value::Tuple(_) => self.parse_tuple_pattern(pat_val),

            // Vector pattern: `{a b c}`
            Value::Vector(_) => self.parse_vector_pattern(pat_val),

            // Map pattern: `%{:k v}`
            Value::Map(_) => self.parse_map_pattern(pat_val),

            // Other types are invalid patterns
            _ => Err(CompileError::InvalidSyntax),
        }
    }

    /// Parse a symbol as a pattern (wildcard or binding).
    fn parse_symbol_pattern(&self, sym: Value) -> Result<Pattern, CompileError> {
        let name_str = self
            .proc
            .read_string(self.mem, sym)
            .ok_or(CompileError::InvalidSyntax)?;

        if name_str == "_" {
            Ok(Pattern::Wildcard)
        } else {
            let name_bytes = name_str.as_bytes();
            Ok(Pattern::binding(name_bytes))
        }
    }

    /// Parse a tuple pattern `[a b]` or `[h & t]`.
    fn parse_tuple_pattern(&self, tuple: Value) -> Result<Pattern, CompileError> {
        let len = self
            .proc
            .read_tuple_len(self.mem, tuple)
            .ok_or(CompileError::InvalidSyntax)?;

        if len > MAX_PATTERN_ELEMENTS {
            return Err(CompileError::InvalidSyntax);
        }

        let mut patterns = Vec::new();

        for i in 0..len {
            let elem = self
                .proc
                .read_tuple_element(self.mem, tuple, i)
                .ok_or(CompileError::InvalidSyntax)?;

            // Check for `& rest` syntax
            if elem.is_symbol() {
                let name = self
                    .proc
                    .read_string(self.mem, elem)
                    .ok_or(CompileError::InvalidSyntax)?;

                if name == "&" {
                    // Must have exactly one more element after `&`
                    if i + 2 != len {
                        return Err(CompileError::InvalidSyntax);
                    }

                    let rest_val = self
                        .proc
                        .read_tuple_element(self.mem, tuple, i + 1)
                        .ok_or(CompileError::InvalidSyntax)?;
                    let rest_pat = self.parse_pattern(rest_val)?;

                    return Ok(Pattern::TupleRest {
                        head: patterns,
                        rest: Box::new(rest_pat),
                    });
                }
            }

            patterns.push(self.parse_pattern(elem)?);
        }

        Ok(Pattern::Tuple(patterns))
    }

    /// Parse a vector pattern `{a b c}`.
    fn parse_vector_pattern(&self, vector: Value) -> Result<Pattern, CompileError> {
        let len = self
            .proc
            .read_tuple_len(self.mem, vector)
            .ok_or(CompileError::InvalidSyntax)?;

        if len > MAX_PATTERN_ELEMENTS {
            return Err(CompileError::InvalidSyntax);
        }

        let mut patterns = Vec::new();

        for i in 0..len {
            let elem = self
                .proc
                .read_tuple_element(self.mem, vector, i)
                .ok_or(CompileError::InvalidSyntax)?;
            patterns.push(self.parse_pattern(elem)?);
        }

        Ok(Pattern::Vector(patterns))
    }

    /// Parse a map pattern `%{:k v}`.
    fn parse_map_pattern(&self, map: Value) -> Result<Pattern, CompileError> {
        let map_val = self
            .proc
            .read_map(self.mem, map)
            .ok_or(CompileError::InvalidSyntax)?;

        let mut pairs = Vec::new();
        let mut current = map_val.entries;

        while let Some(pair) = self.proc.read_pair(self.mem, current) {
            if pairs.len() >= MAX_MAP_PATTERN_PAIRS {
                return Err(CompileError::InvalidSyntax);
            }

            // Each entry is a [key value] tuple
            let kv = pair.first;
            let key = self
                .proc
                .read_tuple_element(self.mem, kv, 0)
                .ok_or(CompileError::InvalidSyntax)?;
            let val_pattern = self
                .proc
                .read_tuple_element(self.mem, kv, 1)
                .ok_or(CompileError::InvalidSyntax)?;

            // Key must be a literal (usually a keyword)
            // Value is parsed as a sub-pattern
            pairs.push((key, self.parse_pattern(val_pattern)?));
            current = pair.rest;
        }

        Ok(Pattern::Map(pairs))
    }
}
