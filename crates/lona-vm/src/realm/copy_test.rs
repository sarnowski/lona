// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for deep copy to realm functionality.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::items_after_statements
)]

use crate::Vaddr;
use crate::platform::{MemorySpace, MockVSpace};
use crate::process::Process;
use crate::realm::{Realm, VisitedTracker, deep_copy_to_realm};
use crate::value::{HeapString, HeapTuple, Pair, Value};

/// Create a test setup with process, realm, and memory.
fn setup() -> (Process, Realm, MockVSpace) {
    // Memory layout:
    // 0x1000-0x2000: Process young heap
    // 0x2000-0x3000: Process old heap
    // 0x4000-0x8000: Realm code region
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x1000));
    let proc = Process::new(1, Vaddr::new(0x1000), 0x1000, Vaddr::new(0x2000), 0x1000);
    let realm = Realm::new(Vaddr::new(0x4000), 0x4000);
    (proc, realm, mem)
}

#[test]
fn test_visited_tracker_basic() {
    let mut tracker = VisitedTracker::new();

    // Initially empty
    assert!(tracker.check(Vaddr::new(0x1000)).is_none());

    // Record a mapping
    assert!(tracker.record(Vaddr::new(0x1000), Vaddr::new(0x4000)));

    // Should find it now
    assert_eq!(tracker.check(Vaddr::new(0x1000)), Some(Vaddr::new(0x4000)));

    // Different address should not be found
    assert!(tracker.check(Vaddr::new(0x1100)).is_none());
}

#[test]
fn test_deep_copy_immediates() {
    let (_, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Nil
    let result = deep_copy_to_realm(Value::Nil, &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(Value::Nil));

    // Bool
    let result = deep_copy_to_realm(Value::Bool(true), &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(Value::Bool(true)));

    // Int
    let result = deep_copy_to_realm(Value::Int(42), &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(Value::Int(42)));

    // NativeFn
    let result = deep_copy_to_realm(Value::NativeFn(5), &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(Value::NativeFn(5)));

    // Unbound
    let result = deep_copy_to_realm(Value::Unbound, &mut realm, &mut mem, &mut visited);
    assert_eq!(result, Some(Value::Unbound));
}

#[test]
fn test_deep_copy_string() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Allocate string on process heap
    let src = proc.alloc_string(&mut mem, "hello").unwrap();

    // Copy to realm
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a string at different address
    assert!(dst.is_string());
    let Value::String(src_addr) = src else {
        panic!()
    };
    let Value::String(dst_addr) = dst else {
        panic!()
    };
    assert_ne!(src_addr, dst_addr);

    // Contents should match
    let src_header: HeapString = mem.read(src_addr);
    let dst_header: HeapString = mem.read(dst_addr);
    assert_eq!(src_header.len, dst_header.len);

    let src_data = src_addr.add(HeapString::HEADER_SIZE as u64);
    let dst_data = dst_addr.add(HeapString::HEADER_SIZE as u64);
    let src_bytes = mem.slice(src_data, src_header.len as usize);
    let dst_bytes = mem.slice(dst_data, dst_header.len as usize);
    assert_eq!(src_bytes, dst_bytes);
    assert_eq!(src_bytes, b"hello");

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);
    assert!(dst_addr.as_u64() < 0x8000);
}

#[test]
fn test_deep_copy_symbol() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Allocate symbol on process heap
    let src = proc.alloc_symbol(&mut mem, "foo").unwrap();

    // Copy to realm (symbols get re-interned)
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a symbol
    assert!(dst.is_symbol());
    let Value::Symbol(dst_addr) = dst else {
        panic!()
    };

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);

    // Copying same symbol again should return same interned address
    let src2 = proc.alloc_symbol(&mut mem, "foo").unwrap();
    let dst2 = deep_copy_to_realm(src2, &mut realm, &mut mem, &mut visited).unwrap();
    assert_eq!(dst, dst2);
}

