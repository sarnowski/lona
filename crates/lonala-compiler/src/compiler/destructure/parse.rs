// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Parsing logic for destructuring patterns.
//!
//! This module handles parsing AST nodes into pattern IR types (`SeqPattern`,
//! `MapPattern`) that the compiler then emits bytecode for.

use alloc::boxed::Box;
use alloc::vec::Vec;

use lona_core::source;
use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use super::{Binding, MapPattern, SeqPattern};
use crate::error::{Error, Kind as ErrorKind, SourceLocation};

/// Maximum nesting depth for destructuring patterns.
///
/// This is a generous limit (1024) that allows legitimate complex patterns
/// while preventing stack overflow from runaway recursion.
pub const MAX_PATTERN_DEPTH: usize = 1024;

// =============================================================================
// Sequential Pattern Parsing
// =============================================================================

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
    interner: &mut symbol::Interner,
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
    let mut ctx = ParseContext {
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
                let rest_binding = parse_rest_binding(&mut ctx, elem, idx)?;
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
                pattern.as_binding = Some(parse_as_binding(&mut ctx, elem, idx)?);
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
    interner: &'elements mut symbol::Interner,
    elements: &'elements [Spanned<Ast>],
    source_id: source::Id,
    depth: usize,
}

/// Parses a rest binding after `&`.
///
/// Returns the parsed binding for the rest position.
fn parse_rest_binding(
    ctx: &mut ParseContext<'_>,
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
    ctx: &mut ParseContext<'_>,
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
fn parse_binding(
    interner: &mut symbol::Interner,
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

// =============================================================================
// Map Pattern Parsing
// =============================================================================

/// Parses an associative (map) destructuring pattern from a map AST.
///
/// # Arguments
///
/// * `interner` - Symbol interner for interning symbol names
/// * `ast` - The map AST to parse as a pattern
/// * `source_id` - Source ID for error reporting
/// * `depth` - Current nesting depth (starts at 0 from external callers)
///
/// # Returns
///
/// A `MapPattern` representing the parsed destructuring pattern.
///
/// # Errors
///
/// Returns an error if:
/// - Pattern nesting exceeds `MAX_PATTERN_DEPTH`
/// - The pattern is invalid. See `MapPatternParser::parse_entry` for specific conditions
///
/// # Examples
///
/// ```text
/// {:keys [a b]}           -> keys=[a, b]
/// {:strs [name]}          -> strs=[name]
/// {:syms [x]}             -> syms=[x]
/// {:or {a 0}}             -> defaults=[(a, 0)]
/// {:as m}                 -> as_binding=Some(m)
/// {a :key-a}              -> explicit=[(a, :key-a)]
/// {:keys [a] :or {a 0}}   -> combined pattern
/// ```
#[inline]
pub fn parse_map_pattern(
    interner: &mut symbol::Interner,
    ast: &Spanned<Ast>,
    source_id: source::Id,
    depth: usize,
) -> Result<MapPattern, Error> {
    if depth > MAX_PATTERN_DEPTH {
        return Err(Error::new(
            ErrorKind::RecursionDepthExceeded {
                max_depth: MAX_PATTERN_DEPTH,
            },
            SourceLocation::new(source_id, ast.span),
        ));
    }

    let Ast::Map(ref elements) = ast.node else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: "expected map pattern",
            },
            SourceLocation::new(source_id, ast.span),
        ));
    };

    // Map must have even number of elements (key-value pairs)
    if !elements.len().is_multiple_of(2) {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: "map pattern must have even number of elements",
            },
            SourceLocation::new(source_id, ast.span),
        ));
    }

    let mut parser = MapPatternParser::new(interner, source_id, ast.span, depth);
    parser.parse_all_entries(elements)?;
    Ok(parser.into_pattern())
}

/// Bitflags for tracking which special keywords have been seen in map patterns.
#[derive(Default)]
struct SeenKeywords {
    /// Packed flags: bit 0=keys, 1=strs, 2=syms, 3=or, 4=as
    flags: u8,
}

impl SeenKeywords {
    const KEYS: u8 = 1 << 0;
    const STRS: u8 = 1 << 1;
    const SYMS: u8 = 1 << 2;
    const OR: u8 = 1 << 3;
    const AS: u8 = 1 << 4;

    /// Check and set a flag, returning true if it was already set.
    const fn check_and_set(&mut self, flag: u8) -> bool {
        let was_set = self.flags & flag != 0;
        self.flags |= flag;
        was_set
    }
}

