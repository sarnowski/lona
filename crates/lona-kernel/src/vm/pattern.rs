// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Runtime pattern matching engine for Lonala.
//!
//! This module provides the core pattern matching functionality used by
//! the `receive` special form (process message matching) and `case` special form.
//!
//! # Specification Reference
//!
//! See the Lonala language specification:
//! - `docs/lonala/special-forms.md` - Pattern syntax (when `receive` and `case` are documented)
//! - `docs/lonala/concurrency.md` - Process model and message matching (planned)
//!
//! # Design
//!
//! Unlike compile-time destructuring (which assumes structure is correct),
//! pattern matching must:
//! 1. **Test** if a value matches a pattern shape
//! 2. **Extract** bound variables if successful
//! 3. **Return failure** if not matched (no error, just move to next pattern)
//!
//! # Pattern Types
//!
//! - [`Pattern::Wildcard`]: Matches any value, discards it (`_`)
//! - [`Pattern::Bind`]: Matches any value, binds it to a symbol
//! - [`Pattern::Literal`]: Matches a specific literal value
//! - [`Pattern::Seq`]: Matches sequential collections (vector or list)
//! - [`Pattern::Map`]: Matches maps by extracting values at specified keys
//! - [`Pattern::Guarded`]: Pattern with guard expression (guard evaluated by VM)
//!
//! # Example
//!
//! ```ignore
//! use lona_kernel::vm::pattern::{Pattern, try_match};
//! use lona_core::{symbol, value::Value};
//!
//! // Pattern: [_ x]
//! let pattern = Pattern::Seq {
//!     items: vec![Pattern::Wildcard, Pattern::Bind(x_id)],
//!     rest: None,
//! };
//!
//! // Value: [1 2]
//! let value = /* vector [1 2] */;
//!
//! // Match returns bindings: [(x_id, 2)]
//! let bindings = try_match(&pattern, &value);
//! ```

use alloc::boxed::Box;
use alloc::vec::Vec;

use lona_core::{list::List, map::Map, symbol, value::Value, vector::Vector};

/// Pattern for runtime matching.
///
/// Represents the shape of a value to match against. Each variant corresponds
/// to a different kind of pattern in the Lonala language.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Pattern {
    /// Matches any value, discards it (`_`).
    ///
    /// Always succeeds, produces no bindings.
    Wildcard,

    /// Matches any value, binds it to a symbol.
    ///
    /// Always succeeds, produces one binding: `(symbol_id, matched_value)`.
    Bind(symbol::Id),

    /// Matches a specific literal value.
    ///
    /// Succeeds if the value equals the literal (using Lonala equality).
    /// Produces no bindings on success.
    Literal(Value),

    /// Matches a sequential collection (vector or list).
    ///
    /// - `items`: Patterns to match against positional elements
    /// - `rest`: Optional pattern to match remaining elements after `items`
    ///
    /// The value must be a vector or list. If `rest` is `None`, the collection
    /// must have exactly `items.len()` elements. If `rest` is `Some`, the
    /// collection must have at least `items.len()` elements.
    Seq {
        /// Positional patterns to match.
        items: Vec<Self>,
        /// Optional rest pattern after `&` (e.g., `[a b & rest]`).
        rest: Option<Box<Self>>,
    },

    /// Matches a map, extracting values by key.
    ///
    /// Uses open matching semantics: extra keys in the map are ignored.
    /// Only the specified keys must exist and match their sub-patterns.
    ///
    /// An empty entries list matches any map (including empty maps).
    Map {
        /// Key-pattern pairs to match.
        ///
        /// Each key must exist in the map, and its value must match the pattern.
        entries: Vec<(Value, Self)>,
    },

    /// Pattern with guard expression.
    ///
    /// The pattern is matched first to produce bindings. If it succeeds,
    /// the VM evaluates the guard expression with those bindings in scope.
    /// If the guard returns false, the match fails.
    ///
    /// This variant only contains the pattern and a reference to the guard;
    /// actual guard evaluation is the VM's responsibility.
    Guarded {
        /// The pattern to match.
        pattern: Box<Self>,
        /// Index into the guard expressions array (VM-specific).
        guard_index: usize,
    },
}

/// Result of a successful pattern match.
///
/// A list of symbol ID to value bindings extracted from the match.
pub type Bindings = Vec<(symbol::Id, Value)>;

/// Maximum recursion depth for nested pattern matching.
///
/// Prevents stack overflow on deeply nested or malicious patterns.
/// The depth of 64 balances practical nesting needs against stack usage.
pub const MAX_PATTERN_DEPTH: usize = 64;

/// Matches a pattern against a value.
///
/// Returns `Some(bindings)` if the pattern matches, where `bindings` is a
/// vector of `(symbol_id, value)` pairs for all bound variables.
///
/// Returns `None` if the pattern does not match. This is not an error—
/// the caller should try the next pattern.
///
/// # Arguments
///
/// * `pattern` - The pattern to match against
/// * `value` - The value to match
///
/// # Returns
///
/// * `Some(bindings)` - Pattern matched, with extracted bindings
/// * `None` - Pattern did not match
#[inline]
#[must_use]
pub fn try_match(pattern: &Pattern, value: &Value) -> Option<Bindings> {
    try_match_with_depth(pattern, value, 0)
}

