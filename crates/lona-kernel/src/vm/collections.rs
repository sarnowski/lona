// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Collection primitive functions for the Lonala language.
//!
//! Provides native implementations of core collection operations:
//! - `cons` - prepend element to a collection
//! - `first` - get first element of a collection
//! - `rest` - get rest of a collection (tail)
//! - `vector` - create a vector from arguments
//! - `hash-map` - create a map from key-value pairs
//! - `list` - create a list from arguments
//! - `concat` - concatenate sequences into a list
//! - `vec` - convert collection to vector
//!
//! # Registration Pattern
//!
//! These primitives use a two-phase registration pattern to avoid borrow conflicts:
//!
//! 1. Call [`intern_primitives`] with `&mut Interner` to intern symbol names
//! 2. Create the VM with `Vm::new(&interner)` (immutable borrow)
//! 3. Call [`register_primitives`] with the VM and the symbols from step 1
//!
//! This pattern is necessary because the VM holds an immutable reference to the
//! interner, preventing mutable access during registration.

use alloc::vec::Vec;

use lona_core::list::List;
use lona_core::map::Map;
use lona_core::value::Value;
use lona_core::vector::Vector;

use lona_core::symbol;

use super::interpreter::Vm;
use super::natives::{NativeContext, NativeError};

/// Returns a type name for error messages.
const fn type_name(value: &Value) -> &'static str {
    match *value {
        Value::Nil => "nil",
        Value::Bool(_) => "boolean",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::Ratio(_) => "ratio",
        Value::Symbol(_) => "symbol",
        Value::String(_) => "string",
        Value::List(_) => "list",
        Value::Vector(_) => "vector",
        Value::Map(_) => "map",
        Value::Function(_) => "function",
        // Value is non-exhaustive - handle future variants
        _ => "unknown",
    }
}