/// State machine for parsing map destructuring patterns.
struct MapPatternParser<'interner> {
    interner: &'interner mut symbol::Interner,
    source_id: source::Id,
    pattern: MapPattern,
    seen: SeenKeywords,
    depth: usize,
}

impl<'interner> MapPatternParser<'interner> {
    /// Creates a new parser instance.
    fn new(
        interner: &'interner mut symbol::Interner,
        source_id: source::Id,
        span: Span,
        depth: usize,
    ) -> Self {
        Self {
            interner,
            source_id,
            pattern: MapPattern::new(span),
            seen: SeenKeywords::default(),
            depth,
        }
    }

    /// Parses all key-value pairs from the map elements.
    fn parse_all_entries(&mut self, elements: &[Spanned<Ast>]) -> Result<(), Error> {
        let mut idx = 0_usize;
        while idx < elements.len() {
            let Some(key_ast) = elements.get(idx) else {
                break;
            };
            let Some(value_ast) = elements.get(idx.saturating_add(1)) else {
                break;
            };
            idx = idx.saturating_add(2);
            self.parse_entry(key_ast, value_ast)?;
        }
        Ok(())
    }

    /// Parses a single key-value entry from the map pattern.
    fn parse_entry(
        &mut self,
        key_ast: &Spanned<Ast>,
        value_ast: &Spanned<Ast>,
    ) -> Result<(), Error> {
        match key_ast.node {
            Ast::Keyword(ref kw) if kw == "keys" => self.parse_keys(key_ast, value_ast),
            Ast::Keyword(ref kw) if kw == "strs" => self.parse_strs(key_ast, value_ast),
            Ast::Keyword(ref kw) if kw == "syms" => self.parse_syms(key_ast, value_ast),
            Ast::Keyword(ref kw) if kw == "or" => self.parse_or(key_ast, value_ast),
            Ast::Keyword(ref kw) if kw == "as" => self.parse_as(key_ast, value_ast),
            // Binding targets: symbol, vector (nested seq), or map (nested map)
            Ast::Symbol(_) | Ast::Vector(_) | Ast::Map(_) => {
                self.parse_explicit_binding(key_ast, value_ast)?;
                Ok(())
            }
            // Explicit variants for clippy::wildcard_enum_match_arm compliance
            Ast::Integer(_)
            | Ast::Float(_)
            | Ast::String(_)
            | Ast::Bool(_)
            | Ast::Nil
            | Ast::Keyword(_)
            | Ast::List(_)
            | Ast::Set(_)
            | Ast::WithMeta { .. }
            // Wildcard required for #[non_exhaustive] - handles future variants
            | _ => Err(self.invalid_key_error(key_ast.span)),
        }
    }

    /// Parses `:keys [a b]` entry.
    fn parse_keys(
        &mut self,
        key_ast: &Spanned<Ast>,
        value_ast: &Spanned<Ast>,
    ) -> Result<(), Error> {
        if self.seen.check_and_set(SeenKeywords::KEYS) {
            return Err(self.duplicate_error("duplicate :keys in map pattern", key_ast.span));
        }
        self.pattern.keys = parse_symbol_vector(self.interner, value_ast, self.source_id)?;
        Ok(())
    }

    /// Parses `:strs [a b]` entry.
    fn parse_strs(
        &mut self,
        key_ast: &Spanned<Ast>,
        value_ast: &Spanned<Ast>,
    ) -> Result<(), Error> {
        if self.seen.check_and_set(SeenKeywords::STRS) {
            return Err(self.duplicate_error("duplicate :strs in map pattern", key_ast.span));
        }
        self.pattern.strs = parse_symbol_vector(self.interner, value_ast, self.source_id)?;
        Ok(())
    }

    /// Parses `:syms [a b]` entry.
    fn parse_syms(
        &mut self,
        key_ast: &Spanned<Ast>,
        value_ast: &Spanned<Ast>,
    ) -> Result<(), Error> {
        if self.seen.check_and_set(SeenKeywords::SYMS) {
            return Err(self.duplicate_error("duplicate :syms in map pattern", key_ast.span));
        }
        self.pattern.syms = parse_symbol_vector(self.interner, value_ast, self.source_id)?;
        Ok(())
    }

    /// Parses `:or {a default}` entry.
    fn parse_or(&mut self, key_ast: &Spanned<Ast>, value_ast: &Spanned<Ast>) -> Result<(), Error> {
        if self.seen.check_and_set(SeenKeywords::OR) {
            return Err(self.duplicate_error("duplicate :or in map pattern", key_ast.span));
        }
        self.pattern.defaults = parse_defaults_map(self.interner, value_ast, self.source_id)?;
        Ok(())
    }