/// Internal matching function with depth tracking.
fn try_match_with_depth(pattern: &Pattern, value: &Value, depth: usize) -> Option<Bindings> {
    // Guard against stack overflow from deeply nested patterns
    if depth > MAX_PATTERN_DEPTH {
        return None;
    }

    match *pattern {
        Pattern::Wildcard => Some(Vec::new()),

        Pattern::Bind(symbol_id) => Some(alloc::vec![(symbol_id, value.clone())]),

        Pattern::Literal(ref lit) => (value == lit).then(Vec::new),

        Pattern::Seq {
            ref items,
            ref rest,
        } => match_sequence(items, rest.as_deref(), value, depth),

        Pattern::Map { ref entries } => match_map(entries, value, depth),

        Pattern::Guarded {
            pattern: ref inner, ..
        } => {
            // Match the inner pattern. Guard evaluation is handled by the VM,
            // which will use the guard_index to find and evaluate the guard.
            // The pattern engine just returns the bindings.
            try_match_with_depth(inner, value, depth)
        }
    }
}

/// Matches a sequence pattern against a value.
///
/// Works with both vectors and lists.
fn match_sequence(
    items: &[Pattern],
    rest: Option<&Pattern>,
    value: &Value,
    depth: usize,
) -> Option<Bindings> {
    // Try vector first, then list. Non-sequence types return None.
    match_sequence_as_vec(items, rest, value, depth)
        .or_else(|| match_sequence_as_list(items, rest, value, depth))
}

/// Attempts to match a sequence pattern against a value as a vector.
fn match_sequence_as_vec(
    items: &[Pattern],
    rest: Option<&Pattern>,
    value: &Value,
    depth: usize,
) -> Option<Bindings> {
    let vec = value.as_vector()?;
    match_sequence_vec(items, rest, vec, depth)
}

/// Attempts to match a sequence pattern against a value as a list.
fn match_sequence_as_list(
    items: &[Pattern],
    rest: Option<&Pattern>,
    value: &Value,
    depth: usize,
) -> Option<Bindings> {
    let list = value.as_list()?;
    match_sequence_list(items, rest, list, depth)
}

/// Matches a sequence pattern against a vector.
fn match_sequence_vec(
    items: &[Pattern],
    rest: Option<&Pattern>,
    vec: &Vector,
    depth: usize,
) -> Option<Bindings> {
    let len = vec.len();

    // Check length constraints
    if rest.is_some() {
        // With rest pattern, need at least items.len() elements
        if len < items.len() {
            return None;
        }
    } else {
        // Without rest pattern, need exactly items.len() elements
        if len != items.len() {
            return None;
        }
    }

    let mut bindings = Vec::new();
    let next_depth = depth.saturating_add(1);

    // Match positional items
    for (i, item_pattern) in items.iter().enumerate() {
        let element = vec.get(i)?;
        let sub_bindings = try_match_with_depth(item_pattern, element, next_depth)?;
        bindings.extend(sub_bindings);
    }

    // Match rest pattern if present
    if let Some(rest_pattern) = rest {
        // Collect remaining elements into a vector
        let remaining: Vec<Value> = (items.len()..len)
            .filter_map(|i| vec.get(i).cloned())
            .collect();
        let rest_value = Value::Vector(Vector::from_vec(remaining));
        let rest_bindings = try_match_with_depth(rest_pattern, &rest_value, next_depth)?;
        bindings.extend(rest_bindings);
    }

    Some(bindings)
}

/// Matches a map pattern against a value.
///
/// Uses open matching semantics: extra keys in the map are ignored.
/// Only specified keys must exist and match their sub-patterns.
fn match_map(entries: &[(Value, Pattern)], value: &Value, depth: usize) -> Option<Bindings> {
    let map = value.as_map()?;
    match_map_inner(entries, map, depth)
}

/// Matches a map pattern against a Map.
fn match_map_inner(entries: &[(Value, Pattern)], map: &Map, depth: usize) -> Option<Bindings> {
    let mut bindings = Vec::new();
    let next_depth = depth.saturating_add(1);

    for entry in entries {
        let (ref key, ref sub_pattern) = *entry;
        // Key must exist in the map
        let val = map.get(key)?;
        // Value at key must match the sub-pattern
        let sub_bindings = try_match_with_depth(sub_pattern, val, next_depth)?;
        bindings.extend(sub_bindings);
    }

    Some(bindings)
}

/// Matches a sequence pattern against a list.
fn match_sequence_list(
    items: &[Pattern],
    rest: Option<&Pattern>,
    list: &List,
    depth: usize,
) -> Option<Bindings> {
    let mut bindings = Vec::new();
    let mut current = list.clone();
    let next_depth = depth.saturating_add(1);

    // Match positional items
    for item_pattern in items {
        let element = current.first()?;
        let sub_bindings = try_match_with_depth(item_pattern, element, next_depth)?;
        bindings.extend(sub_bindings);
        current = current.rest();
    }

    // Check length constraints
    if let Some(rest_pattern) = rest {
        // With rest pattern, bind remaining elements
        let rest_value = Value::List(current);
        let rest_bindings = try_match_with_depth(rest_pattern, &rest_value, next_depth)?;
        bindings.extend(rest_bindings);
    } else {
        // Without rest pattern, list must be exhausted
        if !current.is_empty() {
            return None;
        }
    }

    Some(bindings)
}
