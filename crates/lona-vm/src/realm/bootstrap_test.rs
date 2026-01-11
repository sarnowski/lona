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
use crate::value::{Namespace, Value, VarContent};

/// Create a test setup with realm and memory.
fn setup() -> (Realm, MockVSpace) {
    // Memory: 0x1000-0x10000 for realm code region (60KB should be enough)
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x1000));
    let realm = Realm::new(Vaddr::new(0x1000), 0xF000);
    (realm, mem)
}

#[test]
fn test_bootstrap_creates_lona_core() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem);
    assert!(result.is_some());

    let result = result.unwrap();
    assert!(result.core_ns.is_namespace());

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

    // *ns* should be process-bound
    let Value::Var(slot_addr) = result.ns_var else {
        panic!("Expected var");
    };

    use crate::value::VarSlot;
    let slot: VarSlot = mem.read(slot_addr);
    let content: VarContent = mem.read(slot.content);
    assert!(content.is_process_bound());

    // Root should be lona.core namespace
    assert!(content.root.is_namespace());
    assert_eq!(content.root, result.core_ns);
}

#[test]
fn test_bootstrap_seeds_def() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // def should exist in lona.core
    let def_var = lookup_var_in_ns(&realm, &mem, result.core_ns, "def");
    assert!(def_var.is_some());

    // def should be a special form
    let Value::Var(slot_addr) = def_var.unwrap() else {
        panic!("Expected var");
    };

    use crate::value::VarSlot;
    let slot: VarSlot = mem.read(slot_addr);
    let content: VarContent = mem.read(slot.content);
    assert!(content.is_special_form());
    assert!(content.is_native());
}

#[test]
fn test_bootstrap_seeds_other_special_forms() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // All special forms should exist
    for name in ["fn*", "quote", "do", "var", "match"] {
        let var = lookup_var_in_ns(&realm, &mem, result.core_ns, name);
        assert!(var.is_some(), "Special form '{}' should exist", name);

        let Value::Var(slot_addr) = var.unwrap() else {
            panic!("Expected var for '{}'", name);
        };

        use crate::value::VarSlot;
        let slot: VarSlot = mem.read(slot_addr);
        let content: VarContent = mem.read(slot.content);
        assert!(
            content.is_special_form(),
            "'{}' should be special form",
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
        let var = lookup_var_in_ns(&realm, &mem, result.core_ns, name);
        assert!(var.is_some(), "Intrinsic '{}' should exist", name);

        let Value::Var(slot_addr) = var.unwrap() else {
            panic!("Expected var for '{}'", name);
        };

        use crate::value::VarSlot;
        let slot: VarSlot = mem.read(slot_addr);
        let content: VarContent = mem.read(slot.content);

        assert!(content.is_native(), "'{}' should be native", name);
        assert!(
            !content.is_special_form(),
            "'{}' should not be special form",
            name
        );

        // Root should be NativeFn with correct ID
        match content.root {
            Value::NativeFn(actual_id) => {
                assert_eq!(actual_id, id, "'{}' should have id {}", name, id);
            }
            _ => panic!("'{}' should have NativeFn root", name),
        }
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
        let var = lookup_var_in_ns(&realm, &mem, result.core_ns, name);
        assert!(var.is_some(), "Intrinsic '{}' should exist", name);

        let Value::Var(slot_addr) = var.unwrap() else {
            panic!()
        };

        use crate::value::VarSlot;
        let slot: VarSlot = mem.read(slot_addr);
        let content: VarContent = mem.read(slot.content);

        match content.root {
            Value::NativeFn(actual_id) => {
                assert_eq!(actual_id, id, "'{}' should have id {}", name, id);
            }
            _ => panic!("'{}' should have NativeFn root", name),
        }
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
        let var = lookup_var_in_ns(&realm, &mem, result.core_ns, name);
        assert!(var.is_some(), "Intrinsic '{}' should exist", name);

        let Value::Var(slot_addr) = var.unwrap() else {
            panic!()
        };

        use crate::value::VarSlot;
        let slot: VarSlot = mem.read(slot_addr);
        let content: VarContent = mem.read(slot.content);

        match content.root {
            Value::NativeFn(actual_id) => {
                assert_eq!(actual_id, id, "'{}' should have id {}", name, id);
            }
            _ => panic!("'{}' should have NativeFn root", name),
        }
    }
}

#[test]
fn test_bootstrap_namespace_has_correct_name() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    let Value::Namespace(ns_addr) = result.core_ns else {
        panic!("Expected namespace");
    };

    let ns: Namespace = mem.read(ns_addr);

    // Name should be symbol "lona.core"
    assert!(ns.name.is_symbol());

    use crate::value::HeapString;
    let Value::Symbol(sym_addr) = ns.name else {
        panic!()
    };
    let header: HeapString = mem.read(sym_addr);
    let data_addr = sym_addr.add(HeapString::HEADER_SIZE as u64);
    let bytes = mem.slice(data_addr, header.len as usize);
    assert_eq!(bytes, b"lona.core");
}

#[test]
fn test_lookup_var_finds_existing() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // Should find +
    let var = lookup_var_in_ns(&realm, &mem, result.core_ns, "+");
    assert!(var.is_some());
    assert!(var.unwrap().is_var());
}

#[test]
fn test_lookup_var_returns_none_for_missing() {
    let (mut realm, mut mem) = setup();

    let result = bootstrap(&mut realm, &mut mem).unwrap();

    // Should not find non-existent var
    let var = lookup_var_in_ns(&realm, &mem, result.core_ns, "nonexistent");
    assert!(var.is_none());
}
