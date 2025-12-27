// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Namespace operation native functions.
//!
//! These primitives support the namespace loading and import system:
//! - `require` - loads a namespace if not already loaded
//! - `namespace-add-alias` - adds an alias to the current namespace
//! - `namespace-add-refer` - adds a referred var to the current namespace
//! - `ns-publics` - returns a map of public vars in a namespace
//! - `namespace-use-all` - refers all public vars from a namespace
//!
//! Unlike regular native functions, these primitives need access to VM state
//! (namespace registry, source loader, etc.) and are implemented as VM natives.

use alloc::vec::Vec;

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::map::Map;
use lona_core::symbol::{self, Interner};
use lona_core::value::{self, Value, Var};

use super::NativeError;

// =============================================================================
// Helper Functions
// =============================================================================

/// Checks if a Var is marked as private via `:private true` metadata.
///
/// A var is considered private if its metadata contains the `:private` key
/// with a truthy value. This is used by `ns-publics` and `namespace-use-all`
/// to filter out private vars when exposing a namespace's public interface.
#[inline]
fn is_private_var(var: &Var, private_kw: symbol::Id) -> bool {
    var.meta().is_some_and(|meta| {
        meta.get(&Value::Keyword(private_kw))
            .is_some_and(Value::is_truthy)
    })
}

/// Extracts a symbol ID from a Value, returning a type error if not a symbol.
const fn expect_symbol(value: &Value, arg_index: u8) -> Result<symbol::Id, NativeError> {
    match *value {
        Value::Symbol(ref sym) => Ok(sym.id()),
        // All non-symbol types: exhaustive list + wildcard for non_exhaustive
        Value::Nil
        | Value::Bool(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Ratio(_)
        | Value::Keyword(_)
        | Value::String(_)
        | Value::List(_)
        | Value::Vector(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Binary(_)
        | Value::Function(_)
        | Value::NativeFunction(_)
        | Value::Var(_)
        | _ => Err(NativeError::TypeError {
            expected: TypeExpectation::Single(value::Kind::Symbol),
            got: value.kind(),
            arg_index,
        }),
    }
}

/// Extracts a Var from a Value, returning a type error if not a Var.
fn expect_var(value: &Value, arg_index: u8) -> Result<value::Var, NativeError> {
    match *value {
        Value::Var(ref var) => Ok(var.clone()),
        // All non-Var types: exhaustive list + wildcard for non_exhaustive
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
            got: value.kind(),
            arg_index,
        }),
    }
}

// =============================================================================
// Primitive Registration
// =============================================================================

/// The names of all namespace operation primitives.
pub const NAMESPACE_PRIMITIVE_NAMES: &[&str] = &[
    "require",
    "namespace-add-alias",
    "namespace-add-refer",
    "ns-publics",
    "namespace-use-all",
];

