// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for GC root finding.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::gc::roots::{
    RootIterator, RootLocation, iterate_roots_with_mem, update_root, update_root_y,
};
use crate::gc::utils::needs_tracing;
use crate::platform::MemorySpace;
use crate::platform::MockVSpace;
use crate::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process, WorkerId};
use crate::scheduler::Worker;
use crate::term::Term;

/// Create a test process and worker with mock memory.
fn setup() -> (Process, Worker, MockVSpace) {
    let young_base = Vaddr::new(0x1000);
    let old_base = Vaddr::new(0x0010_0000);
    let process = Process::new(
        young_base,
        INITIAL_YOUNG_HEAP_SIZE,
        old_base,
        INITIAL_OLD_HEAP_SIZE,
    );
    let worker = Worker::new(WorkerId(0));
    let mem = MockVSpace::new(1024 * 1024, Vaddr::new(0)); // 1 MB mock memory
    (process, worker, mem)
}

// =============================================================================
// X Register Root Tests
// =============================================================================

#[test]
fn root_iterator_x_registers_all_nil() {
    let (process, worker, mem) = setup();

    // All X registers are nil - none need tracing
    let mut found_any = false;
    iterate_roots_with_mem(&process, &worker, &mem, |loc, _term| {
        if matches!(loc, RootLocation::XRegister(_)) {
            found_any = true;
        }
    });

    // No X register roots found (nil doesn't need tracing)
    assert!(!found_any, "Expected no X register roots");
}

#[test]
fn root_iterator_x_registers_with_heap_pointers() {
    let (mut process, mut worker, mut mem) = setup();

    // Allocate a tuple on the heap
    let tuple_term = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(1).unwrap()])
        .expect("alloc failed");

    // Allocate a pair on the heap
    let pair_term = process
        .alloc_term_pair(&mut mem, Term::NIL, Term::NIL)
        .expect("alloc failed");

    // Put heap pointers in X registers
    worker.x_regs[0] = tuple_term;
    worker.x_regs[5] = pair_term;
    worker.x_regs[10] = Term::small_int(42).unwrap(); // Immediate, no tracing

    let mut x_roots = Vec::new();
    iterate_roots_with_mem(&process, &worker, &mem, |loc, term| {
        if let RootLocation::XRegister(i) = loc {
            x_roots.push((i, term));
        }
    });

    // Should find 2 roots (the tuple and pair)
    assert_eq!(
        x_roots.len(),
        2,
        "Expected 2 X register roots, got {x_roots:?}",
    );

    // Verify indices
    let indices: Vec<_> = x_roots.iter().map(|(i, _)| *i).collect();
    assert!(indices.contains(&0), "Missing X0");
    assert!(indices.contains(&5), "Missing X5");
}

// =============================================================================
// Process Bindings Root Tests
// =============================================================================

#[test]
fn root_iterator_process_bindings() {
    let (mut process, worker, mut mem) = setup();

    // Allocate a var on the heap
    let var_addr = process.alloc(32, 8).expect("alloc failed");

    // Allocate a value on the heap
    let value_term = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(42).unwrap()])
        .expect("alloc failed");

    // Insert binding
    process.bindings.insert(var_addr, value_term);

    let mut binding_roots = Vec::new();
    iterate_roots_with_mem(&process, &worker, &mem, |loc, term| {
        if let RootLocation::ProcessBinding(addr) = loc {
            binding_roots.push((addr, term));
        }
    });

    // Should find 1 root (the binding value)
    assert_eq!(
        binding_roots.len(),
        1,
        "Expected 1 binding root, got {binding_roots:?}",
    );
    assert_eq!(binding_roots[0].0, var_addr);
    assert_eq!(binding_roots[0].1, value_term);
}

// =============================================================================
// needs_tracing Tests
// =============================================================================