#[test]
fn test_deep_copy_keyword() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Allocate keyword on process heap
    let src = proc.alloc_keyword(&mut mem, "bar").unwrap();

    // Copy to realm (keywords get re-interned)
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a keyword
    assert!(dst.is_keyword());
    let Value::Keyword(dst_addr) = dst else {
        panic!()
    };

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_pair() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create a simple pair: (1 . 2)
    let src = proc
        .alloc_pair(&mut mem, Value::Int(1), Value::Int(2))
        .unwrap();

    // Copy to realm
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a pair at different address
    assert!(dst.is_pair());
    let Value::Pair(src_addr) = src else { panic!() };
    let Value::Pair(dst_addr) = dst else { panic!() };
    assert_ne!(src_addr, dst_addr);

    // Contents should match
    let src_pair: Pair = mem.read(src_addr);
    let dst_pair: Pair = mem.read(dst_addr);
    assert_eq!(src_pair.first, dst_pair.first);
    assert_eq!(src_pair.rest, dst_pair.rest);

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_nested_pair() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create nested pair: (1 2 3) = (1 . (2 . (3 . nil)))
    let inner = proc
        .alloc_pair(&mut mem, Value::Int(3), Value::Nil)
        .unwrap();
    let middle = proc.alloc_pair(&mut mem, Value::Int(2), inner).unwrap();
    let src = proc.alloc_pair(&mut mem, Value::Int(1), middle).unwrap();

    // Copy to realm
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Walk the copied list and verify structure
    let Value::Pair(p1_addr) = dst else { panic!() };
    let p1: Pair = mem.read(p1_addr);
    assert_eq!(p1.first, Value::Int(1));

    let Value::Pair(p2_addr) = p1.rest else {
        panic!()
    };
    let p2: Pair = mem.read(p2_addr);
    assert_eq!(p2.first, Value::Int(2));

    let Value::Pair(p3_addr) = p2.rest else {
        panic!()
    };
    let p3: Pair = mem.read(p3_addr);
    assert_eq!(p3.first, Value::Int(3));
    assert_eq!(p3.rest, Value::Nil);

    // All pair addresses should be in realm region
    assert!(p1_addr.as_u64() >= 0x4000);
    assert!(p2_addr.as_u64() >= 0x4000);
    assert!(p3_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_tuple() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create tuple [1 2 3]
    let elements = [Value::Int(1), Value::Int(2), Value::Int(3)];
    let src = proc.alloc_tuple(&mut mem, &elements).unwrap();

    // Copy to realm
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a tuple at different address
    assert!(dst.is_tuple());
    let Value::Tuple(src_addr) = src else {
        panic!()
    };
    let Value::Tuple(dst_addr) = dst else {
        panic!()
    };
    assert_ne!(src_addr, dst_addr);

    // Check header
    let dst_header: HeapTuple = mem.read(dst_addr);
    assert_eq!(dst_header.len, 3);

    // Check elements
    let elem_base = dst_addr.add(HeapTuple::HEADER_SIZE as u64);
    for (i, expected) in elements.iter().enumerate() {
        let elem: Value = mem.read(elem_base.add((i * core::mem::size_of::<Value>()) as u64));
        assert_eq!(elem, *expected);
    }

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_tuple_with_nested_values() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create tuple with string element: [1 "hello" 3]
    let s = proc.alloc_string(&mut mem, "hello").unwrap();
    let elements = [Value::Int(1), s, Value::Int(3)];
    let src = proc.alloc_tuple(&mut mem, &elements).unwrap();

    // Copy to realm
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Check the string element was deep copied
    let Value::Tuple(dst_addr) = dst else {
        panic!()
    };
    let elem_base = dst_addr.add(HeapTuple::HEADER_SIZE as u64);
    let copied_str: Value = mem.read(elem_base.add(core::mem::size_of::<Value>() as u64));

    assert!(copied_str.is_string());
    let Value::String(str_addr) = copied_str else {
        panic!()
    };

    // String should be in realm region
    assert!(str_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_map() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create a simple map %{:a 1}
    // First create the key-value tuple
    let key = proc.alloc_keyword(&mut mem, "a").unwrap();
    let kv_tuple = proc.alloc_tuple(&mut mem, &[key, Value::Int(1)]).unwrap();

    // Create a pair for the entry
    let entry = proc.alloc_pair(&mut mem, kv_tuple, Value::Nil).unwrap();

    // Create the map with entries
    let src = proc.alloc_map(&mut mem, entry).unwrap();

    // Copy to realm
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a map at different address
    assert!(dst.is_map());
    let Value::Map(dst_addr) = dst else { panic!() };

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_compiled_fn() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create a simple function
    let code = [0x01, 0x02, 0x03];
    let constants = [Value::Int(42)];
    let src = proc
        .alloc_compiled_fn(&mut mem, 1, false, 0, &code, &constants)
        .unwrap();

    // Copy to realm
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a compiled function at different address
    assert!(dst.is_compiled_fn());
    let Value::CompiledFn(src_addr) = src else {
        panic!()
    };
    let Value::CompiledFn(dst_addr) = dst else {
        panic!()
    };
    assert_ne!(src_addr, dst_addr);

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);

    // Verify bytecode was copied
    use crate::value::HeapCompiledFn;
    let dst_header: HeapCompiledFn = mem.read(dst_addr);
    assert_eq!(dst_header.arity, 1);
    assert_eq!(dst_header.code_len, 3);
    assert_eq!(dst_header.constants_len, 1);

    // Check bytecode
    let code_addr = dst_addr.add(HeapCompiledFn::bytecode_offset() as u64);
    for (i, expected) in code.iter().enumerate() {
        let instr: u32 = mem.read(code_addr.add((i * core::mem::size_of::<u32>()) as u64));
        assert_eq!(instr, *expected);
    }

    // Check constants
    let const_addr = dst_addr.add(HeapCompiledFn::constants_offset(3) as u64);
    let copied_const: Value = mem.read(const_addr);
    assert_eq!(copied_const, Value::Int(42));
}

