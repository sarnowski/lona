// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for process-bound variable bindings.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::value::{Value, var_flags};

/// Create a test process with young and old heaps.
fn create_test_process() -> (Process, MockVSpace) {
    let young_base = Vaddr::new(0x1000_0000);
    let young_size = 48 * 1024; // 48KB
    let old_base = Vaddr::new(0x1001_0000);
    let old_size = 12 * 1024; // 12KB

    let process = Process::new(1, young_base, young_size, old_base, old_size);

    // Create memory large enough for both heaps
    let total_size = (old_base.as_u64() - young_base.as_u64()) as usize + old_size;
    let mem = MockVSpace::new(total_size, young_base);

    (process, mem)
}

// --- Basic Binding Operations ---

#[test]
fn test_get_binding_empty() {
    let (process, _mem) = create_test_process();

    // No bindings initially
    let var_id = Vaddr::new(0x2000_0000);
    assert!(process.get_binding(var_id).is_none());
}

#[test]
fn test_set_and_get_binding() {
    let (mut process, _mem) = create_test_process();

    let var_id = Vaddr::new(0x2000_0000);
    let value = Value::int(42);

    // Set binding
    process.set_binding(var_id, value).unwrap();

    // Get binding
    let result = process.get_binding(var_id).unwrap();
    assert_eq!(result, value);
}

#[test]
fn test_update_binding() {
    let (mut process, _mem) = create_test_process();

    let var_id = Vaddr::new(0x2000_0000);

    // Set initial binding
    process.set_binding(var_id, Value::int(1)).unwrap();
    assert_eq!(process.get_binding(var_id).unwrap(), Value::int(1));

    // Update binding
    process.set_binding(var_id, Value::int(2)).unwrap();
    assert_eq!(process.get_binding(var_id).unwrap(), Value::int(2));
}

#[test]
fn test_multiple_bindings() {
    let (mut process, _mem) = create_test_process();

    let var1 = Vaddr::new(0x2000_0000);
    let var2 = Vaddr::new(0x2000_1000);
    let var3 = Vaddr::new(0x2000_2000);

    process.set_binding(var1, Value::int(1)).unwrap();
    process.set_binding(var2, Value::int(2)).unwrap();
    process.set_binding(var3, Value::int(3)).unwrap();

    assert_eq!(process.get_binding(var1).unwrap(), Value::int(1));
    assert_eq!(process.get_binding(var2).unwrap(), Value::int(2));
    assert_eq!(process.get_binding(var3).unwrap(), Value::int(3));
}

#[test]
fn test_has_binding() {
    let (mut process, _mem) = create_test_process();

    let var1 = Vaddr::new(0x2000_0000);
    let var2 = Vaddr::new(0x2000_1000);

    assert!(!process.has_binding(var1));
    assert!(!process.has_binding(var2));

    process.set_binding(var1, Value::int(1)).unwrap();

    assert!(process.has_binding(var1));
    assert!(!process.has_binding(var2));
}

// --- var_get with Process Bindings ---

#[test]
fn test_var_get_without_process_bound_flag() {
    let (mut process, mut mem) = create_test_process();

    // Create a regular var (not process-bound)
    let var = process
        .alloc_var(&mut mem, Vaddr::new(0), Vaddr::new(0), Value::int(42), 0)
        .unwrap();

    let Value::Var(var_addr) = var else { panic!() };

    // Even if we set a binding, it should be ignored since var is not process-bound
    process.set_binding(var_addr, Value::int(100)).unwrap();

    // var_get should return root value, not binding
    let result = process.var_get(&mem, var).unwrap();
    assert_eq!(result, Value::int(42));
}

#[test]
fn test_var_get_process_bound_no_binding() {
    let (mut process, mut mem) = create_test_process();

    // Create a process-bound var
    let var = process
        .alloc_var(
            &mut mem,
            Vaddr::new(0),
            Vaddr::new(0),
            Value::int(42),
            var_flags::PROCESS_BOUND,
        )
        .unwrap();

    // No binding set - should return root value
    let result = process.var_get(&mem, var).unwrap();
    assert_eq!(result, Value::int(42));
}

#[test]
fn test_var_get_process_bound_with_binding() {
    let (mut process, mut mem) = create_test_process();

    // Create a process-bound var with root value
    let var = process
        .alloc_var(
            &mut mem,
            Vaddr::new(0),
            Vaddr::new(0),
            Value::int(42),
            var_flags::PROCESS_BOUND,
        )
        .unwrap();

    let Value::Var(var_addr) = var else { panic!() };

    // Set a process binding
    process.set_binding(var_addr, Value::int(100)).unwrap();

    // var_get should return the binding, not root value
    let result = process.var_get(&mem, var).unwrap();
    assert_eq!(result, Value::int(100));
}

#[test]
fn test_var_get_process_bound_binding_overrides_root() {
    let (mut process, mut mem) = create_test_process();

    // Create a process-bound var
    let var = process
        .alloc_var(
            &mut mem,
            Vaddr::new(0),
            Vaddr::new(0),
            Value::Nil, // Root is nil
            var_flags::PROCESS_BOUND,
        )
        .unwrap();

    let Value::Var(var_addr) = var else { panic!() };

    // Set binding to a non-nil value
    process.set_binding(var_addr, Value::int(123)).unwrap();

    // Binding should take precedence
    let result = process.var_get(&mem, var).unwrap();
    assert_eq!(result, Value::int(123));

    // Update binding
    process.set_binding(var_addr, Value::Bool(true)).unwrap();
    let result = process.var_get(&mem, var).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_var_get_multiple_process_bound_vars() {
    let (mut process, mut mem) = create_test_process();

    // Create two process-bound vars
    let var1 = process
        .alloc_var(
            &mut mem,
            Vaddr::new(0),
            Vaddr::new(0),
            Value::int(1),
            var_flags::PROCESS_BOUND,
        )
        .unwrap();

    let var2 = process
        .alloc_var(
            &mut mem,
            Vaddr::new(0),
            Vaddr::new(0),
            Value::int(2),
            var_flags::PROCESS_BOUND,
        )
        .unwrap();

    let Value::Var(var1_addr) = var1 else {
        panic!()
    };
    let Value::Var(var2_addr) = var2 else {
        panic!()
    };

    // Set bindings for both
    process.set_binding(var1_addr, Value::int(10)).unwrap();
    process.set_binding(var2_addr, Value::int(20)).unwrap();

    // Each var should return its own binding
    assert_eq!(process.var_get(&mem, var1).unwrap(), Value::int(10));
    assert_eq!(process.var_get(&mem, var2).unwrap(), Value::int(20));
}
