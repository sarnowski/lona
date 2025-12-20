// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Set-related collection operations.
//!
//! Provides native implementations for set manipulation:
//! - `hash-set` - create a set from arguments
//! - `disj` - remove element from set
//! - `set?` - type predicate for sets
//! - `conj` - add element to collection (polymorphic)
//! - `contains?` - check if collection contains element (polymorphic)
//! - `count` - get collection size (polymorphic)

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::set::Set;
use lona_core::value::{self, Value};

use crate::vm::natives::{NativeContext, NativeError};

/// `(hash-set & args)` - create set from arguments.
///
/// Creates a new set containing all arguments. Duplicates are silently
/// removed (following Clojure semantics for runtime construction).
#[inline]
pub fn native_hash_set(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let set = Set::from_values(args.iter().cloned());
    Ok(Value::Set(set))
}

/// `(disj set x & xs)` - remove elements from set.
///
/// Returns a new set with the specified elements removed.
/// Requires at least 2 arguments (set and element to remove).
/// Ignores elements that are not present in the set.
#[inline]
pub fn native_disj(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() < 2_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::AtLeast(2_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let set_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
    let set = set_arg.as_set().ok_or_else(|| NativeError::TypeError {
        expected: TypeExpectation::Single(value::Kind::Set),
        got: set_arg.kind(),
        arg_index: 0_u8,
    })?;

    // Remove all specified elements
    let mut result = set.clone();
    for element in args.iter().skip(1_usize) {
        result = result.remove(element);
    }

    Ok(Value::Set(result))
}

/// `(set? x)` - type predicate for sets.
///
/// Returns true if the argument is a set.
#[inline]
pub fn native_set_p(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
    Ok(Value::Bool(arg.is_set()))
}

/// `(conj coll x & xs)` - add elements to collection.
///
/// Adds elements in a type-appropriate way:
/// - List: prepends (like cons)
/// - Vector: appends to end
/// - Set: adds element (no-op if exists)
/// - Map: not yet supported (use assoc)
/// - nil: creates a list with the elements
#[inline]
pub fn native_conj(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() < 2_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::AtLeast(2_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let collection = args.first().ok_or(NativeError::Error("missing argument"))?;
    let elements = args.get(1_usize..).unwrap_or(&[]);

    match *collection {
        Value::List(ref list) => {
            // Lists prepend elements (like cons)
            let mut result = list.clone();
            for element in elements {
                result = result.cons(element.clone());
            }
            Ok(Value::List(result))
        }
        Value::Vector(ref vec) => {
            // Vectors append elements
            let mut result = vec.clone();
            for element in elements {
                result = result.push(element.clone());
            }
            Ok(Value::Vector(result))
        }
        Value::Set(ref set) => {
            // Sets add elements (deduplication is handled internally)
            let mut result = set.clone();
            for element in elements {
                result = result.insert(element.clone());
            }
            Ok(Value::Set(result))
        }
        Value::Nil => {
            // nil becomes a list with elements
            let mut result = lona_core::list::List::empty();
            for element in elements {
                result = result.cons(element.clone());
            }
            Ok(Value::List(result))
        }
        // All other types are errors (explicit list + wildcard for future variants)
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::Keyword(_)
        | Value::String(_)
        | Value::Map(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Collection,
            got: collection.kind(),
            arg_index: 0_u8,
        }),
    }
}

/// `(contains? coll key)` - check if collection contains key.
///
/// For sets: checks if element is in set.
/// For maps: checks if key exists.
/// For vectors: checks if index is valid.
/// For strings: checks if index is valid.
#[inline]
pub fn native_contains_p(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 2_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(2_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let collection = args.first().ok_or(NativeError::Error("missing argument"))?;
    let key = args
        .get(1_usize)
        .ok_or(NativeError::Error("missing argument"))?;

    let result = match *collection {
        Value::Set(ref set) => set.contains(key),
        Value::Map(ref map) => map.contains_key(key),
        Value::Vector(ref vec) => {
            // For vectors, key must be a non-negative integer index
            key.as_integer()
                .and_then(lona_core::integer::Integer::to_i64)
                .and_then(|idx_i64| usize::try_from(idx_i64).ok())
                .is_some_and(|idx_usize| idx_usize < vec.len())
        }
        Value::String(ref string) => {
            // For strings, key must be a non-negative integer index
            key.as_integer()
                .and_then(lona_core::integer::Integer::to_i64)
                .and_then(|idx_i64| usize::try_from(idx_i64).ok())
                .is_some_and(|idx_usize| idx_usize < string.len())
        }
        // All other types return false (consistent with Clojure)
        // Explicit list + wildcard for future variants
        Value::Nil
        | Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::Keyword(_)
        | Value::List(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | _ => false,
    };

    Ok(Value::Bool(result))
}

/// `(count coll)` - get collection size.
///
/// Returns the number of elements in the collection.
/// Returns 0 for nil.
#[inline]
pub fn native_count(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let collection = args.first().ok_or(NativeError::Error("missing argument"))?;

    let count = match *collection {
        Value::List(ref list) => list.len(),
        Value::Vector(ref vec) => vec.len(),
        Value::Map(ref map) => map.len(),
        Value::Set(ref set) => set.len(),
        Value::String(ref string) => string.len(),
        Value::Nil => 0_usize,
        // All other types are errors (explicit list + wildcard for future variants)
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::Keyword(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | _ => {
            return Err(NativeError::TypeError {
                expected: TypeExpectation::Sequence,
                got: collection.kind(),
                arg_index: 0_u8,
            });
        }
    };

    // Convert count to Integer
    let count_i64 = i64::try_from(count).unwrap_or(i64::MAX);
    Ok(Value::Integer(lona_core::integer::Integer::from_i64(
        count_i64,
    )))
}