#[test]
fn test_deep_copy_closure() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create a function for the closure
    let code = [0x01, 0x02];
    let constants: [Value; 0] = [];
    let func = proc
        .alloc_compiled_fn(&mut mem, 0, false, 0, &code, &constants)
        .unwrap();
    let Value::CompiledFn(func_addr) = func else {
        panic!()
    };

    // Create closure with captures
    let captures = [Value::Int(10), Value::Int(20)];
    let src = proc.alloc_closure(&mut mem, func_addr, &captures).unwrap();

    // Copy to realm
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Should be a closure at different address
    assert!(dst.is_closure());
    let Value::Closure(src_addr) = src else {
        panic!()
    };
    let Value::Closure(dst_addr) = dst else {
        panic!()
    };
    assert_ne!(src_addr, dst_addr);

    // Destination should be in realm region
    assert!(dst_addr.as_u64() >= 0x4000);

    // Verify closure structure
    use crate::value::HeapClosure;
    let dst_header: HeapClosure = mem.read(dst_addr);
    assert_eq!(dst_header.captures_len, 2);

    // The function pointer should also be in realm region
    assert!(dst_header.function.as_u64() >= 0x4000);

    // Check captures
    let cap_addr = dst_addr.add(HeapClosure::captures_offset() as u64);
    let cap0: Value = mem.read(cap_addr);
    let cap1: Value = mem.read(cap_addr.add(core::mem::size_of::<Value>() as u64));
    assert_eq!(cap0, Value::Int(10));
    assert_eq!(cap1, Value::Int(20));
}

