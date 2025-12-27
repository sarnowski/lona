// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Symbol operation native functions (symbol, gensym).

use alloc::vec::Vec;

use lona_core::error_context::ArityExpectation;
use lona_core::symbol;
use lona_core::value::{self, Value};

use super::{NativeContext, NativeError, NativeFn};

/// The names of all symbol operation primitives.
pub const SYMBOL_PRIMITIVE_NAMES: &[&str] = &["symbol", "gensym"];

/// Pre-interns all symbol primitive symbols.
#[inline]
pub fn intern_symbol_primitives(interner: &symbol::Interner) -> Vec<symbol::Id> {
    SYMBOL_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up symbol primitive symbols from an immutable interner.
#[inline]
#[must_use]
pub fn lookup_symbol_primitives(interner: &symbol::Interner) -> Option<Vec<symbol::Id>> {
    SYMBOL_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all symbol primitives with the VM.
#[inline]
pub fn register_symbol_primitives(vm: &mut crate::vm::interpreter::Vm<'_>, symbols: &[symbol::Id]) {
    let funcs: &[NativeFn] = &[native_symbol, native_gensym];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        // Register in lona.core namespace for auto-refer
        vm.register_core_primitive(*sym, Value::NativeFunction(*sym));
    }
}

/// Native implementation of `symbol`.
///
/// Creates/interns a symbol from a string name.
/// `(symbol "foo")` => foo
#[inline]
pub fn native_symbol(args: &[Value], ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
    let name = arg.as_string().ok_or_else(|| NativeError::TypeError {
        expected: lona_core::error_context::TypeExpectation::Single(value::Kind::String),
        got: arg.kind(),
        arg_index: 0_u8,
    })?;

    let id = ctx.interner().intern(name.as_str());
    Ok(Value::Symbol(value::Symbol::new(id)))
}

/// Native implementation of `gensym`.
///
/// Generates a unique symbol.
/// - `(gensym)` => `G__123`
/// - `(gensym "prefix")` => `prefix__123`
#[inline]
pub fn native_gensym(args: &[Value], ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    let prefix = match args.len() {
        0_usize => None,
        1_usize => {
            let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            let prefix_str = arg.as_string().ok_or_else(|| NativeError::TypeError {
                expected: lona_core::error_context::TypeExpectation::Single(value::Kind::String),
                got: arg.kind(),
                arg_index: 0_u8,
            })?;
            Some(prefix_str.as_str())
        }
        _ => {
            return Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Range {
                    min: 0_u8,
                    max: 1_u8,
                },
                got: u8::try_from(args.len()).unwrap_or(u8::MAX),
            });
        }
    };

    let id = ctx.interner().gensym(prefix);
    Ok(Value::Symbol(value::Symbol::new(id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::string::HeapStr;
    use lona_core::symbol::Interner;

    #[test]
    fn symbol_creates_symbol_from_string() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let args = [Value::String(HeapStr::new("foo"))];
        let result = native_symbol(&args, &ctx).unwrap();

        if let Value::Symbol(sym) = result {
            assert_eq!(interner.resolve(sym.id()), "foo");
        } else {
            panic!("Expected symbol");
        }
    }

    #[test]
    fn symbol_arity_error_no_args() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let result = native_symbol(&[], &ctx);
        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(1_u8),
                got: 0_u8
            })
        ));
    }

    #[test]
    fn symbol_type_error() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let args = [Value::Integer(lona_core::integer::Integer::from_i64(42))];
        let result = native_symbol(&args, &ctx);
        assert!(matches!(result, Err(NativeError::TypeError { .. })));
    }

    #[test]
    fn gensym_no_args() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let result = native_gensym(&[], &ctx).unwrap();
        if let Value::Symbol(sym) = result {
            let name = interner.resolve(sym.id());
            assert!(
                name.starts_with("G__"),
                "name should start with G__: {name}"
            );
        } else {
            panic!("Expected symbol");
        }
    }

    #[test]
    fn gensym_with_prefix() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let args = [Value::String(HeapStr::new("temp"))];
        let result = native_gensym(&args, &ctx).unwrap();
        if let Value::Symbol(sym) = result {
            let name = interner.resolve(sym.id());
            assert!(
                name.starts_with("temp__"),
                "name should start with temp__: {name}"
            );
        } else {
            panic!("Expected symbol");
        }
    }

    #[test]
    fn gensym_generates_unique() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let result1 = native_gensym(&[], &ctx).unwrap();
        let result2 = native_gensym(&[], &ctx).unwrap();

        assert_ne!(result1, result2);
    }

    #[test]
    fn gensym_arity_error_too_many() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let args = [
            Value::String(HeapStr::new("a")),
            Value::String(HeapStr::new("b")),
        ];
        let result = native_gensym(&args, &ctx);
        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Range {
                    min: 0_u8,
                    max: 1_u8
                },
                ..
            })
        ));
    }

    #[test]
    fn gensym_type_error() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);

        let args = [Value::Integer(lona_core::integer::Integer::from_i64(42))];
        let result = native_gensym(&args, &ctx);
        assert!(matches!(result, Err(NativeError::TypeError { .. })));
    }
}