/// Pre-interns all namespace primitive symbols.
#[inline]
pub fn intern_namespace_primitives(interner: &Interner) -> Vec<symbol::Id> {
    NAMESPACE_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up namespace primitive symbols from an immutable interner.
#[inline]
#[must_use]
pub fn lookup_namespace_primitives(interner: &Interner) -> Option<Vec<symbol::Id>> {
    NAMESPACE_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all namespace primitives with the VM.
///
/// This registers the VM-native namespace primitives which require VM state access.
#[inline]
pub fn register_namespace_primitives(vm: &mut crate::vm::Vm<'_>, symbols: &[symbol::Id]) {
    use super::VmNativeFn;

    let funcs: &[VmNativeFn] = &[
        native_require,
        native_namespace_add_alias,
        native_namespace_add_refer,
        native_ns_publics,
        native_namespace_use_all,
    ];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_vm_native(*sym, *func);
        // Register in lona.core namespace for auto-refer
        vm.register_core_primitive(*sym, Value::NativeFunction(*sym));
    }
}

/// Native implementation of `require`.
///
/// Loads a namespace if not already loaded.
/// `(require 'bar)` => nil
///
/// # Errors
///
/// Returns error if:
/// - Wrong number of arguments (expects 1)
/// - Argument is not a symbol
/// - Circular dependency detected
/// - Namespace source not found
#[inline]
pub fn native_require(
    vm: &mut crate::vm::Vm<'_>,
    args: &[Value],
) -> Result<Value, crate::vm::Error> {
    use crate::vm::error::Kind as ErrorKind;
    use lona_core::source;
    use lona_core::span::Span;

    if args.len() != 1_usize {
        return Err(crate::vm::Error::new(
            ErrorKind::Native {
                error: NativeError::ArityMismatch {
                    expected: ArityExpectation::Exact(1_u8),
                    got: u8::try_from(args.len()).unwrap_or(u8::MAX),
                },
            },
            source::Location::new(vm.current_source(), Span::default()),
        ));
    }

    let make_error = |err| {
        crate::vm::Error::new(
            ErrorKind::Native { error: err },
            source::Location::new(vm.current_source(), Span::default()),
        )
    };

    let arg = args
        .first()
        .ok_or_else(|| make_error(NativeError::Error("missing argument")))?;
    let ns_sym = expect_symbol(arg, 0).map_err(make_error)?;

    // Use prepare_require to check if loading is needed and get source
    let source_opt = vm.prepare_require(ns_sym)?;

    let Some(source_code) = source_opt else {
        // Namespace already loaded, nothing to do
        return Ok(Value::Nil);
    };

    // Push to loading stack before compilation
    vm.push_loading(ns_sym);

    // Compile and execute the source
    // Note: We don't save/restore current namespace because the loaded
    // namespace's `ns` form will set the current namespace appropriately.
    // The namespace should stay as declared by the loaded code.

    // Use the namespace symbol ID as source ID for unique identification.
    // Symbol IDs are unique per interner, avoiding collisions.
    let ns_source_id = source::Id::new(ns_sym.as_u32());

    // Compile the source
    // TODO(Phase 4): Use compile_with_expansion for macro support.
    // Currently namespaces loaded via require cannot define or use macros.
    // This requires the VM to own a mutable MacroRegistry, which is a
    // significant architectural change deferred to the compiler integration phase.
    let compile_result = lonala_compiler::compile(source_code, ns_source_id, vm.interner());

    let chunk = match compile_result {
        Ok(chunk) => chunk,
        Err(err) => {
            // Pop from loading stack before returning error
            vm.pop_loading();
            return Err(crate::vm::Error::new(
                ErrorKind::CompileError { message: err },
                source::Location::new(ns_source_id, Span::default()),
            ));
        }
    };

    // Execute the compiled chunk
    let result = vm.execute_with_source(&chunk, ns_source_id);

    // Pop from loading stack
    vm.pop_loading();

    // On success, ensure namespace is recorded in registry.
    // The namespace's `ns` form should have created it via `SetNamespace`,
    // but we ensure it exists so `prepare_require` will recognize it as loaded.
    if result.is_ok() {
        let _ns = vm.namespace_registry_mut().get_or_create(ns_sym);
    }

    // Return nil on success, propagate error on failure
    result.map(|_value| Value::Nil)
}

/// Native implementation of `namespace-add-alias`.
///
/// Adds an alias to the current namespace.
/// `(namespace-add-alias 'b 'bar)` => nil
///
/// After this call, `b/x` in the current namespace will resolve to `bar/x`.
///
/// # Errors
///
/// Returns error if:
/// - Wrong number of arguments (expects 2)
/// - Arguments are not symbols
#[inline]
pub fn native_namespace_add_alias(
    vm: &mut crate::vm::Vm<'_>,
    args: &[Value],
) -> Result<Value, crate::vm::Error> {
    use crate::vm::error::Kind as ErrorKind;
    use lona_core::source;
    use lona_core::span::Span;

    if args.len() != 2_usize {
        return Err(crate::vm::Error::new(
            ErrorKind::Native {
                error: NativeError::ArityMismatch {
                    expected: ArityExpectation::Exact(2_u8),
                    got: u8::try_from(args.len()).unwrap_or(u8::MAX),
                },
            },
            source::Location::new(vm.current_source(), Span::default()),
        ));
    }

    let make_error = |err| {
        crate::vm::Error::new(
            ErrorKind::Native { error: err },
            source::Location::new(vm.current_source(), Span::default()),
        )
    };

    let alias_arg = args
        .first()
        .ok_or_else(|| make_error(NativeError::Error("missing argument")))?;
    let ns_arg = args
        .get(1)
        .ok_or_else(|| make_error(NativeError::Error("missing argument")))?;
    let alias = expect_symbol(alias_arg, 0).map_err(make_error)?;
    let ns_name = expect_symbol(ns_arg, 1).map_err(make_error)?;

    // Add alias to current namespace
    if let Some(ns) = vm.namespace_registry_mut().current_mut() {
        ns.add_alias(alias, ns_name);
    }

    Ok(Value::Nil)
}

/// Native implementation of `namespace-add-refer`.
///
/// Adds a referred var to the current namespace.
/// `(namespace-add-refer 'x #'bar/x)` => nil
///
/// After this call, `x` in the current namespace will resolve to the given var.
///
/// # Errors
///
/// Returns error if:
/// - Wrong number of arguments (expects 2)
/// - First argument is not a symbol
/// - Second argument is not a var
#[inline]
pub fn native_namespace_add_refer(
    vm: &mut crate::vm::Vm<'_>,
    args: &[Value],
) -> Result<Value, crate::vm::Error> {
    use crate::vm::error::Kind as ErrorKind;
    use lona_core::source;
    use lona_core::span::Span;

    if args.len() != 2_usize {
        return Err(crate::vm::Error::new(
            ErrorKind::Native {
                error: NativeError::ArityMismatch {
                    expected: ArityExpectation::Exact(2_u8),
                    got: u8::try_from(args.len()).unwrap_or(u8::MAX),
                },
            },
            source::Location::new(vm.current_source(), Span::default()),
        ));
    }

    let make_error = |err| {
        crate::vm::Error::new(
            ErrorKind::Native { error: err },
            source::Location::new(vm.current_source(), Span::default()),
        )
    };

    let sym_arg = args
        .first()
        .ok_or_else(|| make_error(NativeError::Error("missing argument")))?;
    let var_arg = args
        .get(1)
        .ok_or_else(|| make_error(NativeError::Error("missing argument")))?;
    let sym = expect_symbol(sym_arg, 0).map_err(make_error)?;
    let var = expect_var(var_arg, 1).map_err(make_error)?;

    // Add refer to current namespace
    if let Some(ns) = vm.namespace_registry_mut().current_mut() {
        ns.add_refer(sym, var);
    }

    Ok(Value::Nil)
}

/// Native implementation of `ns-publics`.
///
/// Returns a map of public vars in a namespace.
/// `(ns-publics 'bar)` => {x #'bar/x, y #'bar/y, ...}
///
/// Private vars (those with `:private true` metadata) are excluded.
///
/// # Errors
///
/// Returns error if:
/// - Wrong number of arguments (expects 1)
/// - Argument is not a symbol
#[inline]
pub fn native_ns_publics(
    vm: &mut crate::vm::Vm<'_>,
    args: &[Value],
) -> Result<Value, crate::vm::Error> {
    use crate::vm::error::Kind as ErrorKind;
    use lona_core::source;
    use lona_core::span::Span;

    if args.len() != 1_usize {
        return Err(crate::vm::Error::new(
            ErrorKind::Native {
                error: NativeError::ArityMismatch {
                    expected: ArityExpectation::Exact(1_u8),
                    got: u8::try_from(args.len()).unwrap_or(u8::MAX),
                },
            },
            source::Location::new(vm.current_source(), Span::default()),
        ));
    }

    let make_error = |err| {
        crate::vm::Error::new(
            ErrorKind::Native { error: err },
            source::Location::new(vm.current_source(), Span::default()),
        )
    };

    let arg = args
        .first()
        .ok_or_else(|| make_error(NativeError::Error("missing argument")))?;
    let ns_name = expect_symbol(arg, 0).map_err(make_error)?;

    // Get the namespace
    let Some(ns) = vm.namespace_registry().get(ns_name) else {
        // Namespace not found, return nil
        return Ok(Value::Nil);
    };

    // Build map of non-private vars
    let private_kw = vm.interner().intern("private");

    let mut result_map = Map::empty();
    for (sym, var) in ns.mappings() {
        if !is_private_var(var, private_kw) {
            let key = Value::Symbol(value::Symbol::new(*sym));
            let val = Value::Var(var.clone());
            result_map = result_map.assoc(key, val);
        }
    }

    Ok(Value::Map(result_map))
}

/// Native implementation of `namespace-use-all`.
///
/// Refers all public vars from a namespace into the current namespace.
/// `(namespace-use-all 'bar)` => nil
///
/// This is equivalent to calling `(namespace-add-refer sym var)` for each
/// public var in the namespace. It's used to implement `:use` clauses.
///
/// # Errors
///
/// Returns error if:
/// - Wrong number of arguments (expects 1)
/// - Argument is not a symbol
#[inline]
pub fn native_namespace_use_all(
    vm: &mut crate::vm::Vm<'_>,
    args: &[Value],
) -> Result<Value, crate::vm::Error> {
    use crate::vm::error::Kind as ErrorKind;
    use lona_core::source;
    use lona_core::span::Span;

    if args.len() != 1_usize {
        return Err(crate::vm::Error::new(
            ErrorKind::Native {
                error: NativeError::ArityMismatch {
                    expected: ArityExpectation::Exact(1_u8),
                    got: u8::try_from(args.len()).unwrap_or(u8::MAX),
                },
            },
            source::Location::new(vm.current_source(), Span::default()),
        ));
    }

    let make_error = |err| {
        crate::vm::Error::new(
            ErrorKind::Native { error: err },
            source::Location::new(vm.current_source(), Span::default()),
        )
    };

    let arg = args
        .first()
        .ok_or_else(|| make_error(NativeError::Error("missing argument")))?;
    let ns_name = expect_symbol(arg, 0).map_err(make_error)?;

    // Get the namespace
    let Some(ns) = vm.namespace_registry().get(ns_name) else {
        // Namespace not found, return nil (nothing to refer)
        return Ok(Value::Nil);
    };

    // Get all public vars from the namespace
    let private_kw = vm.interner().intern("private");

    // Collect public vars first to avoid borrow issues
    let public_vars: alloc::vec::Vec<(symbol::Id, Var)> = ns
        .mappings()
        .filter_map(|(sym, var)| {
            if is_private_var(var, private_kw) {
                None
            } else {
                Some((*sym, var.clone()))
            }
        })
        .collect();

    // Add each public var as a refer to the current namespace
    for (sym, var) in public_vars {
        if let Some(current_ns) = vm.namespace_registry_mut().current_mut() {
            current_ns.add_refer(sym, var);
        }
    }

    Ok(Value::Nil)
}
