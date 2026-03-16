// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for VM yielding and resumption.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{eval, setup};
use crate::Vaddr;
use crate::bytecode::{Chunk, encode_abc, encode_abx, op};
use crate::platform::MockVSpace;
use crate::process::{MAX_REDUCTIONS, Process, WorkerId};
use crate::realm::{Realm, bootstrap};
use crate::scheduler::Worker;
use crate::term::Term;
use crate::vm::{RunResult, Vm};

/// Helper to create a small integer Term.
fn int(n: i64) -> Term {
    Term::small_int(n).expect("integer out of small_int range")
}

// --- Helper functions ---

/// Create a test environment (worker, process, realm, memory).
fn create_test_env() -> (Worker, Process, Realm, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(512 * 1024, base);
    let mut realm = Realm::new_for_test(base).unwrap();

    let (young_base, old_base) = realm
        .allocate_process_memory(128 * 1024, 32 * 1024)
        .unwrap();
    let mut proc = Process::new(young_base, 128 * 1024, old_base, 32 * 1024);

    let result = bootstrap(&mut realm, &mut mem).unwrap();
    proc.bootstrap(result.ns_var, result.core_ns);

    let worker = Worker::new(WorkerId(0));
    (worker, proc, realm, mem)
}

/// Create a simple chunk with LOADINT instructions followed by HALT.
fn create_loadint_chunk(count: usize) -> Chunk {
    let mut chunk = Chunk::new();
    for i in 0..count {
        chunk.emit(encode_abx(op::LOADINT, i as u8, i as u32));
    }
    chunk.emit(encode_abc(op::HALT, 0, 0, 0));
    chunk
}

// --- Yield tests ---

#[test]
fn vm_yields_when_budget_exhausted() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // 10 LOADINT instructions + HALT
    let chunk = create_loadint_chunk(10);
    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );
    proc.reductions = 5; // Budget for only 5 instructions

    let result = Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None);
    assert!(matches!(result, RunResult::Yielded));
    assert_eq!(proc.ip, 5); // Should have executed 5 instructions
}

#[test]
fn vm_completes_with_sufficient_budget() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let chunk = create_loadint_chunk(5);
    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );
    proc.reductions = 100;

    let result = Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None);
    assert!(matches!(result, RunResult::Completed(_)));
}

#[test]
fn vm_resumes_correctly() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // 6 LOADINT + HALT
    // Note: HALT returns X0, so we put the final result in X0
    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // X1 = 1
    chunk.emit(encode_abx(op::LOADINT, 2, 2)); // X2 = 2
    chunk.emit(encode_abx(op::LOADINT, 3, 3)); // X3 = 3
    chunk.emit(encode_abx(op::LOADINT, 4, 4)); // X4 = 4
    chunk.emit(encode_abx(op::LOADINT, 5, 5)); // X5 = 5
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // X0 = 42 (return value)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // Return X0

    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );
    proc.reductions = 3; // First run: execute 3 instructions

    // First run - should yield after 3 instructions
    let result1 = Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None);
    assert!(matches!(result1, RunResult::Yielded));
    assert_eq!(worker.x_regs[1], int(1));
    assert_eq!(worker.x_regs[2], int(2));
    assert_eq!(worker.x_regs[3], int(3));
    assert_eq!(worker.x_regs[4], Term::NIL); // Not yet executed

    // Resume with more budget
    proc.reductions = 100;
    let result2 = Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None);
    assert!(matches!(result2, RunResult::Completed(v) if v == int(42)));
}

#[test]
fn reductions_are_consumed() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let chunk = create_loadint_chunk(3);
    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );
    proc.reductions = 100;
    proc.total_reductions = 0;

    let _ = Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None);

    // 3 LOADINT (cost 1 each) + 1 HALT (cost 0, returns before consuming)
    // So 3 reductions should be consumed
    assert_eq!(proc.total_reductions, 3);
    assert_eq!(proc.reductions, 97);
}

// --- Stack frame tests ---

