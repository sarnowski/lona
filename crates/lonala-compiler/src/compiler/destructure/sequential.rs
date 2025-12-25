// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Sequential (vector) destructuring pattern parsing.
//!
//! This module handles parsing vector patterns like `[a b c]`, `[a & rest]`,
//! and `[a :as all]`.

use alloc::boxed::Box;

use lona_core::source;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use super::{Binding, SeqPattern};
use crate::error::{Error, Kind as ErrorKind, SourceLocation};

use super::parse::{MAX_PATTERN_DEPTH, parse_binding};

/// Parses a sequential destructuring pattern from a vector AST.
///
/// # Arguments
///
/// * `interner` - Symbol interner for interning symbol names
/// * `ast` - The vector AST to parse as a pattern
/// * `source_id` - Source ID for error reporting
/// * `depth` - Current nesting depth (starts at 0 from external callers)
///
/// # Returns
///
/// A `SeqPattern` representing the parsed destructuring pattern.
///
/// # Errors
///
/// Returns an error if:
/// - Pattern nesting exceeds `MAX_PATTERN_DEPTH`
/// - Input is not a vector
/// - Duplicate `&` in pattern
/// - `:as` not followed by a symbol
/// - `&` not followed by a binding
/// - Invalid binding target (not symbol, `_`, or nested vector)
///
/// # Examples
///
/// ```text
/// [a b c]     -> 3 symbol bindings
/// [a & rest]  -> 1 symbol binding + rest binding
/// [a _ c]     -> symbol, ignore, symbol
/// [a :as all] -> 1 symbol + as binding
/// [[a b] c]   -> nested pattern + symbol
/// ```
#[inline]
pub fn parse_sequential_pattern(
    interner: &symbol::Interner,
    ast: &Spanned<Ast>,
    source_id: source::Id,
    depth: usize,
) -> Result<SeqPattern, Error> {
    if depth > MAX_PATTERN_DEPTH {
        return Err(Error::new(
            ErrorKind::RecursionDepthExceeded {
                max_depth: MAX_PATTERN_DEPTH,
            },
            SourceLocation::new(source_id, ast.span),
        ));
    }
    let Ast::Vector(ref elements) = ast.node else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: "expected vector pattern",
            },
            SourceLocation::new(source_id, ast.span),
        ));
    };

    let mut pattern = SeqPattern::new(ast.span);
    let mut idx = 0_usize;
    let mut seen_ampersand = false;
    let mut seen_as = false;
    let ctx = ParseContext {
        interner,
        elements,
        source_id,
        depth,
    };

    while idx < elements.len() {
        let Some(elem) = elements.get(idx) else { break };

        match elem.node {
            // `&` introduces rest binding
            Ast::Symbol(ref name) if name == "&" => {
                if seen_ampersand {
                    return Err(Error::new(
                        ErrorKind::InvalidDestructuringPattern {
                            message: "duplicate & in pattern",
                        },
                        SourceLocation::new(source_id, elem.span),
                    ));
                }
                seen_ampersand = true;
                let rest_binding = parse_rest_binding(&ctx, elem, idx)?;
                pattern.rest = Some(Box::new(rest_binding));
                idx = idx.saturating_add(1); // Skip past the rest binding
            }

            // `:as` introduces whole-collection binding
            Ast::Keyword(ref name) if name == "as" => {
                if seen_as {
                    return Err(Error::new(
                        ErrorKind::InvalidDestructuringPattern {
                            message: "duplicate :as in pattern",
                        },
                        SourceLocation::new(source_id, elem.span),
                    ));
                }
                seen_as = true;
                pattern.as_binding = Some(parse_as_binding(&ctx, elem, idx)?);
                idx = idx.saturating_add(1); // Skip past the symbol
            }

            // Regular binding (symbol, _, or nested vector)
            // All other AST types fall through to error in parse_binding
            Ast::Symbol(_)
            | Ast::Vector(_)
            | Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Map(_)
            | Ast::Set(_)
            | Ast::WithMeta { .. }
            | _ => {
                if seen_ampersand {
                    return Err(Error::new(
                        ErrorKind::InvalidDestructuringPattern {
                            message: "unexpected binding after rest binding",
                        },
                        SourceLocation::new(source_id, elem.span),
                    ));
                }
                let binding = parse_binding(ctx.interner, elem, source_id, ctx.depth)?;
                pattern.items.push(binding);
            }
        }

        idx = idx.saturating_add(1);
    }

    Ok(pattern)
}

/// Context for parsing within a pattern.
struct ParseContext<'elements> {
    interner: &'elements symbol::Interner,
    elements: &'elements [Spanned<Ast>],
    source_id: source::Id,
    depth: usize,
}

/// Parses a rest binding after `&`.
///
/// Returns the parsed binding for the rest position.
fn parse_rest_binding(
    ctx: &ParseContext<'_>,
    ampersand_elem: &Spanned<Ast>,
    idx: usize,
) -> Result<Binding, Error> {
    let rest_idx = idx.saturating_add(1);
    let Some(rest_elem) = ctx.elements.get(rest_idx) else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: "& must be followed by a binding",
            },
            SourceLocation::new(ctx.source_id, ampersand_elem.span),
        ));
    };
    parse_binding(ctx.interner, rest_elem, ctx.source_id, ctx.depth)
}

/// Parses an `:as` binding.
///
/// Returns the symbol ID for the binding.
fn parse_as_binding(
    ctx: &ParseContext<'_>,
    as_elem: &Spanned<Ast>,
    idx: usize,
) -> Result<symbol::Id, Error> {
    let sym_idx = idx.saturating_add(1);
    let Some(sym_elem) = ctx.elements.get(sym_idx) else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: ":as must be followed by a symbol",
            },
            SourceLocation::new(ctx.source_id, as_elem.span),
        ));
    };

    let Ast::Symbol(ref sym_name) = sym_elem.node else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: ":as must be followed by a symbol",
            },
            SourceLocation::new(ctx.source_id, sym_elem.span),
        ));
    };

    Ok(ctx.interner.intern(sym_name))
}
