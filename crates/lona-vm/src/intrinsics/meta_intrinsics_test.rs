// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for namespace and var intrinsic functions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::realm::Realm;

/// Create a test environment with process, memory, and realm.
fn setup() -> (Process, MockVSpace, Realm) {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let proc = Process::new(1, young_base, young_size, old_base, old_size);
    let realm_base = base.add(128 * 1024);
    let realm = Realm::new(realm_base, 64 * 1024);
    (proc, mem, realm)
}

// --- Namespace intrinsics ---

#[test]
fn lookup_namespace_intrinsics() {
    assert_eq!(lookup_intrinsic("namespace?"), Some(id::IS_NAMESPACE));
    assert_eq!(lookup_intrinsic("create-ns"), Some(id::CREATE_NS));
    assert_eq!(lookup_intrinsic("find-ns"), Some(id::FIND_NS));
    assert_eq!(lookup_intrinsic("ns-name"), Some(id::NS_NAME));
    assert_eq!(lookup_intrinsic("ns-map"), Some(id::NS_MAP));
}

#[test]
fn is_namespace_true() {
    let (mut proc, mut mem, mut realm) = setup();

    let name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();
    let ns = proc.alloc_namespace(&mut mem, name).unwrap();

    proc.x_regs[1] = ns;
    call_intrinsic(id::IS_NAMESPACE, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn is_namespace_false() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(42);
    call_intrinsic(id::IS_NAMESPACE, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    proc.x_regs[1] = Value::nil();
    call_intrinsic(id::IS_NAMESPACE, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    let sym = proc.alloc_symbol(&mut mem, "foo").unwrap();
    proc.x_regs[1] = sym;
    call_intrinsic(id::IS_NAMESPACE, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn create_ns_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    let name = proc.alloc_symbol(&mut mem, "my.app").unwrap();
    proc.x_regs[1] = name;

    call_intrinsic(id::CREATE_NS, 1, &mut proc, &mut mem, &mut realm).unwrap();

    let result = proc.x_regs[0];
    assert!(result.is_namespace());
}

#[test]
fn create_ns_type_error() {
    let (mut proc, mut mem, mut realm) = setup();

    // Try to create namespace with non-symbol
    proc.x_regs[1] = Value::int(42);
    let result = call_intrinsic(id::CREATE_NS, 1, &mut proc, &mut mem, &mut realm);
    assert!(result.is_err());
}

#[test]
fn find_ns_exists() {
    let (mut proc, mut mem, mut realm) = setup();

    // First create the namespace
    let name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();
    proc.x_regs[1] = name;
    call_intrinsic(id::CREATE_NS, 1, &mut proc, &mut mem, &mut realm).unwrap();
    let created_ns = proc.x_regs[0];

    // Now find it
    proc.x_regs[1] = name;
    call_intrinsic(id::FIND_NS, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], created_ns);
}

#[test]
fn find_ns_not_exists() {
    let (mut proc, mut mem, mut realm) = setup();

    let name = proc.alloc_symbol(&mut mem, "nonexistent").unwrap();
    proc.x_regs[1] = name;

    call_intrinsic(id::FIND_NS, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::nil());
}

#[test]
fn ns_name_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    let name = proc.alloc_symbol(&mut mem, "lona.core").unwrap();
    let ns = proc.alloc_namespace(&mut mem, name).unwrap();

    proc.x_regs[1] = ns;
    call_intrinsic(id::NS_NAME, 1, &mut proc, &mut mem, &mut realm).unwrap();

    let result = proc.x_regs[0];
    assert!(result.is_symbol());

    let name_str = proc.read_string(&mem, result).unwrap();
    assert_eq!(name_str, "lona.core");
}

#[test]
fn ns_name_type_error() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(42);
    let result = call_intrinsic(id::NS_NAME, 1, &mut proc, &mut mem, &mut realm);
    assert!(result.is_err());
}

#[test]
fn ns_map_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    let name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();
    let ns = proc.alloc_namespace(&mut mem, name).unwrap();

    proc.x_regs[1] = ns;
    call_intrinsic(id::NS_MAP, 1, &mut proc, &mut mem, &mut realm).unwrap();

    let result = proc.x_regs[0];
    assert!(result.is_map());
}

#[test]
fn ns_map_type_error() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::nil();
    let result = call_intrinsic(id::NS_MAP, 1, &mut proc, &mut mem, &mut realm);
    assert!(result.is_err());
}

// --- Var intrinsic tests ---

#[test]
fn lookup_var_intrinsics() {
    assert_eq!(lookup_intrinsic("var?"), Some(id::IS_VAR));
    assert_eq!(lookup_intrinsic("intern"), Some(id::INTERN));
    assert_eq!(lookup_intrinsic("var-get"), Some(id::VAR_GET));
}

#[test]
fn is_var_true() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create namespace
    let ns_name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();
    let ns = proc.alloc_namespace(&mut mem, ns_name).unwrap();

    // Intern a var
    let var_name = proc.alloc_symbol(&mut mem, "x").unwrap();
    proc.x_regs[1] = ns;
    proc.x_regs[2] = var_name;
    proc.x_regs[3] = Value::int(42);
    call_intrinsic(id::INTERN, 3, &mut proc, &mut mem, &mut realm).unwrap();

    let var = proc.x_regs[0];

    // Check is_var
    proc.x_regs[1] = var;
    call_intrinsic(id::IS_VAR, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(true));
}

