// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for deep copy of compiled functions and closures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::allocation_test::setup;
use crate::value::Value;

#[test]
fn copy_compiled_fn_basic() {
    let (mut proc, mut mem) = setup();

    // Create a simple compiled function
    let code = [0x1234_5678_u32, 0xABCD_EF01];
    let constants = [Value::int(42), Value::int(100)];
    let original = proc
        .alloc_compiled_fn(&mut mem, 2, false, 0, &code, &constants)
        .unwrap();

    // Copy it
    let copy = proc.copy_compiled_fn(&mut mem, original).unwrap();

    // Should be a different address
    let Value::CompiledFn(orig_addr) = original else {
        panic!("expected CompiledFn");
    };
    let Value::CompiledFn(copy_addr) = copy else {
        panic!("expected CompiledFn");
    };
    assert_ne!(orig_addr, copy_addr);

    // Should have same header values
    let orig_header = proc.read_compiled_fn(&mem, original).unwrap();
    let copy_header = proc.read_compiled_fn(&mem, copy).unwrap();
    assert_eq!(orig_header.arity, copy_header.arity);
    assert_eq!(orig_header.variadic, copy_header.variadic);
    assert_eq!(orig_header.num_locals, copy_header.num_locals);
    assert_eq!(orig_header.code_len, copy_header.code_len);
    assert_eq!(orig_header.constants_len, copy_header.constants_len);

    // Should have same bytecode
    for i in 0..code.len() {
        let orig_code = proc.read_compiled_fn_code(&mem, original, i).unwrap();
        let copy_code = proc.read_compiled_fn_code(&mem, copy, i).unwrap();
        assert_eq!(orig_code, copy_code);
    }

    // Should have same constants
    for i in 0..constants.len() {
        let orig_const = proc.read_compiled_fn_constant(&mem, original, i).unwrap();
        let copy_const = proc.read_compiled_fn_constant(&mem, copy, i).unwrap();
        assert_eq!(orig_const, copy_const);
    }
}

#[test]
fn copy_compiled_fn_empty() {
    let (mut proc, mut mem) = setup();

    // Create a function with no bytecode or constants
    let original = proc
        .alloc_compiled_fn(&mut mem, 0, false, 0, &[], &[])
        .unwrap();

    let copy = proc.copy_compiled_fn(&mut mem, original).unwrap();

    let Value::CompiledFn(orig_addr) = original else {
        panic!("expected CompiledFn");
    };
    let Value::CompiledFn(copy_addr) = copy else {
        panic!("expected CompiledFn");
    };
    assert_ne!(orig_addr, copy_addr);

    let copy_header = proc.read_compiled_fn(&mem, copy).unwrap();
    assert_eq!(copy_header.arity, 0);
    assert_eq!(copy_header.code_len, 0);
    assert_eq!(copy_header.constants_len, 0);
}

#[test]
fn copy_compiled_fn_variadic() {
    let (mut proc, mut mem) = setup();

    let code = [0x1111_1111_u32];
    let original = proc
        .alloc_compiled_fn(&mut mem, 3, true, 5, &code, &[])
        .unwrap();

    let copy = proc.copy_compiled_fn(&mut mem, original).unwrap();

    let copy_header = proc.read_compiled_fn(&mem, copy).unwrap();
    assert_eq!(copy_header.arity, 3);
    assert!(copy_header.variadic);
    assert_eq!(copy_header.num_locals, 5);
}

#[test]
fn copy_closure_basic() {
    let (mut proc, mut mem) = setup();

    // Create a compiled function first
    let code = [0xDEAD_BEEF_u32];
    let func = proc
        .alloc_compiled_fn(&mut mem, 1, false, 0, &code, &[])
        .unwrap();

    let Value::CompiledFn(func_addr) = func else {
        panic!("expected CompiledFn");
    };

    // Create captures
    let captures = [Value::int(10), Value::int(20), Value::int(30)];

    // Create closure
    let original = proc.alloc_closure(&mut mem, func_addr, &captures).unwrap();

    // Copy it
    let copy = proc.copy_closure(&mut mem, original).unwrap();

    // Should be a different address
    let Value::Closure(orig_addr) = original else {
        panic!("expected Closure");
    };
    let Value::Closure(copy_addr) = copy else {
        panic!("expected Closure");
    };
    assert_ne!(orig_addr, copy_addr);

    // Should have same header values
    let orig_header = proc.read_closure(&mem, original).unwrap();
    let copy_header = proc.read_closure(&mem, copy).unwrap();
    assert_eq!(orig_header.captures_len, copy_header.captures_len);

    // Function pointer should point to a COPY of the function, not the same one
    // (deep copy semantics)
    assert_ne!(orig_header.function, copy_header.function);

    // Captures should have same values
    for i in 0..captures.len() {
        let orig_cap = proc.read_closure_capture(&mem, original, i).unwrap();
        let copy_cap = proc.read_closure_capture(&mem, copy, i).unwrap();
        assert_eq!(orig_cap, copy_cap);
    }
}

#[test]
fn copy_closure_empty_captures() {
    let (mut proc, mut mem) = setup();

    // Create function
    let func = proc
        .alloc_compiled_fn(&mut mem, 0, false, 0, &[], &[])
        .unwrap();

    let Value::CompiledFn(func_addr) = func else {
        panic!("expected CompiledFn");
    };

    // Create closure with no captures
    let original = proc.alloc_closure(&mut mem, func_addr, &[]).unwrap();

    let copy = proc.copy_closure(&mut mem, original).unwrap();

    let Value::Closure(orig_addr) = original else {
        panic!("expected Closure");
    };
    let Value::Closure(copy_addr) = copy else {
        panic!("expected Closure");
    };
    assert_ne!(orig_addr, copy_addr);

    let copy_header = proc.read_closure(&mem, copy).unwrap();
    assert_eq!(copy_header.captures_len, 0);
}

#[test]
fn copy_compiled_fn_with_heap_constants() {
    let (mut proc, mut mem) = setup();

    // Create constants that are heap-allocated
    let str_const = proc.alloc_string(&mut mem, "hello").unwrap();
    let sym_const = proc.alloc_symbol(&mut mem, "world").unwrap();

    let code = [0x1234_u32];
    let constants = [str_const, sym_const, Value::int(42)];
    let original = proc
        .alloc_compiled_fn(&mut mem, 0, false, 0, &code, &constants)
        .unwrap();

    // Copy it
    let copy = proc.copy_compiled_fn(&mut mem, original).unwrap();

    // Verify constants are preserved (by value, not by address for immediates)
    let orig_const0 = proc.read_compiled_fn_constant(&mem, original, 0).unwrap();
    let copy_const0 = proc.read_compiled_fn_constant(&mem, copy, 0).unwrap();

    // For heap values, addresses should be the same (shallow copy of constants)
    // This is correct behavior - constants are shared, not deep copied
    assert_eq!(orig_const0, copy_const0);

    // Integer constant should be equal
    let orig_int = proc.read_compiled_fn_constant(&mem, original, 2).unwrap();
    let copy_int = proc.read_compiled_fn_constant(&mem, copy, 2).unwrap();
    assert_eq!(orig_int, copy_int);
}
