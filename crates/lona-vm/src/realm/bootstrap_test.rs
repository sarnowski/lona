// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for realm bootstrap functionality.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::items_after_statements,
    clippy::uninlined_format_args
)]

use crate::Vaddr;
use crate::platform::{MemorySpace, MockVSpace};
use crate::realm::{Realm, bootstrap, get_core_ns, get_ns_var, lookup_var_in_ns};
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::{HeapNamespace, HeapVar};
use crate::term::tag::object;

/// Create a test setup with realm and memory.
fn setup() -> (Realm, MockVSpace) {
    // Memory: 0x1000-0x10000 for realm code region (60KB should be enough)
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x1000));
    let realm = Realm::new(Vaddr::new(0x1000), 0xF000);
    (realm, mem)
}

/// Check if a term is a namespace by reading its header.
fn is_term_namespace(mem: &MockVSpace, term: Term) -> bool {
    if !term.is_boxed() {
        return false;
    }
    let addr = term.to_vaddr();
    let header: Header = mem.read(addr);
    header.object_tag() == object::NAMESPACE
}

/// Check if a term is a var by reading its header.
fn is_term_var(mem: &MockVSpace, term: Term) -> bool {
    if !term.is_boxed() {
        return false;
    }
    let addr = term.to_vaddr();
    let header: Header = mem.read(addr);
    header.object_tag() == object::VAR
}

/// Read a `HeapVar` from a term.
fn read_heap_var(mem: &MockVSpace, term: Term) -> Option<HeapVar> {
    if !is_term_var(mem, term) {
        return None;
    }
    let addr = term.to_vaddr();
    Some(mem.read(addr))
}

#[test]
fn test_bootstrap_creates_lona_core() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem);
    assert!(result.is_some());

    let result = result.unwrap();
    assert!(is_term_namespace(&mem, result.core_ns));

    // Verify we can find lona.core by name
    let core_ns = get_core_ns(&realm, &mem);
    assert!(core_ns.is_some());
    assert_eq!(core_ns.unwrap(), result.core_ns);
}

#[test]
fn test_bootstrap_seeds_ns_var() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // *ns* should exist
    let ns_var = get_ns_var(&realm, &mem);
    assert!(ns_var.is_some());
    assert_eq!(ns_var.unwrap(), result.ns_var);

    // *ns* should be a var
    assert!(is_term_var(&mem, result.ns_var));

    let var = read_heap_var(&mem, result.ns_var).expect("Expected var");

    // Root should be lona.core namespace
    assert!(is_term_namespace(&mem, var.root));
    assert_eq!(var.root, result.core_ns);
}

#[test]
fn test_bootstrap_seeds_def() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // def should exist in lona.core
    let def_var = lookup_var_in_ns(&realm, &mem, result.core_ns, "def");
    assert!(def_var.is_some());

    // def should be a var
    assert!(is_term_var(&mem, def_var.unwrap()));

    let var = read_heap_var(&mem, def_var.unwrap()).expect("Expected var for 'def'");
    // def is a special form - root should be unbound (special forms are handled by compiler)
    assert!(var.root.is_unbound());
}

#[test]
fn test_bootstrap_seeds_other_special_forms() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // All special forms should exist
    for name in ["fn*", "quote", "do", "var", "match"] {
        let var_term = lookup_var_in_ns(&realm, &mem, result.core_ns, name);
        assert!(var_term.is_some(), "Special form '{}' should exist", name);

        assert!(
            is_term_var(&mem, var_term.unwrap()),
            "'{}' should be a var",
            name
        );

        let var = read_heap_var(&mem, var_term.unwrap()).expect("Expected var");
        // Special forms have unbound root (handled by compiler)
        assert!(
            var.root.is_unbound(),
            "'{}' should have unbound root (special form)",
            name
        );
    }
}

#[test]
fn test_bootstrap_seeds_arithmetic_intrinsics() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // Arithmetic intrinsics should exist
    let intrinsics = [
        ("+", 0u16),
        ("-", 1u16),
        ("*", 2u16),
        ("/", 3u16),
        ("mod", 4u16),
    ];

    for (name, id) in intrinsics {
        let var_term = lookup_var_in_ns(&realm, &mem, result.core_ns, name);
        assert!(var_term.is_some(), "Intrinsic '{}' should exist", name);

        let var = read_heap_var(&mem, var_term.unwrap()).expect("Expected var");

        // Root should be NativeFn with correct ID
        assert!(
            var.root.is_native_fn(),
            "'{}' should have NativeFn root",
            name
        );
        let actual_id = var.root.as_native_fn_id().expect("Expected native fn id");
        assert_eq!(actual_id, id, "'{}' should have id {}", name, id);
    }
}

#[test]
fn test_bootstrap_seeds_comparison_intrinsics() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    let intrinsics = [
        ("=", 5u16),
        ("<", 6u16),
        (">", 7u16),
        ("<=", 8u16),
        (">=", 9u16),
    ];

    for (name, id) in intrinsics {
        let var_term = lookup_var_in_ns(&realm, &mem, result.core_ns, name);
        assert!(var_term.is_some(), "Intrinsic '{}' should exist", name);

        let var = read_heap_var(&mem, var_term.unwrap()).expect("Expected var");

        assert!(
            var.root.is_native_fn(),
            "'{}' should have NativeFn root",
            name
        );
        let actual_id = var.root.as_native_fn_id().expect("Expected native fn id");
        assert_eq!(actual_id, id, "'{}' should have id {}", name, id);
    }
}

#[test]
fn test_bootstrap_seeds_predicate_intrinsics() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    let intrinsics = [
        ("nil?", 11u16),
        ("integer?", 12u16),
        ("string?", 13u16),
        ("keyword?", 15u16),
        ("tuple?", 19u16),
        ("symbol?", 22u16),
        ("map?", 23u16),
        ("namespace?", 30u16),
        ("fn?", 35u16),
        ("var?", 36u16),
    ];

    for (name, id) in intrinsics {
        let var_term = lookup_var_in_ns(&realm, &mem, result.core_ns, name);
        assert!(var_term.is_some(), "Intrinsic '{}' should exist", name);

        let var = read_heap_var(&mem, var_term.unwrap()).expect("Expected var");

        assert!(
            var.root.is_native_fn(),
            "'{}' should have NativeFn root",
            name
        );
        let actual_id = var.root.as_native_fn_id().expect("Expected native fn id");
        assert_eq!(actual_id, id, "'{}' should have id {}", name, id);
    }
}

#[test]
fn test_bootstrap_namespace_has_correct_name() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    assert!(is_term_namespace(&mem, result.core_ns));

    let ns_addr = result.core_ns.to_vaddr();
    let ns: HeapNamespace = mem.read(ns_addr);

    // Name should be an immediate symbol (interned)
    assert!(ns.name.is_symbol());

    // Look up symbol name from realm
    let idx = ns.name.as_symbol_index().expect("Expected symbol index");
    let name_str = realm
        .symbol_name(&mem, idx)
        .expect("Symbol should be in realm");
    assert_eq!(name_str, "lona.core");
}

#[test]
fn test_lookup_var_finds_existing() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // Should find +
    let var = lookup_var_in_ns(&realm, &mem, result.core_ns, "+");
    assert!(var.is_some());
    assert!(is_term_var(&mem, var.unwrap()));
}

#[test]
fn test_lookup_var_returns_none_for_missing() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // Should not find non-existent var
    let var = lookup_var_in_ns(&realm, &mem, result.core_ns, "nonexistent");
    assert!(var.is_none());
}
