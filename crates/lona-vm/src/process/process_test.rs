// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the Process module.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::platform::MockVSpace;

/// Create a test process with `MockVSpace`.
fn setup() -> (Process, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mem_size = 128 * 1024;
    let mem = MockVSpace::new(mem_size, base);

    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;

    let proc = Process::new(1, young_base, young_size, old_base, old_size);
    (proc, mem)
}

// --- Basic allocation tests ---

#[test]
fn process_initial_state() {
    let (proc, _mem) = setup();

    assert_eq!(proc.pid, 1);
    assert_eq!(proc.status, ProcessStatus::Ready);
    assert_eq!(proc.ip, 0);
    assert!(proc.chunk.is_none());

    // Young heap should be empty initially
    assert_eq!(proc.htop, proc.heap);
    assert_eq!(proc.stop, proc.hend);
    assert_eq!(proc.heap_used(), 0);
    assert_eq!(proc.stack_used(), 0);

    // Old heap should be empty
    assert_eq!(proc.old_htop, proc.old_heap);
}

#[test]
fn alloc_basic() {
    let (mut proc, _mem) = setup();

    // Allocate 100 bytes
    let addr = proc.alloc(100, 1).unwrap();
    assert_eq!(addr, proc.heap);
    assert_eq!(proc.heap_used(), 100);
    assert_eq!(proc.htop, proc.heap.add(100));
}

#[test]
fn alloc_aligned() {
    let (mut proc, _mem) = setup();

    // First allocation: 5 bytes
    let addr1 = proc.alloc(5, 1).unwrap();
    assert_eq!(addr1, proc.heap);
    assert_eq!(proc.htop, proc.heap.add(5));

    // Second allocation: 16 bytes, 8-byte aligned
    let addr2 = proc.alloc(16, 8).unwrap();
    // Should be aligned to 8-byte boundary
    assert_eq!(addr2.as_u64() % 8, 0);
    assert!(addr2.as_u64() >= proc.heap.as_u64() + 5);
}

#[test]
fn alloc_zero() {
    let (mut proc, _mem) = setup();

    // Zero-size allocation should succeed and not change htop
    let addr = proc.alloc(0, 1).unwrap();
    assert_eq!(addr, proc.heap);
    assert_eq!(proc.heap_used(), 0);
}

#[test]
fn alloc_multiple() {
    let (mut proc, _mem) = setup();

    // Allocate several chunks
    let addr1 = proc.alloc(100, 1).unwrap();
    let addr2 = proc.alloc(200, 1).unwrap();
    let addr3 = proc.alloc(300, 1).unwrap();

    // They should be sequential
    assert_eq!(addr1, proc.heap);
    assert_eq!(addr2, proc.heap.add(100));
    assert_eq!(addr3, proc.heap.add(300));

    assert_eq!(proc.heap_used(), 600);
}

#[test]
fn alloc_oom() {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(256, base);
    let mut proc = Process::new(1, base, 100, base.add(100), 50);

    // First allocation should succeed
    let addr1 = proc.alloc(50, 1);
    assert!(addr1.is_some());

    // Second allocation should succeed
    let addr2 = proc.alloc(40, 1);
    assert!(addr2.is_some());

    // Third allocation (would exceed) should fail
    let addr3 = proc.alloc(20, 1);
    assert!(addr3.is_none());

    let _ = mem; // Suppress unused warning
}

// --- Stack tests ---

#[test]
fn stack_push_basic() {
    let (mut proc, _mem) = setup();

    let initial_stop = proc.stop;

    // Push 100 bytes onto stack
    let addr = proc.stack_push(100, 1).unwrap();

    // Stack grows down, so addr should be less than initial stop
    assert!(addr.as_u64() < initial_stop.as_u64());
    assert_eq!(proc.stack_used(), 100);
}

#[test]
fn stack_push_aligned() {
    let (mut proc, _mem) = setup();

    // First push: 5 bytes
    let addr1 = proc.stack_push(5, 1).unwrap();
    assert_eq!(proc.stack_used(), 5);

    // Second push: 16 bytes, 8-byte aligned
    let addr2 = proc.stack_push(16, 8).unwrap();
    // Should be aligned
    assert_eq!(addr2.as_u64() % 8, 0);
    assert!(addr2.as_u64() < addr1.as_u64());

    let _ = addr1; // Suppress unused warning
}

#[test]
fn stack_pop() {
    let (mut proc, _mem) = setup();

    let initial_stop = proc.stop;

    // Push and then pop
    proc.stack_push(100, 1).unwrap();
    assert_eq!(proc.stack_used(), 100);

    proc.stack_pop(100);
    assert_eq!(proc.stop, initial_stop);
    assert_eq!(proc.stack_used(), 0);
}

#[test]
fn stack_pop_partial() {
    let (mut proc, _mem) = setup();

    // Push 100 bytes
    proc.stack_push(100, 1).unwrap();

    // Pop only 30
    proc.stack_pop(30);

    // Should have 70 bytes remaining on stack
    assert_eq!(proc.stack_used(), 70);
}