#[test]
fn needs_tracing_immediates() {
    assert!(!needs_tracing(Term::NIL));
    assert!(!needs_tracing(Term::TRUE));
    assert!(!needs_tracing(Term::FALSE));
    assert!(!needs_tracing(Term::small_int(42).unwrap()));
    assert!(!needs_tracing(Term::symbol(0)));
    assert!(!needs_tracing(Term::keyword(0)));
}

#[test]
fn needs_tracing_heap_pointers() {
    // These addresses are fake but have correct tags
    let list_ptr = 0x1000 as *const crate::term::pair::Pair;
    let list_term = Term::list(list_ptr);
    assert!(needs_tracing(list_term));

    let boxed_ptr = 0x2000 as *const crate::term::header::Header;
    let boxed_term = Term::boxed(boxed_ptr);
    assert!(needs_tracing(boxed_term));
}

// =============================================================================
// Root Update Tests
// =============================================================================

#[test]
fn root_update_x_register() {
    let (mut process, mut worker, _mem) = setup();

    // Set an X register
    let original = Term::small_int(42).unwrap();
    worker.x_regs[5] = original;

    // Update it via root updater
    let new_term = Term::small_int(100).unwrap();
    update_root(
        &mut process,
        &mut worker,
        &RootLocation::XRegister(5),
        new_term,
    );

    assert_eq!(worker.x_regs[5], new_term);
}

#[test]
fn root_update_process_binding() {
    let (mut process, mut worker, _mem) = setup();

    let var_addr = Vaddr::new(0x1000);
    let original = Term::small_int(42).unwrap();
    process.bindings.insert(var_addr, original);

    // Update via root updater
    let new_term = Term::small_int(100).unwrap();
    update_root(
        &mut process,
        &mut worker,
        &RootLocation::ProcessBinding(var_addr),
        new_term,
    );

    assert_eq!(*process.bindings.get(&var_addr).unwrap(), new_term);
}

// =============================================================================
// Basic RootIterator Tests (without Y registers)
// =============================================================================

#[test]
fn root_iterator_basic() {
    let (process, worker, _mem) = setup();

    // Basic iterator with no roots
    let roots: Vec<_> = RootIterator::new(&process, &worker).collect();

    // No roots found (all nil or empty)
    assert!(roots.is_empty(), "Expected no roots, got {roots:?}");
}

#[test]
fn root_iterator_with_x_and_binding() {
    let (mut process, mut worker, mut mem) = setup();

    // Put a heap pointer in X register
    let tuple_term = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(1).unwrap()])
        .expect("alloc failed");
    worker.x_regs[3] = tuple_term;

    // Add a binding with a heap pointer
    let var_addr = process.alloc(32, 8).expect("alloc failed");
    let value_term = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(2).unwrap()])
        .expect("alloc failed");
    process.bindings.insert(var_addr, value_term);

    // Collect roots (note: RootIterator doesn't scan Y regs, use iterate_roots_with_mem for that)
    let roots: Vec<_> = RootIterator::new(&process, &worker).collect();

    // Should find 2 roots (X register + binding)
    assert_eq!(roots.len(), 2, "Expected 2 roots, got {roots:?}");
}

// =============================================================================
// ChunkAddr Root Tests
// =============================================================================

#[test]
fn root_iterator_chunk_addr() {
    let (mut process, worker, mut mem) = setup();

    // Allocate a HeapFun on the young heap (simulates compiled function)
    let fn_term = process
        .alloc_term_compiled_fn(&mut mem, 0, false, 0, &[], &[])
        .expect("alloc failed");
    let fn_addr = fn_term.to_vaddr();

    // Set chunk_addr (this is what the VM does when executing a function)
    process.chunk_addr = Some(fn_addr);

    let mut chunk_roots = Vec::new();
    iterate_roots_with_mem(&process, &worker, &mem, |loc, term| {
        if matches!(loc, RootLocation::ChunkAddr) {
            chunk_roots.push(term);
        }
    });

    // Should find exactly 1 ChunkAddr root
    assert_eq!(
        chunk_roots.len(),
        1,
        "Expected 1 ChunkAddr root, got {chunk_roots:?}",
    );
    assert_eq!(chunk_roots[0].to_vaddr(), fn_addr);
}

