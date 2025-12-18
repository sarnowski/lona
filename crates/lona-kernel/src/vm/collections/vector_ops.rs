// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Vector-related collection operations.
//!
//! Provides native implementations for vector manipulation:
//! - `vector` - create a vector from arguments
//! - `vec` - convert collection to vector

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::value::Value;
use lona_core::vector::Vector;

use crate::vm::natives::{NativeContext, NativeError};

/// `(vector & args)` - create vector from arguments.
///
/// Creates a new vector containing all arguments in order.
/// Accepts any number of arguments (including zero).
#[inline]
pub fn native_vector(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let vec = Vector::from_vec(args.to_vec());
    Ok(Value::Vector(vec))
}

/// `(vec coll)` - convert collection to vector.
///
/// Creates a new vector from a collection.
/// Accepts lists, vectors, maps, and nil (produces empty vector).
/// Maps are converted to vectors of `[key value]` vectors.
///
/// # Errors
///
/// Returns a type error if the argument is not a sequence type.
/// Returns an arity error if called with more or fewer than one argument.
#[inline]
pub fn native_vec(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let &[ref collection] = args else {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    };

    let result = match *collection {
        Value::Vector(ref vec) => vec.clone(),
        Value::List(ref list) => Vector::from_vec(list.iter().cloned().collect()),
        Value::Map(ref map) => {
            // Maps are sequences of [key value] vectors
            let entries: alloc::vec::Vec<_> = map
                .iter()
                .map(|(key, value)| {
                    let entry = Vector::from_vec(alloc::vec![key.value().clone(), value.clone()]);
                    Value::Vector(entry)
                })
                .collect();
            Vector::from_vec(entries)
        }
        Value::Nil => Vector::empty(),
        // All other types are errors
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::String(_)
        | Value::Function(_)
        | _ => {
            return Err(NativeError::TypeError {
                expected: TypeExpectation::Sequence,
                got: collection.kind(),
                arg_index: 0_u8,
            });
        }
    };

    Ok(Value::Vector(result))
}
