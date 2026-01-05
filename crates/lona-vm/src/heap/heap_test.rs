// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the heap allocator.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::Heap;
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::value::Value;

#[test]
fn heap_new() {
    let heap = Heap::new(Vaddr::new(0x10000), 0x1000);
    assert_eq!(heap.base().as_u64(), 0x10000);
    assert_eq!(heap.ptr().as_u64(), 0x10000);
    assert_eq!(heap.limit().as_u64(), 0x0F000);
    assert_eq!(heap.remaining(), 0x1000);
    assert_eq!(heap.used(), 0);
}

#[test]
fn heap_alloc_basic() {
    let mut heap = Heap::new(Vaddr::new(0x10000), 0x1000);

    let first = heap.alloc(64, 8);
    assert!(first.is_some());
    let first_addr = first.unwrap();
    assert!(first_addr.as_u64() < 0x10000);
    assert!(first_addr.as_u64() >= heap.limit().as_u64());
    assert!(first_addr.as_u64() % 8 == 0); // Aligned

    let second = heap.alloc(64, 8);
    assert!(second.is_some());
    let second_addr = second.unwrap();
    assert!(second_addr.as_u64() < first_addr.as_u64()); // Grows down
}

#[test]
fn heap_alloc_zero() {
    let mut heap = Heap::new(Vaddr::new(0x10000), 0x1000);
    let addr = heap.alloc(0, 1);
    assert!(addr.is_some());
    assert_eq!(heap.used(), 0); // No space consumed
}

#[test]
fn heap_alloc_oom() {
    let mut heap = Heap::new(Vaddr::new(0x10000), 64);
    let first = heap.alloc(32, 8);
    assert!(first.is_some());

    let second = heap.alloc(64, 8); // Too big
    assert!(second.is_none());
}

#[test]
fn heap_alloc_string() {
    let mut mem = MockVSpace::new(0x1000, Vaddr::new(0x0F000));
    let mut heap = Heap::new(Vaddr::new(0x10000), 0x1000);

    let value = heap.alloc_string(&mut mem, "hello");
    assert!(value.is_some());
    let value = value.unwrap();

    let s = heap.read_string(&mem, value).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn heap_alloc_string_empty() {
    // Use heap top below the memory end to avoid edge case
    let mut mem = MockVSpace::new(0x2000, Vaddr::new(0x0E000));
    let mut heap = Heap::new(Vaddr::new(0x0F000), 0x1000);

    let value = heap.alloc_string(&mut mem, "");
    assert!(value.is_some());
    let value = value.unwrap();

    let s = heap.read_string(&mem, value).unwrap();
    assert_eq!(s, "");
}

#[test]
fn heap_alloc_pair() {
    let mut mem = MockVSpace::new(0x1000, Vaddr::new(0x0F000));
    let mut heap = Heap::new(Vaddr::new(0x10000), 0x1000);

    let value = heap.alloc_pair(&mut mem, Value::int(1), Value::nil());
    assert!(value.is_some());
    let value = value.unwrap();

    let pair = heap.read_pair(&mem, value).unwrap();
    assert_eq!(pair.first, Value::int(1));
    assert_eq!(pair.rest, Value::nil());
}

#[test]
fn heap_alloc_symbol() {
    let mut mem = MockVSpace::new(0x1000, Vaddr::new(0x0F000));
    let mut heap = Heap::new(Vaddr::new(0x10000), 0x1000);

    let value = heap.alloc_symbol(&mut mem, "quote");
    assert!(value.is_some());
    let value = value.unwrap();

    let s = heap.read_string(&mem, value).unwrap();
    assert_eq!(s, "quote");
}

#[test]
fn heap_build_list() {
    // Build (1 2 3) = Pair(1, Pair(2, Pair(3, Nil)))
    let mut mem = MockVSpace::new(0x1000, Vaddr::new(0x0F000));
    let mut heap = Heap::new(Vaddr::new(0x10000), 0x1000);

    let p3 = heap
        .alloc_pair(&mut mem, Value::int(3), Value::nil())
        .unwrap();
    let p2 = heap.alloc_pair(&mut mem, Value::int(2), p3).unwrap();
    let p1 = heap.alloc_pair(&mut mem, Value::int(1), p2).unwrap();

    // Traverse the list
    let pair1 = heap.read_pair(&mem, p1).unwrap();
    assert_eq!(pair1.first, Value::int(1));

    let pair2 = heap.read_pair(&mem, pair1.rest).unwrap();
    assert_eq!(pair2.first, Value::int(2));

    let pair3 = heap.read_pair(&mem, pair2.rest).unwrap();
    assert_eq!(pair3.first, Value::int(3));
    assert!(pair3.rest.is_nil());
}