#[test]
fn stack_frame_allocate_deallocate() {
    let (mut _worker, mut proc, _, mut mem) = create_test_env();

    // Set up initial chunk and allocate it on heap
    let chunk = create_loadint_chunk(1);
    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );
    proc.ip = 42;
    let original_chunk_addr = proc.chunk_addr.unwrap();

    // Allocate frame
    let result = proc.allocate_frame(&mut mem, proc.ip, original_chunk_addr);
    assert!(result.is_ok());
    assert_eq!(proc.call_depth(), 1);

    // Set callee state
    let callee_chunk = create_loadint_chunk(1);
    assert!(
        proc.write_chunk_to_heap(&mut mem, &callee_chunk),
        "out of memory writing callee chunk to heap"
    );

    // Deallocate frame
    let result = proc.deallocate_frame(&mem);
    assert!(result.is_some());
    let (return_ip, chunk_addr) = result.unwrap();
    assert_eq!(return_ip, 42);
    assert_eq!(chunk_addr, original_chunk_addr);
    assert_eq!(proc.call_depth(), 0);
}

#[test]
fn stack_frame_overflow_detection() {
    let (mut _worker, mut proc, _, mut mem) = create_test_env();

    // Allocate frames until stack overflow
    let mut frame_count = 0;
    loop {
        let result = proc.allocate_frame(&mut mem, 0, Vaddr::new(0));
        if result.is_err() {
            break;
        }
        frame_count += 1;
        // Safety: don't loop forever
        assert!(frame_count <= 10000, "Stack overflow not detected");
    }

    // We should have allocated some frames before overflow
    assert!(frame_count > 0, "Should allocate at least one frame");
}

#[test]
fn deallocate_at_top_level_returns_none() {
    let (mut _worker, mut proc, _, mem) = create_test_env();
    assert!(proc.at_top_level());
    assert!(proc.deallocate_frame(&mem).is_none());
}

// --- Integration tests for yield during function calls ---

#[test]
fn yield_during_simple_function_call() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let mut worker = Worker::new(WorkerId(0));

    // Compile a function call that will yield
    let result = crate::reader::read("((fn* [x] (+ x 1)) 5)", &mut proc, &mut realm, &mut mem)
        .ok()
        .flatten()
        .unwrap();
    let chunk = crate::compiler::compile(result, &mut proc, &mut mem, &mut realm).unwrap();
    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );

    // Very small budget to force yield inside function
    proc.reductions = 2;

    let result1 = Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None);
    // May yield or complete depending on exact instruction count
    match result1 {
        RunResult::Yielded => {
            // Resume and complete
            proc.reductions = 100;
            let result2 = Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None);
            assert!(matches!(result2, RunResult::Completed(v) if v == int(6)));
        }
        RunResult::Completed(v) => {
            assert_eq!(v, int(6));
        }
        RunResult::Waiting => panic!("Unexpected Waiting"),
        RunResult::Exited(r) => panic!("Unexpected exit: {r:?}"),
        RunResult::Error(e) => panic!("Unexpected error: {e:?}"),
    }
}

