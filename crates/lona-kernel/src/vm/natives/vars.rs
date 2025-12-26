// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Var operation native functions (var-get, var-set!).
//!
//! Provides programmatic access to Var values for metaprogramming and tooling.
//! These functions enable runtime manipulation of global bindings.

use alloc::vec::Vec;

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::symbol;
use lona_core::value::{self, Value};

use super::{NativeContext, NativeError, NativeFn};

/// The names of all var operation primitives.
pub const VAR_PRIMITIVE_NAMES: &[&str] = &["var-get", "var-set!"];

/// Pre-interns all var primitive symbols.
#[inline]
pub fn intern_var_primitives(interner: &symbol::Interner) -> Vec<symbol::Id> {
    VAR_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up var primitive symbols from an immutable interner.
#[inline]
#[must_use]
pub fn lookup_var_primitives(interner: &symbol::Interner) -> Option<Vec<symbol::Id>> {
    VAR_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all var primitives with the VM.
#[inline]
pub fn register_var_primitives(vm: &mut crate::vm::interpreter::Vm<'_>, symbols: &[symbol::Id]) {
    let funcs: &[NativeFn] = &[native_var_get, native_var_set];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        vm.set_global(*sym, Value::NativeFunction(*sym));
    }
}

/// Native implementation of `var-get`.
///
/// Returns the current value bound to a var.
/// `(var-get #'x)` => the value of x
///
/// # Errors
///
/// Returns `NativeError::ArityMismatch` if not exactly one argument.
/// Returns `NativeError::TypeError` if argument is not a Var.
#[inline]
pub fn native_var_get(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let arg = args.first().ok_or(NativeError::Error("missing argument"))?;

    match *arg {
        Value::Var(ref var) => Ok(var.value()),
        // Non-var types produce type error (wildcard covers future variants)
        Value::Nil
        | Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::Keyword(_)
        | Value::String(_)
        | Value::List(_)
        | Value::Vector(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Binary(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Single(value::Kind::Var),
            got: arg.kind(),
            arg_index: 0_u8,
        }),
    }
}

/// Native implementation of `var-set!`.
///
/// Sets the root binding of a var and returns the new value.
/// `(var-set! #'x 42)` => 42 (and x is now bound to 42)
///
/// Note: Dynamic var enforcement (requiring :dynamic metadata) is deferred
/// to Task 1.3.7 when binding stacks are implemented.
///
/// # Errors
///
/// Returns `NativeError::ArityMismatch` if not exactly two arguments.
/// Returns `NativeError::TypeError` if first argument is not a Var.
#[inline]
pub fn native_var_set(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 2_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(2_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let var_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
    let new_value = args
        .get(1_usize)
        .ok_or(NativeError::Error("missing argument"))?;

    match *var_arg {
        Value::Var(ref var) => {
            let value = new_value.clone();
            var.set_value(value.clone());
            Ok(value)
        }
        // Non-var types produce type error (wildcard covers future variants)
        Value::Nil
        | Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Symbol(_)
        | Value::Keyword(_)
        | Value::String(_)
        | Value::List(_)
        | Value::Vector(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Binary(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Single(value::Kind::Var),
            got: var_arg.kind(),
            arg_index: 0_u8,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::integer::Integer;
    use lona_core::symbol::Interner;
    use lona_core::value::Var;

    #[test]
    fn var_get_returns_value() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let name = interner.intern("x");
        let var = Var::new(name, None, Value::Integer(Integer::from_i64(42)), None);

        let args = [Value::Var(var)];
        let result = native_var_get(&args, &ctx).unwrap();

        assert_eq!(result, Value::Integer(Integer::from_i64(42)));
    }

    #[test]
    fn var_get_arity_error_no_args() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let result = native_var_get(&[], &ctx);
        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(1_u8),
                got: 0_u8
            })
        ));
    }

    #[test]
    fn var_get_arity_error_too_many_args() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let name = interner.intern("x");
        let var = Var::new(name, None, Value::Nil, None);

        let args = [Value::Var(var.clone()), Value::Var(var)];
        let result = native_var_get(&args, &ctx);
        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(1_u8),
                got: 2_u8
            })
        ));
    }

    #[test]
    fn var_get_type_error_non_var() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let args = [Value::Integer(Integer::from_i64(42))];
        let result = native_var_get(&args, &ctx);

        match result {
            Err(NativeError::TypeError {
                expected,
                got,
                arg_index,
            }) => {
                assert_eq!(expected, TypeExpectation::Single(value::Kind::Var));
                assert_eq!(got, value::Kind::Integer);
                assert_eq!(arg_index, 0_u8);
            }
            other => panic!("Expected TypeError, got {:?}", other),
        }
    }

    #[test]
    fn var_set_updates_value() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let name = interner.intern("x");
        let var = Var::new(name, None, Value::Integer(Integer::from_i64(1)), None);

        let args = [
            Value::Var(var.clone()),
            Value::Integer(Integer::from_i64(100)),
        ];
        let result = native_var_set(&args, &ctx).unwrap();

        // Returns the new value
        assert_eq!(result, Value::Integer(Integer::from_i64(100)));

        // Var is actually updated
        assert_eq!(var.value(), Value::Integer(Integer::from_i64(100)));
    }

    #[test]
    fn var_set_returns_new_value() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let name = interner.intern("y");
        let var = Var::new(name, None, Value::Nil, None);

        let args = [Value::Var(var), Value::Integer(Integer::from_i64(999))];
        let result = native_var_set(&args, &ctx).unwrap();

        assert_eq!(result, Value::Integer(Integer::from_i64(999)));
    }

    #[test]
    fn var_set_arity_error_too_few() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let name = interner.intern("x");
        let var = Var::new(name, None, Value::Nil, None);

        let args = [Value::Var(var)];
        let result = native_var_set(&args, &ctx);
        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(2_u8),
                got: 1_u8
            })
        ));
    }

    #[test]
    fn var_set_arity_error_too_many() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let name = interner.intern("x");
        let var = Var::new(name, None, Value::Nil, None);

        let args = [Value::Var(var), Value::Nil, Value::Nil];
        let result = native_var_set(&args, &ctx);
        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(2_u8),
                got: 3_u8
            })
        ));
    }

    #[test]
    fn var_set_type_error_non_var() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let args = [Value::Integer(Integer::from_i64(42)), Value::Nil];
        let result = native_var_set(&args, &ctx);

        match result {
            Err(NativeError::TypeError {
                expected,
                got,
                arg_index,
            }) => {
                assert_eq!(expected, TypeExpectation::Single(value::Kind::Var));
                assert_eq!(got, value::Kind::Integer);
                assert_eq!(arg_index, 0_u8);
            }
            other => panic!("Expected TypeError, got {:?}", other),
        }
    }

    #[test]
    fn var_set_visible_through_clone() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let name = interner.intern("z");
        let var = Var::new(name, None, Value::Integer(Integer::from_i64(10)), None);
        let var_clone = var.clone();

        let args = [Value::Var(var), Value::Integer(Integer::from_i64(20))];
        let _result = native_var_set(&args, &ctx).unwrap();

        // The clone should see the new value (vars are shared)
        assert_eq!(var_clone.value(), Value::Integer(Integer::from_i64(20)));
    }
}
