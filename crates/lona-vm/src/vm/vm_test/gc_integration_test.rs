// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for GC integration with the VM execution loop.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{eval, setup};
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::realm::{Realm, bootstrap};
use crate::term::Term;

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

/// Setup with smaller heaps to leave pool room for major GC heap reallocation.
fn setup_for_major_gc() -> Option<(Process, Realm, MockVSpace)> {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(512 * 1024, base);
    let mut realm = Realm::new_for_test(base)?;

    // 32KB young + 8KB old = 40KB from 192KB pool → 152KB free for major GC
    let (young_base, old_base) = realm.allocate_process_memory(32 * 1024, 8 * 1024)?;
    let mut proc = Process::new(young_base, 32 * 1024, old_base, 8 * 1024);

    let result = bootstrap(&mut realm, &mut mem)?;
    proc.bootstrap(result.ns_var, result.core_ns);

    Some((proc, realm, mem))
}

// --- GC retry on OOM ---

#[test]
fn gc_triggered_on_allocation_pressure() {
    // Allocate enough data to trigger GC, verify execution continues
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Create a string (heap allocation) that should succeed even if heap is pressured
    let result = eval(
        "(str \"hello\" \" \" \"world\")",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    let s = proc.read_term_string(&mem, result).unwrap();
    assert_eq!(s, "hello world");
}

#[test]
fn gc_allows_continued_allocation_after_collection() {
    // Repeatedly allocate to force GC cycles, verify all results are correct
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define a function that creates allocations (tuples, strings)
    eval(
        "(def make-pair (fn* [a b] [a b]))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    // Call it many times - this should trigger GC at some point
    for i in 0..20 {
        let expr = format!("(make-pair {} {})", i, i + 1);
        let result = eval(&expr, &mut proc, &mut realm, &mut mem).unwrap();
        // Verify result is a tuple
        assert!(
            proc.is_term_tuple(&mem, result),
            "iteration {i}: expected tuple"
        );
    }
}

#[test]
fn gc_preserves_values_across_collection() {
    // Verify that values remain correct after GC has run
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Define vars before GC pressure
    eval("(def x 42)", &mut proc, &mut realm, &mut mem).unwrap();
    eval("(def y 100)", &mut proc, &mut realm, &mut mem).unwrap();

    // Create allocation pressure with many tuples
    for i in 0..20 {
        let expr = format!("[{} {} {}]", i, i + 1, i + 2);
        let _ = eval(&expr, &mut proc, &mut realm, &mut mem);
    }

    // Verify defined values are still correct
    let result = eval("(+ x y)", &mut proc, &mut realm, &mut mem).unwrap();
    assert_eq!(result, int(142));
}

// --- garbage-collect intrinsic ---

#[test]
fn garbage_collect_intrinsic_returns_ok() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    let result = eval("(garbage-collect)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_keyword(), "expected keyword :ok");
}

#[test]
fn garbage_collect_full_returns_ok() {
    let (mut proc, mut realm, mut mem) = setup_for_major_gc().unwrap();

    let result = eval("(garbage-collect :full)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(result.is_keyword(), "expected keyword :ok");
}

#[test]
fn garbage_collect_increments_minor_gc_count() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    let count_before = proc.minor_gc_count;
    eval("(garbage-collect)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(
        proc.minor_gc_count > count_before,
        "minor GC count should increase"
    );
}

#[test]
fn garbage_collect_full_increments_major_gc_count() {
    let (mut proc, mut realm, mut mem) = setup_for_major_gc().unwrap();

    let count_before = proc.major_gc_count;
    eval("(garbage-collect :full)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(
        proc.major_gc_count > count_before,
        "major GC count should increase"
    );
}

// --- process-info intrinsic ---

#[test]
fn process_info_returns_map() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    let result = eval("(process-info)", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(proc.is_term_map(&mem, result), "expected map");
}

#[test]
fn process_info_has_heap_size() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    let result = eval(
        "(get (process-info) :heap-size)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert!(
        result.is_small_int(),
        "expected integer for :heap-size, got {result:?}",
    );
    let heap_size = result.as_small_int().unwrap();
    assert!(heap_size > 0, "heap-size should be positive");
}

#[test]
fn process_info_has_gc_stats() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Run a GC first to get non-zero stats
    eval("(garbage-collect)", &mut proc, &mut realm, &mut mem).unwrap();

    let result = eval(
        "(get (process-info) :minor-gc-count)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert!(
        result.is_small_int(),
        "expected integer for :minor-gc-count"
    );
    let count = result.as_small_int().unwrap();
    assert!(count > 0, "minor-gc-count should be > 0 after GC");
}

#[test]
fn process_info_has_reductions() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    let result = eval(
        "(get (process-info) :reductions)",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    assert!(result.is_small_int(), "expected integer for :reductions");
    let reductions = result.as_small_int().unwrap();
    assert!(reductions >= 0, "reductions should be non-negative");
}