#[test]
fn heap_stack_collision() {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(256, base);
    let mut proc = Process::new(1, base, 100, base.add(100), 50);

    // Allocate most of the heap
    proc.alloc(40, 1).unwrap();

    // Push most of the stack
    proc.stack_push(40, 1).unwrap();

    // Free space should be reduced
    assert_eq!(proc.free_space(), 20);

    // Another large allocation should fail (would collide)
    assert!(proc.alloc(30, 1).is_none());
    assert!(proc.stack_push(30, 1).is_none());

    let _ = mem; // Suppress unused warning
}

// --- Value allocation tests ---

#[test]
fn alloc_string() {
    let (mut proc, mut mem) = setup();

    let value = proc.alloc_string(&mut mem, "hello").unwrap();
    assert!(matches!(value, Value::String(_)));

    let s = proc.read_string(&mem, value).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn alloc_string_empty() {
    let (mut proc, mut mem) = setup();

    let value = proc.alloc_string(&mut mem, "").unwrap();
    let s = proc.read_string(&mem, value).unwrap();
    assert_eq!(s, "");
}

#[test]
fn alloc_string_unicode() {
    let (mut proc, mut mem) = setup();

    let value = proc.alloc_string(&mut mem, "你好世界").unwrap();
    let s = proc.read_string(&mem, value).unwrap();
    assert_eq!(s, "你好世界");
}

#[test]
fn alloc_pair() {
    let (mut proc, mut mem) = setup();

    let first = Value::int(1);
    let rest = Value::int(2);
    let value = proc.alloc_pair(&mut mem, first, rest).unwrap();

    assert!(matches!(value, Value::Pair(_)));

    let pair = proc.read_pair(&mem, value).unwrap();
    assert_eq!(pair.first, Value::int(1));
    assert_eq!(pair.rest, Value::int(2));
}

#[test]
fn alloc_list() {
    let (mut proc, mut mem) = setup();

    // Build list (1 2 3)
    let v3 = proc
        .alloc_pair(&mut mem, Value::int(3), Value::Nil)
        .unwrap();
    let v2 = proc.alloc_pair(&mut mem, Value::int(2), v3).unwrap();
    let v1 = proc.alloc_pair(&mut mem, Value::int(1), v2).unwrap();

    // Read back
    let p1 = proc.read_pair(&mem, v1).unwrap();
    assert_eq!(p1.first, Value::int(1));

    let p2 = proc.read_pair(&mem, p1.rest).unwrap();
    assert_eq!(p2.first, Value::int(2));

    let p3 = proc.read_pair(&mem, p2.rest).unwrap();
    assert_eq!(p3.first, Value::int(3));
    assert_eq!(p3.rest, Value::Nil);
}

#[test]
fn alloc_symbol() {
    let (mut proc, mut mem) = setup();

    let value = proc.alloc_symbol(&mut mem, "foo").unwrap();
    assert!(matches!(value, Value::Symbol(_)));

    let name = proc.read_string(&mem, value).unwrap();
    assert_eq!(name, "foo");
}

// --- ProcessPool tests ---

mod pool_tests {
    use super::*;
    use crate::process::pool::ProcessPool;

    #[test]
    fn pool_initial_state() {
        let base = Vaddr::new(0x1_0000);
        let pool = ProcessPool::new(base, 1024);

        assert_eq!(pool.next(), base);
        assert_eq!(pool.limit(), base.add(1024));
        assert_eq!(pool.remaining(), 1024);
    }

    #[test]
    fn pool_allocate_process() {
        let base = Vaddr::new(0x1_0000);
        let mut pool = ProcessPool::new(base, 1024);

        let (young_base, old_base) = pool.allocate_process_memory(512, 256).unwrap();

        assert_eq!(young_base, base);
        assert_eq!(old_base, base.add(512));
        assert_eq!(pool.remaining(), 256);
    }

    #[test]
    fn pool_allocate_multiple() {
        let base = Vaddr::new(0x1_0000);
        let mut pool = ProcessPool::new(base, 2048);

        // First process
        let (young1, old1) = pool.allocate_process_memory(512, 256).unwrap();
        assert_eq!(young1, base);
        assert_eq!(old1, base.add(512));

        // Second process
        let (young2, old2) = pool.allocate_process_memory(512, 256).unwrap();
        assert_eq!(young2, base.add(768));
        assert_eq!(old2, base.add(1280));

        assert_eq!(pool.remaining(), 512);
    }

    #[test]
    fn pool_oom() {
        let base = Vaddr::new(0x1_0000);
        let mut pool = ProcessPool::new(base, 100);

        // Should fail - not enough space
        let result = pool.allocate_process_memory(80, 40);
        assert!(result.is_none());
    }
}

// --- Execution state tests ---

#[test]
fn process_set_chunk() {
    use crate::bytecode::{Chunk, encode_abx, op};

    let (mut proc, _mem) = setup();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42));
    chunk.emit(encode_abx(op::HALT, 0, 0));

    proc.set_chunk(chunk);

    assert!(proc.chunk.is_some());
    assert_eq!(proc.ip, 0);
}

#[test]
fn process_reset() {
    let (mut proc, _mem) = setup();

    // Modify state
    proc.ip = 100;
    proc.x_regs[0] = Value::int(42);
    proc.status = ProcessStatus::Running;

    // Reset
    proc.reset();

    assert_eq!(proc.ip, 0);
    assert_eq!(proc.x_regs[0], Value::Nil);
    assert_eq!(proc.status, ProcessStatus::Ready);
}

// --- Namespace tests ---

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
