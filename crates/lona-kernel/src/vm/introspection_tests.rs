// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for macro introspection functions.

use alloc::string::String;
use alloc::sync::Arc;

use lona_core::chunk::Chunk;
use lona_core::integer::Integer;
use lona_core::list::List;
use lona_core::opcode::{Opcode, encode_abc};
use lona_core::span::Span;
use lona_core::symbol::Interner;
use lona_core::value::Value;
use lonala_compiler::{MacroDefinition, MacroRegistry};

use super::{native_expand_fully, native_expand_once, native_is_macro, values_equal};
use crate::vm::collections::intern_primitives;
use crate::vm::natives::NativeContext;

/// Creates a simple identity macro chunk (returns first argument).
fn make_identity_chunk() -> Chunk {
    let mut chunk = Chunk::with_name(String::from("test-macro"));
    chunk.set_arity(1_u8);
    // Return R[0] - the first argument
    chunk.emit(
        encode_abc(Opcode::Return, 0_u8, 1_u8, 0_u8),
        Span::new(0_usize, 1_usize),
    );
    chunk.set_max_registers(1_u8);
    chunk
}

/// Helper to create a native context with a macro registry.
fn ctx_with_macros<'a>(interner: &'a Interner, registry: &'a MacroRegistry) -> NativeContext<'a> {
    NativeContext::new(interner, Some(registry))
}

/// Helper to create a native context without a macro registry.
fn ctx_without_macros(interner: &Interner) -> NativeContext<'_> {
    NativeContext::new(interner, None)
}

#[test]
fn is_macro_returns_true_for_registered_macro() {
    let mut interner = Interner::new();
    let mut registry = MacroRegistry::new();

    let macro_name = interner.intern("my-macro");
    let chunk = Arc::new(make_identity_chunk());
    let def = MacroDefinition::single_arity(chunk, 1_u8, false, String::from("my-macro"));
    registry.register(macro_name, def);

    let ctx = ctx_with_macros(&interner, &registry);
    let args = [Value::Symbol(macro_name)];
    let result = native_is_macro(&args, &ctx).unwrap();

    assert_eq!(result, Value::Bool(true));
}

#[test]
fn is_macro_returns_false_for_unknown_symbol() {
    let mut interner = Interner::new();
    let registry = MacroRegistry::new();

    let unknown_sym = interner.intern("unknown");
    let ctx = ctx_with_macros(&interner, &registry);
    let args = [Value::Symbol(unknown_sym)];
    let result = native_is_macro(&args, &ctx).unwrap();

    assert_eq!(result, Value::Bool(false));
}

#[test]
fn is_macro_returns_false_without_registry() {
    let mut interner = Interner::new();

    let sym = interner.intern("anything");
    let ctx = ctx_without_macros(&interner);
    let args = [Value::Symbol(sym)];
    let result = native_is_macro(&args, &ctx).unwrap();

    assert_eq!(result, Value::Bool(false));
}

#[test]
fn is_macro_rejects_non_symbol() {
    let interner = Interner::new();
    let registry = MacroRegistry::new();

    let ctx = ctx_with_macros(&interner, &registry);
    let args = [Value::Integer(Integer::from_i64(42_i64))];
    let result = native_is_macro(&args, &ctx);

    assert!(result.is_err());
}

#[test]
fn is_macro_rejects_wrong_arity() {
    let mut interner = Interner::new();
    let registry = MacroRegistry::new();

    // No arguments
    {
        let ctx = ctx_with_macros(&interner, &registry);
        let result = native_is_macro(&[], &ctx);
        assert!(result.is_err());
    }

    // Two arguments
    let sym1 = interner.intern("foo");
    let sym2 = interner.intern("bar");
    let ctx = ctx_with_macros(&interner, &registry);
    let args = [Value::Symbol(sym1), Value::Symbol(sym2)];
    let result = native_is_macro(&args, &ctx);
    assert!(result.is_err());
}

#[test]
fn expand_once_returns_non_list_unchanged() {
    let mut interner = Interner::new();
    let registry = MacroRegistry::new();

    let sym = interner.intern("foo");
    let ctx = ctx_with_macros(&interner, &registry);
    let args = [Value::Symbol(sym)];
    let result = native_expand_once(&args, &ctx).unwrap();

    assert_eq!(result, Value::Symbol(sym));
}

#[test]
fn expand_once_returns_non_macro_list_unchanged() {
    let mut interner = Interner::new();
    let registry = MacroRegistry::new();

    let sym = interner.intern("not-a-macro");
    let list = List::from_vec(alloc::vec![
        Value::Symbol(sym),
        Value::Integer(Integer::from_i64(1_i64)),
    ]);
    let ctx = ctx_with_macros(&interner, &registry);
    let args = [Value::List(list.clone())];
    let result = native_expand_once(&args, &ctx).unwrap();

    assert_eq!(result, Value::List(list));
}

#[test]
fn expand_once_expands_macro_call() {
    let mut interner = Interner::new();
    let mut registry = MacroRegistry::new();

    // Pre-intern collection primitives (required for macro expansion)
    let _primitives = intern_primitives(&mut interner);

    // Register an identity macro
    let macro_name = interner.intern("identity");
    let chunk = Arc::new(make_identity_chunk());
    let def = MacroDefinition::single_arity(chunk, 1_u8, false, String::from("identity"));
    registry.register(macro_name, def);

    // Create call: (identity 42)
    let list = List::from_vec(alloc::vec![
        Value::Symbol(macro_name),
        Value::Integer(Integer::from_i64(42_i64)),
    ]);
    let ctx = ctx_with_macros(&interner, &registry);
    let args = [Value::List(list)];

    let result = native_expand_once(&args, &ctx).unwrap();

    // Should return the argument (identity returns first arg)
    assert_eq!(result, Value::Integer(Integer::from_i64(42_i64)));
}

#[test]
fn expand_fully_stops_when_stable() {
    let mut interner = Interner::new();
    let registry = MacroRegistry::new();

    // A non-macro form should return immediately
    let sym = interner.intern("foo");
    let ctx = ctx_with_macros(&interner, &registry);
    let args = [Value::Symbol(sym)];
    let result = native_expand_fully(&args, &ctx).unwrap();

    assert_eq!(result, Value::Symbol(sym));
}

#[test]
fn values_equal_handles_lists() {
    let list1 = Value::List(List::from_vec(alloc::vec![
        Value::Integer(Integer::from_i64(1_i64)),
        Value::Integer(Integer::from_i64(2_i64)),
    ]));
    let list2 = Value::List(List::from_vec(alloc::vec![
        Value::Integer(Integer::from_i64(1_i64)),
        Value::Integer(Integer::from_i64(2_i64)),
    ]));
    let list3 = Value::List(List::from_vec(alloc::vec![
        Value::Integer(Integer::from_i64(1_i64)),
        Value::Integer(Integer::from_i64(3_i64)),
    ]));

    assert!(values_equal(&list1, &list2));
    assert!(!values_equal(&list1, &list3));
}
