// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for deep copy message passing functions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::Process;
use super::deep_copy::{deep_copy_message_to_fragment, deep_copy_message_to_process};
use super::heap_fragment::HeapFragment;
use crate::Vaddr;
use crate::platform::{MemorySpace, MockVSpace};
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::{HeapFun, HeapPair, HeapTuple, HeapVector};
use crate::term::tag::object;

/// Create two processes in the same `MockVSpace`.
fn setup_two_processes() -> (MockVSpace, Process, Process) {
    let mem = MockVSpace::new(256 * 1024, Vaddr::new(0x1_0000));
    let proc_a = Process::new(Vaddr::new(0x1_0000), 4096, Vaddr::new(0x1_1000), 1024);
    let proc_b = Process::new(Vaddr::new(0x2_0000), 4096, Vaddr::new(0x2_1000), 1024);
    (mem, proc_a, proc_b)
}

// --- Immediate terms ---

#[test]
fn copy_immediate_int() {
    let (mut mem, _, mut dst) = setup_two_processes();
    let val = Term::small_int(42).unwrap();
    let copied = deep_copy_message_to_process(val, &mut dst, &mut mem).unwrap();
    assert_eq!(copied, val); // Same immediate value, no allocation
}

#[test]
fn copy_nil() {
    let (mut mem, _, mut dst) = setup_two_processes();
    let copied = deep_copy_message_to_process(Term::NIL, &mut dst, &mut mem).unwrap();
    assert_eq!(copied, Term::NIL);
}

#[test]
fn copy_boolean() {
    let (mut mem, _, mut dst) = setup_two_processes();
    let copied = deep_copy_message_to_process(Term::TRUE, &mut dst, &mut mem).unwrap();
    assert_eq!(copied, Term::TRUE);
}

#[test]
fn copy_keyword() {
    let (mut mem, _, mut dst) = setup_two_processes();
    let kw = Term::keyword(5);
    let copied = deep_copy_message_to_process(kw, &mut dst, &mut mem).unwrap();
    assert_eq!(copied, kw);
}

// --- Heap-allocated terms ---

#[test]
fn copy_string_between_processes() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    // Allocate a string on src's heap
    let original = src.alloc_term_string(&mut mem, "hello").unwrap();
    assert!(original.is_boxed());

    // Copy to dst
    let copied = deep_copy_message_to_process(original, &mut dst, &mut mem).unwrap();
    assert!(copied.is_boxed());
    assert_ne!(original.to_vaddr(), copied.to_vaddr()); // Different addresses

    // Verify content
    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.object_tag(), object::STRING);
    assert_eq!(header.arity(), 5); // "hello" = 5 bytes
}

#[test]
fn copy_tuple_between_processes() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    let elements = [
        Term::small_int(1).unwrap(),
        Term::small_int(2).unwrap(),
        Term::small_int(3).unwrap(),
    ];
    let original = src.alloc_term_tuple(&mut mem, &elements).unwrap();

    let copied = deep_copy_message_to_process(original, &mut dst, &mut mem).unwrap();
    assert!(copied.is_boxed());
    assert_ne!(original.to_vaddr(), copied.to_vaddr());

    // Verify elements
    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.object_tag(), object::TUPLE);
    assert_eq!(header.arity(), 3);

    let data_base = copied.to_vaddr().add(HeapTuple::HEADER_SIZE as u64);
    for (i, expected) in elements.iter().enumerate() {
        let elem: Term = mem.read(data_base.add((i * 8) as u64));
        assert_eq!(elem, *expected);
    }
}

