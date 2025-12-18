// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Vector-related collection operations.
//!
//! Provides native implementations for vector manipulation:
//! - `vector` - create a vector from arguments
//! - `vec` - convert collection to vector

use lona_core::value::Value;
use lona_core::vector::Vector;

use super::type_name;
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
/// Accepts lists, vectors, and nil (produces empty vector).
///
/// # Errors
///
/// Returns a type error if the argument is not a list, vector, or nil.
/// Returns an arity error if called with more or fewer than one argument.
#[inline]
pub fn native_vec(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let &[ref collection] = args else {
        return Err(NativeError::ArityMismatch {
            expected: 1,
            got: args.len(),
        });
    };

    let result = match *collection {
        Value::Vector(ref vec) => vec.clone(),
        Value::List(ref list) => Vector::from_vec(list.iter().cloned().collect()),
        Value::Nil => Vector::empty(),
        // All other types are errors
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::String(_)
        | Value::Map(_)
        | Value::Function(_)
        | _ => {
            return Err(NativeError::TypeError {
                expected: "list, vector, or nil",
                got: type_name(collection),
                arg_index: 0,
            });
        }
    };

    Ok(Value::Vector(result))
}
