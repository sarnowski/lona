// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for namespace allocation on the process heap.
//!
//! For namespace registry tests (find, register, `get_or_create`),
//! see `realm/realm_test.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::allocation_test::setup;
use crate::value::Value;

#[test]
fn alloc_namespace() {
    let (mut proc, mut mem) = setup();

    // Create a symbol for the namespace name
    let name = proc.alloc_symbol(&mut mem, "my.app").unwrap();

    // Allocate namespace
    let ns = proc.alloc_namespace(&mut mem, name).unwrap();
    assert!(matches!(ns, Value::Namespace(_)));

    // Read back the namespace
    let ns_val = proc.read_namespace(&mem, ns).unwrap();
    assert_eq!(ns_val.name, name);
    assert!(matches!(ns_val.mappings, Value::Map(_)));
}

#[test]
fn alloc_namespace_multiple() {
    let (mut proc, mut mem) = setup();

    // Create multiple namespaces (each on the heap)
    let name1 = proc.alloc_symbol(&mut mem, "ns.one").unwrap();
    let name2 = proc.alloc_symbol(&mut mem, "ns.two").unwrap();

    let ns1 = proc.alloc_namespace(&mut mem, name1).unwrap();
    let ns2 = proc.alloc_namespace(&mut mem, name2).unwrap();

    // Different names, different namespace values
    assert_ne!(ns1, ns2);

    // Both should be readable
    let val1 = proc.read_namespace(&mem, ns1).unwrap();
    let val2 = proc.read_namespace(&mem, ns2).unwrap();
    assert_eq!(val1.name, name1);
    assert_eq!(val2.name, name2);
}

#[test]
fn read_namespace_wrong_type() {
    let (mut proc, mut mem) = setup();

    // Non-namespace values should return None
    let int_val = Value::int(42);
    assert!(proc.read_namespace(&mem, int_val).is_none());

    let string_val = proc.alloc_string(&mut mem, "hello").unwrap();
    assert!(proc.read_namespace(&mem, string_val).is_none());
}