#[test]
fn yield_and_resume_nested_calls() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let mut worker = Worker::new(WorkerId(0));

    // Test yield and resume with nested function calls
    // Define several functions and call them in a chain
    // This tests that the call stack is preserved across yields

    // Define add1 function
    eval(
        "(def add1 (fn* [x] (+ x 1)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    // Define add2 that calls add1 twice
    eval(
        "(def add2 (fn* [x] (add1 (add1 x))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    // Define add4 that calls add2 twice
    eval(
        "(def add4 (fn* [x] (add2 (add2 x))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    // Reset the process state after eval (which resets it too)
    proc.reset();

    // Compile (add4 10) - should return 14
    let expr = crate::reader::read("(add4 10)", &mut proc, &mut realm, &mut mem)
        .ok()
        .flatten()
        .unwrap();
    let chunk = crate::compiler::compile(expr, &mut proc, &mut mem, &mut realm).unwrap();
    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );

    // Small budget to force multiple yields
    proc.reductions = 3;
    let mut yield_count = 0;

    loop {
        match Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None) {
            RunResult::Completed(v) => {
                assert_eq!(v, int(14));
                break;
            }
            RunResult::Yielded => {
                yield_count += 1;
                proc.reset_reductions();
                // Prevent infinite loop
                assert!(yield_count <= 100, "Too many yields");
            }
            RunResult::Waiting => panic!("Unexpected Waiting"),
            RunResult::Exited(r) => panic!("Unexpected exit: {r:?}"),
            RunResult::Error(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    // Should have yielded at least once with such a small budget
    assert!(yield_count > 0, "Expected at least one yield");
}

#[test]
fn run_result_methods() {
    assert!(RunResult::Completed(Term::NIL).is_terminal());
    assert!(RunResult::Error(crate::vm::RuntimeError::NoCode).is_terminal());
    assert!(RunResult::Exited(Term::NIL).is_terminal());
    assert!(!RunResult::Yielded.is_terminal());

    assert!(RunResult::Yielded.is_yielded());
    assert!(!RunResult::Completed(Term::NIL).is_yielded());
    assert!(!RunResult::Error(crate::vm::RuntimeError::NoCode).is_yielded());
    assert!(!RunResult::Exited(Term::NIL).is_yielded());

    assert!(!RunResult::Exited(Term::NIL).is_waiting());
}

#[test]
fn execute_handles_yielding_transparently() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Note: eval() creates its own Worker internally

    // The execute() function should handle yielding internally
    // and always run to completion
    // Note: + is binary, so we nest the additions
    let result = eval("(+ (+ (+ 1 2) 3) 4)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Ok(int(10)));
}

#[test]
fn reset_reductions_sets_max() {
    let (mut _worker, mut proc, _, _) = create_test_env();
    proc.reductions = 0;
    proc.reset_reductions();
    assert_eq!(proc.reductions, MAX_REDUCTIONS);
}

// --- Stress tests ---

/// Number of iterations for `stress_many_yields` test.
const STRESS_ITERATION_COUNT: usize = 1000;

/// Reduction budget per time slice for `stress_many_yields` test.
const STRESS_BUDGET_PER_SLICE: u32 = 100;

/// Stress test: many iterations with small budget causes many yields.
///
/// This test creates bytecode that executes many instructions (simulating a long loop),
/// runs it with a small reduction budget, and verifies that:
/// 1. The VM yields many times
/// 2. The VM eventually completes with the correct result
#[test]
fn stress_many_yields() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // Create a chunk with 1000 LOADINT instructions to simulate a long computation.
    // Each LOADINT costs 1 reduction, so with budget=100 we should yield ~10 times.
    let mut chunk = Chunk::new();
    for i in 0..STRESS_ITERATION_COUNT {
        // Use register 0 so the final value is 999 (the return value)
        chunk.emit(encode_abx(op::LOADINT, 0, i as u32));
    }
    chunk.emit(encode_abc(op::HALT, 0, 0, 0));

    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );

    // Small budget to force many yields - reset to this value each time
    proc.reductions = STRESS_BUDGET_PER_SLICE;
    let mut yield_count = 0;

    loop {
        match Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None) {
            RunResult::Completed(v) => {
                // Final value should be the last iteration (999)
                assert_eq!(v, int(999));
                break;
            }
            RunResult::Yielded => {
                yield_count += 1;
                // Reset to the same small budget, not MAX_REDUCTIONS
                proc.reductions = STRESS_BUDGET_PER_SLICE;
                // Safety: prevent infinite loop in case of bug
                assert!(
                    yield_count <= 100,
                    "Too many yields - possible infinite loop"
                );
            }
            RunResult::Waiting => panic!("Unexpected Waiting"),
            RunResult::Exited(r) => panic!("Unexpected exit: {r:?}"),
            RunResult::Error(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    // With 1000 instructions and budget of 100, we should yield at least 9 times
    // (1000 / 100 = 10 slices, minus 1 for completion = 9 yields minimum)
    assert!(
        yield_count >= 9,
        "Expected at least 9 yields, got {yield_count}"
    );
}

/// Stress test: many nested calls with small budget yields and resumes correctly.
///
/// This test creates a deep chain of function calls (simulating recursion depth)
/// and verifies the VM can yield and resume with preserved call stacks.
#[test]
#[allow(clippy::too_many_lines)]
fn stress_recursive_with_yields() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let mut worker = Worker::new(WorkerId(0));

    // Create a chain of 10 add functions to simulate deep nesting.
    // Each function adds its number and calls the next one.
    eval(
        "(def a10 (fn* [x] (+ x 10)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def a9 (fn* [x] (a10 (+ x 9))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def a8 (fn* [x] (a9 (+ x 8))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def a7 (fn* [x] (a8 (+ x 7))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def a6 (fn* [x] (a7 (+ x 6))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def a5 (fn* [x] (a6 (+ x 5))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def a4 (fn* [x] (a5 (+ x 4))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def a3 (fn* [x] (a4 (+ x 3))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def a2 (fn* [x] (a3 (+ x 2))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def a1 (fn* [x] (a2 (+ x 1))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    proc.reset();

    // Compile (a1 0) - result should be 0 + 1 + 2 + ... + 10 = 55
    let expr = crate::reader::read("(a1 0)", &mut proc, &mut realm, &mut mem)
        .ok()
        .flatten()
        .unwrap();
    let chunk = crate::compiler::compile(expr, &mut proc, &mut mem, &mut realm).unwrap();
    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );

    // Very small budget to force yields during the deep call chain
    proc.reductions = 3;
    let mut yield_count = 0;

    loop {
        match Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None) {
            RunResult::Completed(v) => {
                // sum(1..10) = 55
                assert_eq!(v, int(55));
                break;
            }
            RunResult::Yielded => {
                yield_count += 1;
                proc.reductions = 3; // Keep small budget
                // Safety: prevent infinite loop
                assert!(
                    yield_count <= 200,
                    "Too many yields - possible infinite loop"
                );
            }
            RunResult::Waiting => panic!("Unexpected Waiting"),
            RunResult::Exited(r) => panic!("Unexpected exit: {r:?}"),
            RunResult::Error(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    // With 10 nested calls and budget of 3, we should yield multiple times
    assert!(
        yield_count > 0,
        "Expected at least one yield during deep calls"
    );
}

/// Stress test: deep call chain yields at various depths and resumes correctly.
///
/// This test creates a chain of function calls and verifies that yielding
/// preserves the call stack correctly by completing with the right result.
#[test]
fn stress_deep_call_chain_yield_resume() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let mut worker = Worker::new(WorkerId(0));

    // Define a chain of 5 functions that each call the next
    eval(
        "(def f5 (fn* [x] (+ x 5)))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def f4 (fn* [x] (f5 (+ x 4))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def f3 (fn* [x] (f4 (+ x 3))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def f2 (fn* [x] (f3 (+ x 2))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();
    eval(
        "(def f1 (fn* [x] (f2 (+ x 1))))",
        &mut proc,
        &mut realm,
        &mut mem,
    )
    .unwrap();

    proc.reset();

    // Compile (f1 0) - result should be 0 + 1 + 2 + 3 + 4 + 5 = 15
    let expr = crate::reader::read("(f1 0)", &mut proc, &mut realm, &mut mem)
        .ok()
        .flatten()
        .unwrap();
    let chunk = crate::compiler::compile(expr, &mut proc, &mut mem, &mut realm).unwrap();
    assert!(
        proc.write_chunk_to_heap(&mut mem, &chunk),
        "out of memory writing chunk to heap"
    );

    // Very small budget to force yields during nested calls
    proc.reductions = 2;
    let mut yield_count = 0;

    loop {
        match Vm::run(&mut worker, &mut proc, &mut mem, &mut realm, None) {
            RunResult::Completed(v) => {
                assert_eq!(v, int(15));
                break;
            }
            RunResult::Yielded => {
                yield_count += 1;
                proc.reductions = 2; // Keep small budget
                assert!(yield_count <= 100, "Too many yields");
            }
            RunResult::Waiting => panic!("Unexpected Waiting"),
            RunResult::Exited(r) => panic!("Unexpected exit: {r:?}"),
            RunResult::Error(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    // Should have yielded multiple times with such a small budget
    assert!(yield_count > 0, "Expected at least one yield");
    // Call stack should be empty after completion
    assert!(proc.at_top_level(), "Call stack should be empty at end");
}
