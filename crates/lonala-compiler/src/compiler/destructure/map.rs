// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Map (associative) destructuring pattern parsing.
//!
//! This module handles parsing map patterns like `{:keys [a b]}`, `{a :key-a}`,
//! and `{:or {a 0} :as m}`.

use alloc::vec::Vec;

use lona_core::source;
use lona_core::span::Span;
use lona_core::symbol;
use lonala_parser::{Ast, Spanned};

use super::MapPattern;
use super::parse::{MAX_PATTERN_DEPTH, parse_binding};
use crate::error::{Error, Kind as ErrorKind, SourceLocation};

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
    interner: &symbol::Interner,
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
    interner: &'interner symbol::Interner,
    source_id: source::Id,
    pattern: MapPattern,
    seen: SeenKeywords,
    depth: usize,
}

impl<'interner> MapPatternParser<'interner> {
    /// Creates a new parser instance.
    fn new(
        interner: &'interner symbol::Interner,
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
    interner: &symbol::Interner,
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
    interner: &symbol::Interner,
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