#[test]
fn test_deep_copy_shared_structure() {
    let (mut proc, mut realm, mut mem) = setup();
    let mut visited = VisitedTracker::new();

    // Create a string that's shared in two places
    let shared_str = proc.alloc_string(&mut mem, "shared").unwrap();

    // Create two pairs that share the same string
    let pair1 = proc.alloc_pair(&mut mem, shared_str, Value::Nil).unwrap();
    let pair2 = proc.alloc_pair(&mut mem, shared_str, Value::Nil).unwrap();

    // Create a tuple containing both pairs
    let src = proc.alloc_tuple(&mut mem, &[pair1, pair2]).unwrap();

    // Copy to realm
    let dst = deep_copy_to_realm(src, &mut realm, &mut mem, &mut visited).unwrap();

    // Extract the copied pairs
    let Value::Tuple(tuple_addr) = dst else {
        panic!()
    };
    let elem_base = tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
    let copied_pair1: Value = mem.read(elem_base);
    let copied_pair2: Value = mem.read(elem_base.add(core::mem::size_of::<Value>() as u64));

    // Extract the strings from each pair
    let Value::Pair(p1_addr) = copied_pair1 else {
        panic!()
    };
    let Value::Pair(p2_addr) = copied_pair2 else {
        panic!()
    };
    let p1: Pair = mem.read(p1_addr);
    let p2: Pair = mem.read(p2_addr);

    // Both pairs should have the SAME string address (shared structure preserved)
    assert_eq!(p1.first, p2.first);

    // And it should be in realm region
    let Value::String(str_addr) = p1.first else {
        panic!()
    };
    assert!(str_addr.as_u64() >= 0x4000);
}

#[test]
fn test_deep_copy_oom() {
    // Create a very small realm that will run out of memory
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x1000));
    let proc = Process::new(1, Vaddr::new(0x1000), 0x1000, Vaddr::new(0x2000), 0x1000);
    // Realm with only 8 bytes - definitely too small for any heap allocation
    // HeapString header alone is 8 bytes, plus we need alignment
    let mut realm = Realm::new(Vaddr::new(0x4000), 8);
    let mut mem = mem;
    let mut proc = proc;
    let mut visited = VisitedTracker::new();

    // Allocate a string on process heap - needs header (8 bytes) + content
    let s = proc.alloc_string(&mut mem, "hello world").unwrap();

    // Try to copy to realm - should fail due to OOM
    // String needs 8 bytes header + 11 bytes content = 19 bytes minimum
    let result = deep_copy_to_realm(s, &mut realm, &mut mem, &mut visited);
    assert!(
        result.is_none(),
        "deep copy should fail when realm is out of memory"
    );
}

#[test]
fn test_deep_copy_large_nested_structure_oom() {
    // Create a realm with limited space
    let mem = MockVSpace::new(0x10000, Vaddr::new(0x1000));
    let proc = Process::new(1, Vaddr::new(0x1000), 0x1000, Vaddr::new(0x2000), 0x1000);
    // Realm with 256 bytes - enough for a few small allocations but not a large structure
    let mut realm = Realm::new(Vaddr::new(0x4000), 256);
    let mut mem = mem;
    let mut proc = proc;
    let mut visited = VisitedTracker::new();

    // Create a deeply nested structure that will exhaust realm memory
    let s1 = proc.alloc_string(&mut mem, "string1").unwrap();
    let s2 = proc.alloc_string(&mut mem, "string2").unwrap();
    let s3 = proc.alloc_string(&mut mem, "string3").unwrap();
    let s4 = proc.alloc_string(&mut mem, "string4").unwrap();

    // Create nested pairs
    let p1 = proc.alloc_pair(&mut mem, s1, Value::Nil).unwrap();
    let p2 = proc.alloc_pair(&mut mem, s2, p1).unwrap();
    let p3 = proc.alloc_pair(&mut mem, s3, p2).unwrap();
    let nested = proc.alloc_pair(&mut mem, s4, p3).unwrap();

    // Create tuple with multiple nested elements
    let elements = [nested, nested, nested, nested];
    let large_tuple = proc.alloc_tuple(&mut mem, &elements).unwrap();

    // Try to copy to realm - should fail due to OOM at some point
    let result = deep_copy_to_realm(large_tuple, &mut realm, &mut mem, &mut visited);
    assert!(
        result.is_none(),
        "deep copy of large structure should fail when realm runs out of memory"
    );
}