#[test]
fn copy_nested_tuple_with_string() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    let s = src.alloc_term_string(&mut mem, "nested").unwrap();
    let inner = src
        .alloc_term_tuple(&mut mem, &[Term::small_int(99).unwrap(), s])
        .unwrap();
    let outer = src
        .alloc_term_tuple(&mut mem, &[Term::TRUE, inner])
        .unwrap();

    let copied = deep_copy_message_to_process(outer, &mut dst, &mut mem).unwrap();

    // Outer tuple
    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.arity(), 2);

    // Element 0 = true (immediate)
    let e0: Term = mem.read(copied.to_vaddr().add(HeapTuple::HEADER_SIZE as u64));
    assert_eq!(e0, Term::TRUE);

    // Element 1 = inner tuple (boxed, different address from original)
    let e1: Term = mem.read(copied.to_vaddr().add((HeapTuple::HEADER_SIZE + 8) as u64));
    assert!(e1.is_boxed());
    assert_ne!(e1.to_vaddr(), inner.to_vaddr());
}

#[test]
fn copy_list_pair() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    let pair = src
        .alloc_term_pair(&mut mem, Term::small_int(1).unwrap(), Term::NIL)
        .unwrap();
    assert!(pair.is_list());

    let copied = deep_copy_message_to_process(pair, &mut dst, &mut mem).unwrap();
    assert!(copied.is_list());
    assert_ne!(pair.to_vaddr(), copied.to_vaddr());

    let cp: HeapPair = mem.read(copied.to_vaddr());
    assert_eq!(cp.head, Term::small_int(1).unwrap());
    assert_eq!(cp.tail, Term::NIL);
}

// --- Fragment copy ---

#[test]
fn copy_to_fragment() {
    let (mut mem, mut src, _) = setup_two_processes();

    // Fragment memory in the MockVSpace range
    let frag_base = Vaddr::new(0x3_0000);
    let mut frag = HeapFragment::new(frag_base, 1024);

    let original = src.alloc_term_string(&mut mem, "fragmented").unwrap();

    let copied = deep_copy_message_to_fragment(original, &mut frag, &mut mem).unwrap();
    assert!(copied.is_boxed());

    // Copied address should be within fragment's region
    let copied_addr = copied.to_vaddr().as_u64();
    assert!(copied_addr >= frag_base.as_u64());
    assert!(copied_addr < frag_base.as_u64() + 1024);

    // Verify content
    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.object_tag(), object::STRING);
    assert_eq!(header.arity(), 10); // "fragmented" = 10 bytes
}

#[test]
fn copy_to_fragment_oom() {
    let (mut mem, mut src, _) = setup_two_processes();

    // Tiny fragment — won't fit a string
    let frag_base = Vaddr::new(0x3_0000);
    let mut frag = HeapFragment::new(frag_base, 4);

    let original = src.alloc_term_string(&mut mem, "too long").unwrap();
    let result = deep_copy_message_to_fragment(original, &mut frag, &mut mem);
    assert!(result.is_none());
}

// --- PID copy ---

#[test]
fn copy_pid_between_processes() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    let pid_term = src.alloc_term_pid(&mut mem, 5, 3).unwrap();
    let copied = deep_copy_message_to_process(pid_term, &mut dst, &mut mem).unwrap();

    assert!(copied.is_boxed());
    assert_ne!(pid_term.to_vaddr(), copied.to_vaddr());

    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.object_tag(), object::PID);
}

// --- Vector copy ---

#[test]
fn copy_vector_between_processes() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    let elements = [
        Term::small_int(10).unwrap(),
        Term::small_int(20).unwrap(),
        Term::small_int(30).unwrap(),
    ];
    let original = src.alloc_term_vector(&mut mem, &elements).unwrap();

    let copied = deep_copy_message_to_process(original, &mut dst, &mut mem).unwrap();
    assert!(copied.is_boxed());
    assert_ne!(original.to_vaddr(), copied.to_vaddr());

    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.object_tag(), object::VECTOR);
    assert_eq!(header.arity(), 3); // capacity matches length

    // Verify element values
    let data_base = copied.to_vaddr().add(HeapVector::PREFIX_SIZE as u64);
    for (i, expected) in elements.iter().enumerate() {
        let elem: Term = mem.read(data_base.add((i * 8) as u64));
        assert_eq!(elem, *expected);
    }
}