/// `(cons x coll)` - prepend x to collection, returns list.
///
/// Prepends the first argument to the collection in the second argument.
/// Always returns a list (following Clojure semantics).
///
/// # Type handling
///
/// - List: New list with x prepended
/// - Vector: Convert vector to list, prepend x
/// - nil: Single-element list (x)
/// - Other: `TypeError`
#[inline]
pub fn native_cons(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let &[ref element, ref collection] = args else {
        return Err(NativeError::ArityMismatch {
            expected: 2,
            got: args.len(),
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
        Value::Nil => List::empty().cons(element.clone()),
        // All other types are errors (explicit list + wildcard for future variants)
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
                arg_index: 1,
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
            expected: 1,
            got: args.len(),
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
        Value::Nil => Value::Nil,
        // All other types are errors (explicit list + wildcard for future variants)
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::String(_)
        | Value::Function(_)
        | _ => {
            return Err(NativeError::TypeError {
                expected: "list, vector, map, or nil",
                got: type_name(collection),
                arg_index: 0,
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
            expected: 1,
            got: args.len(),
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
        Value::Nil => List::empty(),
        // All other types are errors (explicit list + wildcard for future variants)
        Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::String(_)
        | Value::Function(_)
        | _ => {
            return Err(NativeError::TypeError {
                expected: "list, vector, map, or nil",
                got: type_name(collection),
                arg_index: 0,
            });
        }
    };

    Ok(Value::List(result))
}

/// `(vector & args)` - create vector from arguments.
///
/// Creates a new vector containing all arguments in order.
/// Accepts any number of arguments (including zero).
#[inline]
pub fn native_vector(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let vec = Vector::from_vec(args.to_vec());
    Ok(Value::Vector(vec))
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
/// Accepts lists, vectors, and nil (treated as empty sequence).
///
/// # Errors
///
/// Returns a type error if any argument is not a list, vector, or nil.
#[inline]
pub fn native_concat(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let mut result = Vec::new();
    for (idx, arg) in args.iter().enumerate() {
        match *arg {
            Value::List(ref list) => result.extend(list.iter().cloned()),
            Value::Vector(ref vec) => result.extend(vec.iter().cloned()),
            Value::Nil => {} // Empty sequence, skip
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
                    got: type_name(arg),
                    arg_index: idx,
                });
            }
        }
    }
    Ok(Value::List(List::from_vec(result)))
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

/// `(hash-map & kvs)` - create map from key-value pairs.
///
/// Creates a new map from alternating key-value pairs.
/// Requires an even number of arguments.
///
/// # Errors
///
/// Returns an error if an odd number of arguments is provided.
#[inline]
pub fn native_hash_map(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if !args.len().is_multiple_of(2) {
        return Err(NativeError::Error(
            "hash-map requires even number of arguments",
        ));
    }

    let pairs = args
        .chunks_exact(2_usize)
        .map(|chunk| {
            // chunks_exact(2) guarantees exactly 2 elements per chunk
            let &[ref key, ref value] = chunk else {
                // Cannot happen due to chunks_exact guarantee
                return Err(NativeError::Error("internal error: invalid chunk size"));
            };
            Ok((key.clone(), value.clone()))
        })
        .collect::<Result<Vec<_>, NativeError>>()?;

    let map = Map::from_pairs(pairs);
    Ok(Value::Map(map))
}

/// The names of all collection primitives.
pub const PRIMITIVE_NAMES: &[&str] = &[
    "cons", "first", "rest", "vector", "hash-map", "list", "concat", "vec",
];

/// Pre-interns all collection primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `PRIMITIVE_NAMES`.
#[inline]
pub fn intern_primitives(interner: &mut symbol::Interner) -> alloc::vec::Vec<symbol::Id> {
    PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up collection primitive symbols from an immutable interner.
///
/// This is used when primitives should already be interned (e.g., by the REPL)
/// and we only have an immutable reference to the interner.
///
/// Returns `Some(symbols)` if all primitives are found, `None` otherwise.
#[inline]
#[must_use]
pub fn lookup_primitives(interner: &symbol::Interner) -> Option<alloc::vec::Vec<symbol::Id>> {
    PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all collection primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_primitives` with the same interner.
#[inline]
pub fn register_primitives(vm: &mut Vm<'_>, symbols: &[symbol::Id]) {
    let funcs: &[super::natives::NativeFn] = &[
        native_cons,
        native_first,
        native_rest,
        native_vector,
        native_hash_map,
        native_list,
        native_concat,
        native_vec,
    ];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        vm.set_global(*sym, Value::Symbol(*sym));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::integer::Integer;
    use lona_core::string::HeapStr;
    use lona_core::symbol::Interner;

    /// Helper to create an integer value.
    fn int(value: i64) -> Value {
        Value::Integer(Integer::from_i64(value))
    }

    /// Helper to create a string value.
    fn string(text: &str) -> Value {
        Value::String(HeapStr::new(text))
    }

    /// Helper to create a native context for testing.
    fn ctx(interner: &Interner) -> NativeContext<'_> {
        NativeContext::new(interner, None)
    }

    // =========================================================================
    // cons tests
    // =========================================================================

    #[test]
    fn cons_to_list() {
        let interner = Interner::new();
        let list = List::from_vec(alloc::vec![int(2), int(3)]);
        let args = alloc::vec![int(1), Value::List(list)];

        let result = native_cons(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert_eq!(list.len(), 3);
            assert_eq!(list.first(), Some(&int(1)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn cons_to_vector() {
        let interner = Interner::new();
        let vec = Vector::from_vec(alloc::vec![int(2), int(3)]);
        let args = alloc::vec![int(1), Value::Vector(vec)];

        let result = native_cons(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert_eq!(list.len(), 3);
            assert_eq!(list.first(), Some(&int(1)));
            // Verify rest is (2 3)
            let rest = list.rest();
            assert_eq!(rest.first(), Some(&int(2)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn cons_to_nil() {
        let interner = Interner::new();
        let args = alloc::vec![int(1), Value::Nil];

        let result = native_cons(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert_eq!(list.len(), 1);
            assert_eq!(list.first(), Some(&int(1)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn cons_type_error() {
        let interner = Interner::new();
        let args = alloc::vec![int(1), int(2)];

        let result = native_cons(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::TypeError {
                expected: "list, vector, or nil",
                got: "integer",
                arg_index: 1
            })
        ));
    }

    #[test]
    fn cons_type_error_map() {
        let interner = Interner::new();
        // cons with a map should produce type error (unlike first/rest which support maps)
        let map = Map::from_pairs(alloc::vec![(string("a"), int(1))]);
        let args = alloc::vec![int(1), Value::Map(map)];

        let result = native_cons(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::TypeError {
                expected: "list, vector, or nil",
                got: "map",
                arg_index: 1
            })
        ));
    }

    #[test]
    fn cons_arity_error() {
        let interner = Interner::new();
        let args = alloc::vec![int(1)];

        let result = native_cons(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: 2,
                got: 1
            })
        ));
    }

    // =========================================================================
    // first tests
    // =========================================================================

    #[test]
    fn first_of_list() {
        let interner = Interner::new();
        let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let args = alloc::vec![Value::List(list)];

        let result = native_first(&args, &ctx(&interner)).unwrap();
        assert_eq!(result, int(1));
    }

    #[test]
    fn first_of_empty_list() {
        let interner = Interner::new();
        let list = List::empty();
        let args = alloc::vec![Value::List(list)];

        let result = native_first(&args, &ctx(&interner)).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn first_of_vector() {
        let interner = Interner::new();
        let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let args = alloc::vec![Value::Vector(vec)];

        let result = native_first(&args, &ctx(&interner)).unwrap();
        assert_eq!(result, int(1));
    }

    #[test]
    fn first_of_empty_vector() {
        let interner = Interner::new();
        let vec = Vector::empty();
        let args = alloc::vec![Value::Vector(vec)];

        let result = native_first(&args, &ctx(&interner)).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn first_of_map() {
        let interner = Interner::new();
        let map = Map::from_pairs(alloc::vec![(string("a"), int(1))]);
        let args = alloc::vec![Value::Map(map)];

        let result = native_first(&args, &ctx(&interner)).unwrap();

        // Should be a vector [key value]
        if let Value::Vector(vec) = result {
            assert_eq!(vec.len(), 2);
        } else {
            panic!("Expected Vector");
        }
    }

    #[test]
    fn first_of_empty_map() {
        let interner = Interner::new();
        let map = Map::empty();
        let args = alloc::vec![Value::Map(map)];

        let result = native_first(&args, &ctx(&interner)).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn first_of_nil() {
        let interner = Interner::new();
        let args = alloc::vec![Value::Nil];

        let result = native_first(&args, &ctx(&interner)).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn first_type_error() {
        let interner = Interner::new();
        let args = alloc::vec![int(42)];

        let result = native_first(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::TypeError {
                expected: "list, vector, map, or nil",
                got: "integer",
                arg_index: 0
            })
        ));
    }

    #[test]
    fn first_arity_error() {
        let interner = Interner::new();
        let args = alloc::vec![];

        let result = native_first(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: 1,
                got: 0
            })
        ));
    }

    // =========================================================================
    // rest tests
    // =========================================================================

    #[test]
    fn rest_of_list() {
        let interner = Interner::new();
        let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let args = alloc::vec![Value::List(list)];

        let result = native_rest(&args, &ctx(&interner)).unwrap();

        if let Value::List(rest_list) = result {
            assert_eq!(rest_list.len(), 2);
            assert_eq!(rest_list.first(), Some(&int(2)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn rest_of_single_element_list() {
        let interner = Interner::new();
        let list = List::from_vec(alloc::vec![int(1)]);
        let args = alloc::vec![Value::List(list)];

        let result = native_rest(&args, &ctx(&interner)).unwrap();

        if let Value::List(rest_list) = result {
            assert!(rest_list.is_empty());
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn rest_of_empty_list() {
        let interner = Interner::new();
        let list = List::empty();
        let args = alloc::vec![Value::List(list)];

        let result = native_rest(&args, &ctx(&interner)).unwrap();

        if let Value::List(rest_list) = result {
            assert!(rest_list.is_empty());
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn rest_of_vector() {
        let interner = Interner::new();
        let vec = Vector::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let args = alloc::vec![Value::Vector(vec)];

        let result = native_rest(&args, &ctx(&interner)).unwrap();

        if let Value::List(rest_list) = result {
            assert_eq!(rest_list.len(), 2);
            assert_eq!(rest_list.first(), Some(&int(2)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn rest_of_empty_vector() {
        let interner = Interner::new();
        let vec = Vector::empty();
        let args = alloc::vec![Value::Vector(vec)];

        let result = native_rest(&args, &ctx(&interner)).unwrap();

        if let Value::List(rest_list) = result {
            assert!(rest_list.is_empty());
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn rest_of_nil() {
        let interner = Interner::new();
        let args = alloc::vec![Value::Nil];

        let result = native_rest(&args, &ctx(&interner)).unwrap();

        if let Value::List(rest_list) = result {
            assert!(rest_list.is_empty());
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn rest_of_map() {
        let interner = Interner::new();
        // Map with two entries
        let map = Map::from_pairs(alloc::vec![(string("a"), int(1)), (string("b"), int(2)),]);
        let args = alloc::vec![Value::Map(map)];

        let result = native_rest(&args, &ctx(&interner)).unwrap();

        // Should return a list with one [key value] vector (the second entry)
        if let Value::List(rest_list) = result {
            assert_eq!(rest_list.len(), 1);
            // The single entry should be a vector
            if let Some(Value::Vector(entry)) = rest_list.first() {
                assert_eq!(entry.len(), 2);
            } else {
                panic!("Expected Vector entry");
            }
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn rest_arity_error() {
        let interner = Interner::new();
        let args = alloc::vec![];

        let result = native_rest(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: 1,
                got: 0
            })
        ));
    }

    #[test]
    fn rest_type_error() {
        let interner = Interner::new();
        let args = alloc::vec![int(42)];

        let result = native_rest(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::TypeError {
                expected: "list, vector, map, or nil",
                got: "integer",
                arg_index: 0
            })
        ));
    }

    // =========================================================================
    // vector tests
    // =========================================================================

    #[test]
    fn vector_empty() {
        let interner = Interner::new();
        let args: Vec<Value> = alloc::vec![];

        let result = native_vector(&args, &ctx(&interner)).unwrap();

        if let Value::Vector(vec) = result {
            assert!(vec.is_empty());
        } else {
            panic!("Expected Vector");
        }
    }

    #[test]
    fn vector_with_elements() {
        let interner = Interner::new();
        let args = alloc::vec![int(1), int(2), int(3)];

        let result = native_vector(&args, &ctx(&interner)).unwrap();

        if let Value::Vector(vec) = result {
            assert_eq!(vec.len(), 3);
            assert_eq!(vec.get(0_usize), Some(&int(1)));
            assert_eq!(vec.get(1_usize), Some(&int(2)));
            assert_eq!(vec.get(2_usize), Some(&int(3)));
        } else {
            panic!("Expected Vector");
        }
    }

    #[test]
    fn vector_preserves_order() {
        let interner = Interner::new();
        let args = alloc::vec![string("a"), string("b"), string("c")];

        let result = native_vector(&args, &ctx(&interner)).unwrap();

        if let Value::Vector(vec) = result {
            assert_eq!(vec.get(0_usize), Some(&string("a")));
            assert_eq!(vec.get(1_usize), Some(&string("b")));
            assert_eq!(vec.get(2_usize), Some(&string("c")));
        } else {
            panic!("Expected Vector");
        }
    }

    // =========================================================================
    // hash-map tests
    // =========================================================================

    #[test]
    fn hash_map_empty() {
        let interner = Interner::new();
        let args: Vec<Value> = alloc::vec![];

        let result = native_hash_map(&args, &ctx(&interner)).unwrap();

        if let Value::Map(map) = result {
            assert!(map.is_empty());
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn hash_map_with_pairs() {
        let interner = Interner::new();
        let args = alloc::vec![string("a"), int(1), string("b"), int(2)];

        let result = native_hash_map(&args, &ctx(&interner)).unwrap();

        if let Value::Map(map) = result {
            assert_eq!(map.len(), 2);
            assert_eq!(map.get(&string("a")), Some(&int(1)));
            assert_eq!(map.get(&string("b")), Some(&int(2)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn hash_map_odd_args_error() {
        let interner = Interner::new();
        let args = alloc::vec![string("a"), int(1), string("b")];

        let result = native_hash_map(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::Error(
                "hash-map requires even number of arguments"
            ))
        ));
    }

    #[test]
    fn hash_map_duplicate_keys() {
        let interner = Interner::new();
        // Later value wins
        let args = alloc::vec![string("a"), int(1), string("a"), int(2)];

        let result = native_hash_map(&args, &ctx(&interner)).unwrap();

        if let Value::Map(map) = result {
            // Only one entry since key is duplicated
            assert_eq!(map.len(), 1);
            // Later value should win
            assert_eq!(map.get(&string("a")), Some(&int(2)));
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn hash_map_mixed_key_types() {
        let interner = Interner::new();
        let args = alloc::vec![string("str"), int(1), int(42), int(2), Value::Nil, int(3)];

        let result = native_hash_map(&args, &ctx(&interner)).unwrap();

        if let Value::Map(map) = result {
            assert_eq!(map.len(), 3);
            assert_eq!(map.get(&string("str")), Some(&int(1)));
            assert_eq!(map.get(&int(42)), Some(&int(2)));
            assert_eq!(map.get(&Value::Nil), Some(&int(3)));
        } else {
            panic!("Expected Map");
        }
    }

    // =========================================================================
    // list tests
    // =========================================================================

    #[test]
    fn list_empty() {
        let interner = Interner::new();
        let args: Vec<Value> = alloc::vec![];

        let result = native_list(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert!(list.is_empty());
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn list_single_element() {
        let interner = Interner::new();
        let args = alloc::vec![int(42)];

        let result = native_list(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert_eq!(list.len(), 1);
            assert_eq!(list.first(), Some(&int(42)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn list_multiple_elements() {
        let interner = Interner::new();
        let args = alloc::vec![int(1), int(2), int(3)];

        let result = native_list(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert_eq!(list.len(), 3);
            assert_eq!(list.first(), Some(&int(1)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn list_nested() {
        let interner = Interner::new();
        let inner = List::from_vec(alloc::vec![int(1), int(2)]);
        let args = alloc::vec![Value::List(inner), int(3)];

        let result = native_list(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert_eq!(list.len(), 2);
        } else {
            panic!("Expected List");
        }
    }

    // =========================================================================
    // concat tests
    // =========================================================================

    #[test]
    fn concat_empty() {
        let interner = Interner::new();
        let args: Vec<Value> = alloc::vec![];

        let result = native_concat(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert!(list.is_empty());
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn concat_single_list() {
        let interner = Interner::new();
        let list = List::from_vec(alloc::vec![int(1), int(2)]);
        let args = alloc::vec![Value::List(list)];

        let result = native_concat(&args, &ctx(&interner)).unwrap();

        if let Value::List(result_list) = result {
            assert_eq!(result_list.len(), 2);
            assert_eq!(result_list.first(), Some(&int(1)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn concat_single_vector() {
        let interner = Interner::new();
        let vec = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let args = alloc::vec![Value::Vector(vec)];

        let result = native_concat(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert_eq!(list.len(), 2);
            assert_eq!(list.first(), Some(&int(1)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn concat_multiple_lists() {
        let interner = Interner::new();
        let list1 = List::from_vec(alloc::vec![int(1), int(2)]);
        let list2 = List::from_vec(alloc::vec![int(3), int(4)]);
        let args = alloc::vec![Value::List(list1), Value::List(list2)];

        let result = native_concat(&args, &ctx(&interner)).unwrap();

        if let Value::List(list) = result {
            assert_eq!(list.len(), 4);
            assert_eq!(list.first(), Some(&int(1)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn concat_mixed_types() {
        let interner = Interner::new();
        let list = List::from_vec(alloc::vec![int(1), int(2)]);
        let vec = Vector::from_vec(alloc::vec![int(3), int(4)]);
        let args = alloc::vec![Value::List(list), Value::Vector(vec)];

        let result = native_concat(&args, &ctx(&interner)).unwrap();

        if let Value::List(result_list) = result {
            assert_eq!(result_list.len(), 4);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn concat_with_nil() {
        let interner = Interner::new();
        let list = List::from_vec(alloc::vec![int(1), int(2)]);
        let args = alloc::vec![Value::Nil, Value::List(list), Value::Nil];

        let result = native_concat(&args, &ctx(&interner)).unwrap();

        if let Value::List(result_list) = result {
            assert_eq!(result_list.len(), 2);
            assert_eq!(result_list.first(), Some(&int(1)));
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn concat_type_error() {
        let interner = Interner::new();
        let args = alloc::vec![int(42)];

        let result = native_concat(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::TypeError {
                expected: "list, vector, or nil",
                got: "integer",
                arg_index: 0
            })
        ));
    }

    #[test]
    fn concat_type_error_second_arg() {
        let interner = Interner::new();
        let list = List::from_vec(alloc::vec![int(1)]);
        let args = alloc::vec![Value::List(list), int(42)];

        let result = native_concat(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::TypeError {
                expected: "list, vector, or nil",
                got: "integer",
                arg_index: 1
            })
        ));
    }

    // =========================================================================
    // vec tests
    // =========================================================================

    #[test]
    fn vec_from_nil() {
        let interner = Interner::new();
        let args = alloc::vec![Value::Nil];

        let result = native_vec(&args, &ctx(&interner)).unwrap();

        if let Value::Vector(vec) = result {
            assert!(vec.is_empty());
        } else {
            panic!("Expected Vector");
        }
    }

    #[test]
    fn vec_from_list() {
        let interner = Interner::new();
        let list = List::from_vec(alloc::vec![int(1), int(2), int(3)]);
        let args = alloc::vec![Value::List(list)];

        let result = native_vec(&args, &ctx(&interner)).unwrap();

        if let Value::Vector(vec) = result {
            assert_eq!(vec.len(), 3);
            assert_eq!(vec.get(0_usize), Some(&int(1)));
            assert_eq!(vec.get(1_usize), Some(&int(2)));
            assert_eq!(vec.get(2_usize), Some(&int(3)));
        } else {
            panic!("Expected Vector");
        }
    }

    #[test]
    fn vec_from_vector() {
        let interner = Interner::new();
        let original = Vector::from_vec(alloc::vec![int(1), int(2)]);
        let args = alloc::vec![Value::Vector(original.clone())];

        let result = native_vec(&args, &ctx(&interner)).unwrap();

        if let Value::Vector(vec) = result {
            assert_eq!(vec.len(), 2);
            assert_eq!(vec, original);
        } else {
            panic!("Expected Vector");
        }
    }

    #[test]
    fn vec_type_error() {
        let interner = Interner::new();
        let args = alloc::vec![int(42)];

        let result = native_vec(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::TypeError {
                expected: "list, vector, or nil",
                got: "integer",
                arg_index: 0
            })
        ));
    }

    #[test]
    fn vec_arity_error_too_few() {
        let interner = Interner::new();
        let args: Vec<Value> = alloc::vec![];

        let result = native_vec(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: 1,
                got: 0
            })
        ));
    }

    #[test]
    fn vec_arity_error_too_many() {
        let interner = Interner::new();
        let args = alloc::vec![Value::Nil, Value::Nil];

        let result = native_vec(&args, &ctx(&interner));

        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: 1,
                got: 2
            })
        ));
    }
}