    /// Parses `:as name` entry.
    fn parse_as(&mut self, key_ast: &Spanned<Ast>, value_ast: &Spanned<Ast>) -> Result<(), Error> {
        if self.seen.check_and_set(SeenKeywords::AS) {
            return Err(self.duplicate_error("duplicate :as in map pattern", key_ast.span));
        }

        let Ast::Symbol(ref sym_name) = value_ast.node else {
            return Err(Error::new(
                ErrorKind::InvalidDestructuringPattern {
                    message: ":as must be followed by a symbol",
                },
                SourceLocation::new(self.source_id, value_ast.span),
            ));
        };

        self.pattern.as_binding = Some(self.interner.intern(sym_name));
        Ok(())
    }

    /// Parses explicit binding (`{a :key-a}`, `{[a b] :point}`, or `{{:keys [x]} :inner}`).
    fn parse_explicit_binding(
        &mut self,
        binding_ast: &Spanned<Ast>,
        value_ast: &Spanned<Ast>,
    ) -> Result<(), Error> {
        let binding = parse_binding(self.interner, binding_ast, self.source_id, self.depth)?;
        self.pattern.explicit.push((binding, value_ast.clone()));
        Ok(())
    }

    /// Creates a duplicate keyword error.
    const fn duplicate_error(&self, message: &'static str, span: Span) -> Error {
        Error::new(
            ErrorKind::InvalidDestructuringPattern { message },
            SourceLocation::new(self.source_id, span),
        )
    }

    /// Creates an invalid key error.
    const fn invalid_key_error(&self, span: Span) -> Error {
        Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: "map pattern key must be a binding target (symbol, vector, or map) or special keyword (:keys, :strs, :syms, :or, :as)",
            },
            SourceLocation::new(self.source_id, span),
        )
    }

    /// Consumes the parser and returns the built pattern.
    fn into_pattern(self) -> MapPattern {
        self.pattern
    }
}

/// Parses a vector of symbols for `:keys`, `:strs`, or `:syms`.
///
/// # Errors
///
/// Returns an error if the value is not a vector or contains non-symbols.
fn parse_symbol_vector(
    interner: &mut symbol::Interner,
    ast: &Spanned<Ast>,
    source_id: source::Id,
) -> Result<Vec<symbol::Id>, Error> {
    let Ast::Vector(ref elements) = ast.node else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: ":keys/:strs/:syms value must be a vector",
            },
            SourceLocation::new(source_id, ast.span),
        ));
    };

    let mut result = Vec::new();

    for elem in elements {
        let Ast::Symbol(ref name) = elem.node else {
            return Err(Error::new(
                ErrorKind::InvalidDestructuringPattern {
                    message: ":keys/:strs/:syms vector must contain only symbols",
                },
                SourceLocation::new(source_id, elem.span),
            ));
        };

        result.push(interner.intern(name));
    }

    Ok(result)
}

/// Parses a defaults map for `:or`.
///
/// # Errors
///
/// Returns an error if the value is not a map or keys are not symbols.
fn parse_defaults_map(
    interner: &mut symbol::Interner,
    ast: &Spanned<Ast>,
    source_id: source::Id,
) -> Result<Vec<(symbol::Id, Spanned<Ast>)>, Error> {
    let Ast::Map(ref elements) = ast.node else {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: ":or value must be a map",
            },
            SourceLocation::new(source_id, ast.span),
        ));
    };

    // Map must have even number of elements
    if !elements.len().is_multiple_of(2) {
        return Err(Error::new(
            ErrorKind::InvalidDestructuringPattern {
                message: ":or map must have even number of elements",
            },
            SourceLocation::new(source_id, ast.span),
        ));
    }

    let mut result = Vec::new();

    let mut idx = 0_usize;
    while idx < elements.len() {
        let Some(key_ast) = elements.get(idx) else {
            break;
        };
        let Some(value_ast) = elements.get(idx.saturating_add(1)) else {
            break;
        };
        idx = idx.saturating_add(2);

        let Ast::Symbol(ref sym_name) = key_ast.node else {
            return Err(Error::new(
                ErrorKind::InvalidDestructuringPattern {
                    message: ":or map keys must be symbols",
                },
                SourceLocation::new(source_id, key_ast.span),
            ));
        };

        let sym_id = interner.intern(sym_name);
        result.push((sym_id, value_ast.clone()));
    }

    Ok(result)
}