#[test]
fn root_iterator_chunk_addr_none() {
    let (process, worker, mem) = setup();

    // No chunk_addr set — should yield no ChunkAddr roots
    assert!(process.chunk_addr.is_none());

    let mut found = false;
    iterate_roots_with_mem(&process, &worker, &mem, |loc, _term| {
        if matches!(loc, RootLocation::ChunkAddr) {
            found = true;
        }
    });

    assert!(
        !found,
        "Should not find ChunkAddr root when chunk_addr is None"
    );
}

#[test]
fn root_update_chunk_addr() {
    let (mut process, mut worker, mut mem) = setup();

    // Allocate a HeapFun on the young heap
    let fn_term = process
        .alloc_term_compiled_fn(&mut mem, 0, false, 0, &[], &[])
        .expect("alloc failed");
    process.chunk_addr = Some(fn_term.to_vaddr());

    // Simulate GC updating chunk_addr to a new address
    let new_addr = Vaddr::new(0x0010_1000); // In old heap region
    let new_term = Term::boxed_vaddr(new_addr);
    update_root(
        &mut process,
        &mut worker,
        &RootLocation::ChunkAddr,
        new_term,
    );

    assert_eq!(process.chunk_addr, Some(new_addr));
}

// =============================================================================
// FrameChunkAddr Root Tests
// =============================================================================

#[test]
fn root_iterator_frame_chunk_addr() {
    let (mut process, worker, mut mem) = setup();

    // Allocate a HeapFun on the young heap (simulates caller's function)
    let fn_term = process
        .alloc_term_compiled_fn(&mut mem, 0, false, 0, &[], &[])
        .expect("alloc failed");
    let fn_addr = fn_term.to_vaddr();

    // Manually set up a frame with chunk_addr pointing to HeapFun
    let y_count: u64 = 0;
    let frame_header_size = 32u64;

    let frame_base = Vaddr::new(process.hend.as_u64() - frame_header_size);

    mem.write(Vaddr::new(frame_base.as_u64()), 0u64); // return_ip
    mem.write(Vaddr::new(frame_base.as_u64() + 8), fn_addr.as_u64()); // chunk_addr = HeapFun
    mem.write(Vaddr::new(frame_base.as_u64() + 16), 0u64); // caller_frame_base (top level)
    mem.write(Vaddr::new(frame_base.as_u64() + 24), y_count); // y_count

    process.stop = frame_base;
    process.frame_base = Some(frame_base);
    process.current_y_count = 0;

    let mut frame_chunk_roots = Vec::new();
    iterate_roots_with_mem(&process, &worker, &mem, |loc, term| {
        if let RootLocation::FrameChunkAddr { frame_addr } = loc {
            frame_chunk_roots.push((frame_addr, term));
        }
    });

    // Should find exactly 1 FrameChunkAddr root
    assert_eq!(
        frame_chunk_roots.len(),
        1,
        "Expected 1 FrameChunkAddr root, got {frame_chunk_roots:?}",
    );
    assert_eq!(frame_chunk_roots[0].0, frame_base);
    assert_eq!(frame_chunk_roots[0].1.to_vaddr(), fn_addr);
}

#[test]
fn root_iterator_frame_chunk_addr_null() {
    let (mut process, worker, mut mem) = setup();

    // Frame with chunk_addr = 0 (null, e.g. REPL top level)
    let y_count: u64 = 0;
    let frame_header_size = 32u64;

    let frame_base = Vaddr::new(process.hend.as_u64() - frame_header_size);

    mem.write(Vaddr::new(frame_base.as_u64()), 0u64); // return_ip
    mem.write(Vaddr::new(frame_base.as_u64() + 8), 0u64); // chunk_addr = null
    mem.write(Vaddr::new(frame_base.as_u64() + 16), 0u64); // caller_frame_base
    mem.write(Vaddr::new(frame_base.as_u64() + 24), y_count); // y_count

    process.stop = frame_base;
    process.frame_base = Some(frame_base);
    process.current_y_count = 0;

    let mut found = false;
    iterate_roots_with_mem(&process, &worker, &mem, |loc, _term| {
        if matches!(loc, RootLocation::FrameChunkAddr { .. }) {
            found = true;
        }
    });

    assert!(
        !found,
        "Should not find FrameChunkAddr root when chunk_addr is null"
    );
}

