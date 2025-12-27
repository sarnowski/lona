// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for namespace loading and require functionality.

use lona_core::symbol::Interner;

use super::make_vm;
use crate::namespace::MemorySourceLoader;
use crate::vm::error::Kind as ErrorKind;

/// Test that prepare_require returns None for already-loaded namespaces.
#[test]
fn prepare_require_returns_none_for_loaded_namespace() {
    let interner = Interner::new();
    let vm = make_vm(&interner);

    // "user" is bootstrapped in the namespace registry
    let user_ns = interner.intern("user");
    let result = vm.prepare_require(user_ns);

    assert!(result.is_ok());
    assert!(result.as_ref().ok().and_then(|opt| *opt).is_none());
}

/// Test that prepare_require returns None for lona.core which is also bootstrapped.
#[test]
fn prepare_require_returns_none_for_core_namespace() {
    let interner = Interner::new();
    let vm = make_vm(&interner);

    // "lona.core" is bootstrapped in the namespace registry
    let core_ns = interner.intern("lona.core");
    let result = vm.prepare_require(core_ns);

    assert!(result.is_ok());
    assert!(result.as_ref().ok().and_then(|opt| *opt).is_none());
}

/// Test that prepare_require returns NoSourceLoader error when no loader is set.
#[test]
fn prepare_require_returns_no_source_loader_error() {
    let interner = Interner::new();
    let vm = make_vm(&interner);

    // "my.unknown.ns" is not in the registry
    let unknown_ns = interner.intern("my.unknown.ns");
    let result = vm.prepare_require(unknown_ns);

    assert!(result.is_err());
    let err = result.err();
    let Some(ref error) = err else {
        panic!("expected Some(error)");
    };
    assert!(matches!(error.kind, ErrorKind::NoSourceLoader));
}

/// Test that prepare_require returns NamespaceNotFound when loader doesn't have source.
#[test]
fn prepare_require_returns_namespace_not_found() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    // Set up an empty loader
    let loader = MemorySourceLoader::new();
    vm.set_loader(&loader);

    // "my.unknown.ns" is not in the loader
    let unknown_ns = interner.intern("my.unknown.ns");
    let result = vm.prepare_require(unknown_ns);

    assert!(result.is_err());
    let err = result.err();
    let Some(ref error) = err else {
        panic!("expected Some(error)");
    };
    match error.kind {
        ErrorKind::NamespaceNotFound { namespace } => {
            assert_eq!(namespace, unknown_ns);
        }
        ref other => panic!("expected NamespaceNotFound, got {other:?}"),
    }
}

/// Test that prepare_require returns source code when namespace exists in loader.
#[test]
fn prepare_require_returns_source_for_existing_namespace() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    // Set up a loader with source
    let mut loader = MemorySourceLoader::new();
    loader.add("my.namespace".into(), "(def x 42)".into());
    vm.set_loader(&loader);

    // "my.namespace" is in the loader but not in the registry
    let my_ns = interner.intern("my.namespace");
    let result = vm.prepare_require(my_ns);

    assert!(result.is_ok());
    let Some(source) = result.as_ref().ok().and_then(|opt| *opt) else {
        panic!("expected Some(source)");
    };
    assert_eq!(source, "(def x 42)");
}

/// Test that prepare_require detects circular dependencies.
#[test]
fn prepare_require_detects_circular_dependency() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    // Set up a loader with source
    let mut loader = MemorySourceLoader::new();
    loader.add("ns.a".into(), "(ns ns.a (:require [ns.b]))".into());
    loader.add("ns.b".into(), "(ns ns.b (:require [ns.a]))".into());
    vm.set_loader(&loader);

    // Simulate that ns.a is being loaded
    let ns_a = interner.intern("ns.a");
    vm.push_loading(ns_a);

    // Now try to load ns.a again (circular dependency)
    let result = vm.prepare_require(ns_a);

    assert!(result.is_err());
    let err = result.err();
    let Some(ref error) = err else {
        panic!("expected Some(error)");
    };
    match error.kind {
        ErrorKind::CircularDependency {
            namespace,
            ref stack,
        } => {
            assert_eq!(namespace, ns_a);
            assert_eq!(stack.len(), 1);
            assert_eq!(stack.first().copied(), Some(ns_a));
        }
        ref other => panic!("expected CircularDependency, got {other:?}"),
    }
}

/// Test that loading stack grows correctly with multiple namespaces.
#[test]
fn prepare_require_captures_full_dependency_chain() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    // Set up a loader with source
    let mut loader = MemorySourceLoader::new();
    loader.add("ns.a".into(), "(ns ns.a)".into());
    loader.add("ns.b".into(), "(ns ns.b)".into());
    loader.add("ns.c".into(), "(ns ns.c)".into());
    vm.set_loader(&loader);

    // Simulate loading chain: ns.a -> ns.b -> ns.c -> ns.a (circular)
    let ns_a = interner.intern("ns.a");
    let ns_b = interner.intern("ns.b");
    let ns_c = interner.intern("ns.c");

    vm.push_loading(ns_a);
    vm.push_loading(ns_b);
    vm.push_loading(ns_c);

    // Now try to load ns.a again (circular dependency)
    let result = vm.prepare_require(ns_a);

    assert!(result.is_err());
    let err = result.err();
    let Some(ref error) = err else {
        panic!("expected Some(error)");
    };
    match error.kind {
        ErrorKind::CircularDependency {
            namespace,
            ref stack,
        } => {
            assert_eq!(namespace, ns_a);
            assert_eq!(stack.len(), 3);
            assert_eq!(stack.first().copied(), Some(ns_a));
            assert_eq!(stack.get(1).copied(), Some(ns_b));
            assert_eq!(stack.get(2).copied(), Some(ns_c));
        }
        ref other => panic!("expected CircularDependency, got {other:?}"),
    }
}

/// Test that namespace registry is accessible and can be modified.
#[test]
fn namespace_registry_is_accessible() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    // The registry should have "user" and "lona.core"
    let registry = vm.namespace_registry();
    assert_eq!(registry.len(), 2);

    let user_ns = interner.intern("user");
    assert!(registry.contains(user_ns));

    // Switch to a new namespace (creates it)
    let new_ns = interner.intern("my.new.ns");
    vm.namespace_registry_mut().switch_to(new_ns);

    // Now it should exist
    assert!(vm.namespace_registry().contains(new_ns));
    assert_eq!(vm.namespace_registry().len(), 3);
}

/// Test that push_loading and pop_loading work correctly.
#[test]
fn loading_stack_push_pop_works() {
    let interner = Interner::new();
    let mut vm = make_vm(&interner);

    let ns_a = interner.intern("ns.a");
    let ns_b = interner.intern("ns.b");

    assert!(!vm.is_loading(ns_a));
    assert!(!vm.is_loading(ns_b));

    vm.push_loading(ns_a);
    assert!(vm.is_loading(ns_a));
    assert!(!vm.is_loading(ns_b));

    vm.push_loading(ns_b);
    assert!(vm.is_loading(ns_a));
    assert!(vm.is_loading(ns_b));

    vm.pop_loading();
    assert!(vm.is_loading(ns_a));
    assert!(!vm.is_loading(ns_b));

    vm.pop_loading();
    assert!(!vm.is_loading(ns_a));
    assert!(!vm.is_loading(ns_b));
}
