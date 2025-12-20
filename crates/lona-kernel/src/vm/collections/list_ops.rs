// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! List-related collection operations.
//!
//! Provides native implementations for list manipulation:
//! - `cons` - prepend element to a collection
//! - `first` - get first element of a collection
//! - `rest` - get rest of a collection (tail)
//! - `list` - create a list from arguments
//! - `concat` - concatenate sequences into a list

use alloc::vec::Vec;

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::list::List;
use lona_core::value::Value;
use lona_core::vector::Vector;

use crate::vm::natives::{NativeContext, NativeError};

/// `(cons x coll)` - prepend x to collection, returns list.
///
/// Prepends the first argument to the collection in the second argument.
/// Always returns a list (following Clojure semantics).
///
/// # Type handling
///
/// - List: New list with x prepended
/// - Vector: Convert vector to list, prepend x
/// - Map: Convert map to list of `[key value]` vectors, prepend x
/// - nil: Single-element list (x)
/// - Other: `TypeError`
#[inline]
pub fn native_cons(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let &[ref element, ref collection] = args else {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(2_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    };

    let result = match *collection {
        Value::List(ref list) => list.cons(element.clone()),
        Value::Vector(ref vec) => {
            // Convert vector to list, then prepend
            let mut list = List::empty();
            // Collect to Vec first since Vector iterator doesn't implement DoubleEndedIterator
            let items: Vec<_> = vec.iter().collect();
            // Build list in reverse order since cons prepends
            for item in items.into_iter().rev() {
                list = list.cons(item.clone());
            }
            list.cons(element.clone())
        }
        Value::Map(ref map) => {
            // Convert map to list of [key value] vectors, then prepend
            let mut list = List::empty();
            let entries: Vec<_> = map.iter().collect();
            // Build list in reverse order since cons prepends
            for (key, value) in entries.into_iter().rev() {
                let entry = Vector::from_vec(alloc::vec![key.value().clone(), value.clone()]);
                list = list.cons(Value::Vector(entry));
            }
            list.cons(element.clone())
        }
        Value::Set(ref set) => {
            // Convert set to list, then prepend
            let mut list = List::empty();
            let items: Vec<_> = set.iter().collect();
            // Build list in reverse order since cons prepends
            for item in items.into_iter().rev() {
                list = list.cons(item.value().clone());
            }
            list.cons(element.clone())
        }
        Value::Nil => List::empty().cons(element.clone()),
        // All other types are errors (explicit list + wildcard for future variants)
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::Keyword(_)
        | Value::String(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | _ => {
            return Err(NativeError::TypeError {
                expected: TypeExpectation::Sequence,
                got: collection.kind(),
                arg_index: 1_u8,
            });
        }
    };

    Ok(Value::List(result))
}

/// `(first coll)` - return first element or nil.
///
/// Returns the first element of a collection, or nil if empty or nil.
///
/// # Type handling
///
/// - List: First element, or nil if empty
/// - Vector: First element (index 0), or nil if empty
/// - Map: First entry as [key value] vector, or nil if empty (order is hash-based, not insertion order)
/// - nil: nil
/// - Other: `TypeError`
#[inline]
pub fn native_first(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let &[ref collection] = args else {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    };

    let result = match *collection {
        Value::List(ref list) => list.first().cloned().unwrap_or(Value::Nil),
        Value::Vector(ref vec) => vec.get(0_usize).cloned().unwrap_or(Value::Nil),
        Value::Map(ref map) => {
            // Get first entry as [key value] vector
            if let Some((key, value)) = map.iter().next() {
                let entry = Vector::from_vec(alloc::vec![key.value().clone(), value.clone()]);
                Value::Vector(entry)
            } else {
                Value::Nil
            }
        }
        Value::Set(ref set) => {
            // Get first element of set (order is hash-based, not insertion order)
            set.iter()
                .next()
                .map_or(Value::Nil, |item| item.value().clone())
        }
        Value::Nil => Value::Nil,
        // All other types are errors (explicit list + wildcard for future variants)
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::Keyword(_)
        | Value::String(_)
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

    Ok(result)
}

/// `(rest coll)` - return rest as list.
///
/// Returns all elements except the first. Always returns a list.
/// Returns empty list for empty collections or nil.
///
/// # Type handling
///
/// - List: Tail of list (shared structure)
/// - Vector: Elements 1..n as a new list
/// - Map: Remaining entries as list of [k v] vectors (order is hash-based, not insertion order)
/// - nil: Empty list
/// - Other: `TypeError`
#[inline]
pub fn native_rest(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let &[ref collection] = args else {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    };

    let result = match *collection {
        Value::List(ref list) => list.rest(),
        Value::Vector(ref vec) => {
            // Convert vector tail to list
            let mut list = List::empty();
            // Skip first element, build list in reverse since cons prepends
            let mut items: Vec<_> = vec.iter().skip(1_usize).collect();
            items.reverse();
            for item in items {
                list = list.cons(item.clone());
            }
            list
        }
        Value::Map(ref map) => {
            // Get remaining entries as list of [k v] vectors
            let mut list = List::empty();
            let mut entries: Vec<_> = map.iter().skip(1_usize).collect();
            entries.reverse();
            for (key, value) in entries {
                let entry = Vector::from_vec(alloc::vec![key.value().clone(), value.clone()]);
                list = list.cons(Value::Vector(entry));
            }
            list
        }
        Value::Set(ref set) => {
            // Get remaining elements as list (order is hash-based)
            let mut list = List::empty();
            let mut items: Vec<_> = set.iter().skip(1_usize).collect();
            items.reverse();
            for item in items {
                list = list.cons(item.value().clone());
            }
            list
        }
        Value::Nil => List::empty(),
        // All other types are errors (explicit list + wildcard for future variants)
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::Keyword(_)
        | Value::String(_)
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

    Ok(Value::List(result))
}

/// `(list & args)` - create list from arguments.
///
/// Creates a new list containing all arguments in order.
/// Accepts any number of arguments (including zero).
#[inline]
pub fn native_list(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let list = List::from_vec(args.to_vec());
    Ok(Value::List(list))
}

/// `(concat & seqs)` - concatenate sequences into a list.
///
/// Concatenates all arguments (which must be sequences) into a single list.
/// Accepts lists, vectors, maps, and nil (treated as empty sequence).
/// Maps are treated as sequences of `[key value]` vectors.
///
/// # Errors
///
/// Returns a type error if any argument is not a sequence type.
#[inline]
pub fn native_concat(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let mut result = Vec::new();
    for (idx, arg) in args.iter().enumerate() {
        match *arg {
            Value::List(ref list) => result.extend(list.iter().cloned()),
            Value::Vector(ref vec) => result.extend(vec.iter().cloned()),
            Value::Map(ref map) => {
                // Maps are sequences of [key value] vectors
                for (key, value) in map.iter() {
                    let entry = Vector::from_vec(alloc::vec![key.value().clone(), value.clone()]);
                    result.push(Value::Vector(entry));
                }
            }
            Value::Set(ref set) => {
                // Sets are sequences of their elements
                for item in set.iter() {
                    result.push(item.value().clone());
                }
            }
            Value::Nil => {} // Empty sequence, skip
            // All other types are errors (explicit list + wildcard for future variants)
            Value::Bool(_)
            | Value::Integer(_)
            | Value::Float(_)
            | Value::Ratio(_)
            | Value::Symbol(_)
            | Value::Keyword(_)
            | Value::String(_)
            | Value::Function(_)
            | Value::NativeFunction(_)
            | _ => {
                return Err(NativeError::TypeError {
                    expected: TypeExpectation::Sequence,
                    got: arg.kind(),
                    arg_index: u8::try_from(idx).unwrap_or(u8::MAX),
                });
            }
        }
    }
    Ok(Value::List(List::from_vec(result)))
}
