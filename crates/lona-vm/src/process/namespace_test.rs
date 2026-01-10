// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for namespace allocation and registry.

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
fn namespace_registry_basic() {
    let (mut proc, mut mem) = setup();

    // Create namespace
    let name = proc.alloc_symbol(&mut mem, "lona.core").unwrap();
    let ns = proc.get_or_create_namespace(&mut mem, name).unwrap();
    assert!(matches!(ns, Value::Namespace(_)));

    // Find it again
    let found = proc.find_namespace(&mem, name);
    assert!(found.is_some());
    assert_eq!(found.unwrap(), ns);
}

#[test]
fn namespace_registry_not_found() {
    let (mut proc, mut mem) = setup();

    // Search for non-existent namespace
    let name = proc.alloc_symbol(&mut mem, "nonexistent").unwrap();
    let found = proc.find_namespace(&mem, name);
    assert!(found.is_none());
}

#[test]
fn namespace_registry_get_or_create_idempotent() {
    let (mut proc, mut mem) = setup();

    let name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();

    // First call creates
    let ns1 = proc.get_or_create_namespace(&mut mem, name).unwrap();

    // Second call returns same namespace
    let ns2 = proc.get_or_create_namespace(&mut mem, name).unwrap();

    assert_eq!(ns1, ns2);
}

#[test]
fn namespace_registry_multiple() {
    let (mut proc, mut mem) = setup();

    // Create multiple namespaces
    let name1 = proc.alloc_symbol(&mut mem, "ns.one").unwrap();
    let name2 = proc.alloc_symbol(&mut mem, "ns.two").unwrap();
    let name3 = proc.alloc_symbol(&mut mem, "ns.three").unwrap();

    let ns1 = proc.get_or_create_namespace(&mut mem, name1).unwrap();
    let ns2 = proc.get_or_create_namespace(&mut mem, name2).unwrap();
    let ns3 = proc.get_or_create_namespace(&mut mem, name3).unwrap();

    // All should be different
    assert_ne!(ns1, ns2);
    assert_ne!(ns2, ns3);
    assert_ne!(ns1, ns3);

    // All should be findable
    assert_eq!(proc.find_namespace(&mem, name1), Some(ns1));
    assert_eq!(proc.find_namespace(&mem, name2), Some(ns2));
    assert_eq!(proc.find_namespace(&mem, name3), Some(ns3));
}