// --- Map copy ---

#[test]
fn copy_map_between_processes() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    // Build a simple map with one entry: {:a 1}
    let key = Term::keyword(0);
    let val = Term::small_int(1).unwrap();
    let entry = src.alloc_term_tuple(&mut mem, &[key, val]).unwrap();
    let entries = src.alloc_term_pair(&mut mem, entry, Term::NIL).unwrap();
    let original = src.alloc_term_map(&mut mem, entries, 1).unwrap();

    let copied = deep_copy_message_to_process(original, &mut dst, &mut mem).unwrap();
    assert!(copied.is_boxed());
    assert_ne!(original.to_vaddr(), copied.to_vaddr());

    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.object_tag(), object::MAP);
}

// --- Float copy ---

#[test]
fn copy_float_between_processes() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    let original = src.alloc_term_float(&mut mem, 1.234_567_89).unwrap();

    let copied = deep_copy_message_to_process(original, &mut dst, &mut mem).unwrap();
    assert!(copied.is_boxed());
    assert_ne!(original.to_vaddr(), copied.to_vaddr());

    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.object_tag(), object::FLOAT);

    // Verify the float value survived the copy
    let src_bits: u64 = mem.read(original.to_vaddr().add(8));
    let dst_bits: u64 = mem.read(copied.to_vaddr().add(8));
    assert_eq!(src_bits, dst_bits);
}

// --- Fun copy ---

#[test]
fn copy_fun_between_processes() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    // Create a simple function with bytecode and one constant
    let constant = Term::small_int(99).unwrap();
    let original = src
        .alloc_term_compiled_fn(&mut mem, 0, false, 0, &[0x0001_0002], &[constant])
        .unwrap();

    let copied = deep_copy_message_to_process(original, &mut dst, &mut mem).unwrap();
    assert!(copied.is_boxed());
    assert_ne!(original.to_vaddr(), copied.to_vaddr());

    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.object_tag(), object::FUN);

    // Verify bytecode survived
    let src_word: u32 = mem.read(original.to_vaddr().add(HeapFun::PREFIX_SIZE as u64));
    let dst_word: u32 = mem.read(copied.to_vaddr().add(HeapFun::PREFIX_SIZE as u64));
    assert_eq!(src_word, dst_word);
}

// --- Closure copy ---

#[test]
fn copy_closure_between_processes() {
    let (mut mem, mut src, mut dst) = setup_two_processes();

    // Create a function to wrap in a closure
    let fun = src
        .alloc_term_compiled_fn(&mut mem, 0, false, 0, &[0x0001], &[])
        .unwrap();

    // Create a closure with one captured value (a string)
    let capture = src.alloc_term_string(&mut mem, "captured").unwrap();
    let original = src.alloc_term_closure(&mut mem, fun, &[capture]).unwrap();

    let copied = deep_copy_message_to_process(original, &mut dst, &mut mem).unwrap();
    assert!(copied.is_boxed());
    assert_ne!(original.to_vaddr(), copied.to_vaddr());

    let header: Header = mem.read(copied.to_vaddr());
    assert_eq!(header.object_tag(), object::CLOSURE);
}

// --- OOM ---

#[test]
fn copy_to_process_oom() {
    let mem_size = 256 * 1024;
    let mut mem = MockVSpace::new(mem_size, Vaddr::new(0x1_0000));

    // Source process with normal heap
    let mut src = Process::new(Vaddr::new(0x1_0000), 4096, Vaddr::new(0x1_1000), 1024);

    // Destination process with tiny heap (16 bytes — too small for any string)
    let mut dst = Process::new(Vaddr::new(0x2_0000), 16, Vaddr::new(0x2_0010), 16);

    let original = src
        .alloc_term_string(
            &mut mem,
            "this string is too long for the tiny heap and should cause OOM",
        )
        .unwrap();
    let result = deep_copy_message_to_process(original, &mut dst, &mut mem);
    assert!(result.is_none());
}