// =============================================================================
// Y Register Tests (using iterate_roots_with_mem with manual frame setup)
// =============================================================================

#[test]
fn iterate_roots_with_frame_manually_created() {
    let (mut process, worker, mut mem) = setup();

    // Manually set up a frame structure in memory
    // This simulates what allocate_frame would do

    // Frame structure (from top of heap, growing down):
    // - Frame header at frame_base:
    //   - return_ip (8 bytes)
    //   - chunk_addr (8 bytes)
    //   - caller_frame_base (8 bytes, 0 for top level)
    //   - y_count (8 bytes)
    // - Y registers below frame_base (y_count * 8 bytes)

    let y_count: u64 = 2;
    let frame_header_size = 32u64; // 4 * 8 bytes
    let y_space = y_count * 8;

    // Calculate addresses (stack grows down from hend)
    let frame_base = Vaddr::new(process.hend.as_u64() - frame_header_size);
    let y_base = Vaddr::new(frame_base.as_u64() - y_space);

    // Write frame header
    mem.write(Vaddr::new(frame_base.as_u64()), 0u64); // return_ip
    mem.write(Vaddr::new(frame_base.as_u64() + 8), 0u64); // chunk_addr
    mem.write(Vaddr::new(frame_base.as_u64() + 16), 0u64); // caller_frame_base (top level)
    mem.write(Vaddr::new(frame_base.as_u64() + 24), y_count); // y_count

    // Allocate a heap term and write to Y[0]
    let heap_term = process
        .alloc_term_tuple(&mut mem, &[Term::small_int(42).unwrap()])
        .expect("alloc failed");

    // Write Y registers
    mem.write(y_base, heap_term); // Y[0] = heap pointer
    mem.write(Vaddr::new(y_base.as_u64() + 8), Term::NIL); // Y[1] = nil

    // Update process state
    process.stop = y_base;
    process.frame_base = Some(frame_base);
    process.current_y_count = y_count as usize;

    // Now iterate roots
    let mut y_roots = Vec::new();
    iterate_roots_with_mem(&process, &worker, &mem, |loc, term| {
        if let RootLocation::YRegister {
            frame_addr,
            y_index,
        } = loc
        {
            y_roots.push((frame_addr, y_index, term));
        }
    });

    // Should find 1 Y register root (Y[0] has heap pointer, Y[1] is nil)
    assert_eq!(
        y_roots.len(),
        1,
        "Expected 1 Y register root, got {y_roots:?}",
    );
    assert_eq!(y_roots[0].0, frame_base);
    assert_eq!(y_roots[0].1, 0);
    assert_eq!(y_roots[0].2, heap_term);
}

#[test]
fn update_root_y_register() {
    let (process, _worker, mut mem) = setup();

    // Set up frame like above
    let y_count: u64 = 2;
    let frame_header_size = 32u64;
    let y_space = y_count * 8;

    let frame_base = Vaddr::new(process.hend.as_u64() - frame_header_size);
    let y_base = Vaddr::new(frame_base.as_u64() - y_space);

    mem.write(Vaddr::new(frame_base.as_u64() + 24), y_count);

    let original = Term::small_int(42).unwrap();
    mem.write(Vaddr::new(y_base.as_u64() + 8), original); // Y[1]

    // Update Y[1] via root updater
    let new_term = Term::small_int(100).unwrap();
    update_root_y(&process, &mut mem, frame_base, 1, new_term);

    // Verify update
    let read_back: Term = mem.read(Vaddr::new(y_base.as_u64() + 8));
    assert_eq!(read_back, new_term);
}
