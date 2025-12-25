// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Common parsing utilities for destructuring patterns.
//!
//! This module contains shared utilities used by both sequential and map
//! pattern parsing:
//! - `MAX_PATTERN_DEPTH` - recursion limit for nested patterns
//! - `parse_binding` - parses a single binding target

use alloc::boxed::Box;

use lona_core::source;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use super::Binding;
use crate::error::{Error, Kind as ErrorKind, SourceLocation};

// Import parsing functions from sibling modules
use super::map::parse_map_pattern;
use super::sequential::parse_sequential_pattern;

/// Maximum nesting depth for destructuring patterns.
///
/// This is a generous limit (1024) that allows legitimate complex patterns
/// while preventing stack overflow from runaway recursion.
pub const MAX_PATTERN_DEPTH: usize = 1024;

// =============================================================================
// Common Binding Parsing
// =============================================================================

/// Parses a single binding target.
///
/// A binding can be:
/// - A symbol name (binds value to that name)
/// - `_` (ignores the value)
/// - A nested vector (recursive sequential destructuring)
/// - A nested map (recursive associative destructuring)
///
/// # Arguments
///
/// * `interner` - Symbol interner for interning symbol names
/// * `ast` - The AST node to parse as a binding
/// * `source_id` - Source ID for error reporting
/// * `depth` - Current nesting depth for recursion limit checking
#[inline]
pub fn parse_binding(
    interner: &symbol::Interner,
    ast: &Spanned<Ast>,
    source_id: source::Id,
    depth: usize,
) -> Result<Binding, Error> {
    match ast.node {
        Ast::Symbol(ref name) if name == "_" => Ok(Binding::Ignore),

        Ast::Symbol(ref name) => {
            let sym_id = interner.intern(name);
            Ok(Binding::Symbol(sym_id))
        }

        Ast::Vector(_) => {
            let nested =
                parse_sequential_pattern(interner, ast, source_id, depth.saturating_add(1))?;
            Ok(Binding::Seq(Box::new(nested)))
        }

        Ast::Map(_) => {
            let nested = parse_map_pattern(interner, ast, source_id, depth.saturating_add(1))?;
            Ok(Binding::Map(Box::new(nested)))
        }

        // All other AST types are invalid binding targets
        Ast::Integer(_)
        | Ast::Float(_)
        | Ast::String(_)
        | Ast::Bool(_)
        | Ast::Nil
        | Ast::Keyword(_)
        | Ast::List(_)
        | Ast::Set(_)
        | Ast::WithMeta { .. }
        | _ => Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: "binding must be a symbol, _, vector, or map pattern",
            },
            SourceLocation::new(source_id, ast.span),
        )),
    }
}