#[test]
fn is_var_false() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(42);
    call_intrinsic(id::IS_VAR, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));

    proc.x_regs[1] = Value::nil();
    call_intrinsic(id::IS_VAR, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::bool(false));
}

#[test]
fn intern_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create namespace
    let ns_name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();
    let ns = proc.alloc_namespace(&mut mem, ns_name).unwrap();

    // Intern a var
    let var_name = proc.alloc_symbol(&mut mem, "x").unwrap();
    proc.x_regs[1] = ns;
    proc.x_regs[2] = var_name;
    proc.x_regs[3] = Value::int(42);
    call_intrinsic(id::INTERN, 3, &mut proc, &mut mem, &mut realm).unwrap();

    let var = proc.x_regs[0];
    assert!(var.is_var());

    // Get the var content
    let content = proc.read_var_content(&mem, var).unwrap();
    assert_eq!(content.root, Value::int(42));
}

#[test]
fn intern_type_error_namespace() {
    let (mut proc, mut mem, mut realm) = setup();

    let var_name = proc.alloc_symbol(&mut mem, "x").unwrap();
    proc.x_regs[1] = Value::int(42); // Not a namespace
    proc.x_regs[2] = var_name;
    proc.x_regs[3] = Value::int(100);

    let result = call_intrinsic(id::INTERN, 3, &mut proc, &mut mem, &mut realm);
    assert!(result.is_err());
}

#[test]
fn intern_type_error_symbol() {
    let (mut proc, mut mem, mut realm) = setup();

    let ns_name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();
    let ns = proc.alloc_namespace(&mut mem, ns_name).unwrap();

    proc.x_regs[1] = ns;
    proc.x_regs[2] = Value::int(42); // Not a symbol
    proc.x_regs[3] = Value::int(100);

    let result = call_intrinsic(id::INTERN, 3, &mut proc, &mut mem, &mut realm);
    assert!(result.is_err());
}

#[test]
fn var_get_basic() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create namespace
    let ns_name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();
    let ns = proc.alloc_namespace(&mut mem, ns_name).unwrap();

    // Intern a var with value 42
    let var_name = proc.alloc_symbol(&mut mem, "x").unwrap();
    proc.x_regs[1] = ns;
    proc.x_regs[2] = var_name;
    proc.x_regs[3] = Value::int(42);
    call_intrinsic(id::INTERN, 3, &mut proc, &mut mem, &mut realm).unwrap();

    let var = proc.x_regs[0];

    // Get the var's value
    proc.x_regs[1] = var;
    call_intrinsic(id::VAR_GET, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(42));
}

#[test]
fn var_get_type_error() {
    let (mut proc, mut mem, mut realm) = setup();

    proc.x_regs[1] = Value::int(42); // Not a var
    let result = call_intrinsic(id::VAR_GET, 1, &mut proc, &mut mem, &mut realm);
    assert!(result.is_err());
}

#[test]
fn var_get_unbound() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create namespace
    let ns_name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();
    let ns = proc.alloc_namespace(&mut mem, ns_name).unwrap();

    // Intern a var with unbound value
    let var_name = proc.alloc_symbol(&mut mem, "x").unwrap();
    proc.x_regs[1] = ns;
    proc.x_regs[2] = var_name;
    proc.x_regs[3] = Value::Unbound;
    call_intrinsic(id::INTERN, 3, &mut proc, &mut mem, &mut realm).unwrap();

    let var = proc.x_regs[0];

    // Get the var's value (should error because unbound)
    proc.x_regs[1] = var;
    let result = call_intrinsic(id::VAR_GET, 1, &mut proc, &mut mem, &mut realm);
    assert!(result.is_err());
}

#[test]
fn intern_update_var() {
    let (mut proc, mut mem, mut realm) = setup();

    // Create namespace
    let ns_name = proc.alloc_symbol(&mut mem, "test.ns").unwrap();
    let ns = proc.alloc_namespace(&mut mem, ns_name).unwrap();

    // Intern a var with value 42
    let var_name = proc.alloc_symbol(&mut mem, "x").unwrap();
    proc.x_regs[1] = ns;
    proc.x_regs[2] = var_name;
    proc.x_regs[3] = Value::int(42);
    call_intrinsic(id::INTERN, 3, &mut proc, &mut mem, &mut realm).unwrap();

    let var1 = proc.x_regs[0];

    // Intern same var with new value 100
    proc.x_regs[1] = ns;
    proc.x_regs[2] = var_name;
    proc.x_regs[3] = Value::int(100);
    call_intrinsic(id::INTERN, 3, &mut proc, &mut mem, &mut realm).unwrap();

    let var2 = proc.x_regs[0];

    // Both should be the same var (same address)
    assert_eq!(var1, var2);

    // Get the var's value - should be updated to 100
    proc.x_regs[1] = var2;
    call_intrinsic(id::VAR_GET, 1, &mut proc, &mut mem, &mut realm).unwrap();
    assert_eq!(proc.x_regs[0], Value::int(100));
}
