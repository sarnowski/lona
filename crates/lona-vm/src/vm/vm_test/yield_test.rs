// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for VM yielding and resumption.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{eval, setup};
use crate::Vaddr;
use crate::bytecode::{Chunk, encode_abc, encode_abx, op};
use crate::platform::MockVSpace;
use crate::process::{MAX_CALL_DEPTH, MAX_REDUCTIONS, Process};
use crate::realm::{Realm, bootstrap};
use crate::value::Value;
use crate::vm::{RunResult, Vm};

// --- Helper functions ---

/// Create a test environment (process, realm, memory).
fn create_test_env() -> (Process, Realm, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let mut proc = Process::new(1, young_base, young_size, old_base, old_size);

    let realm_base = base.add(128 * 1024);
    let mut realm = Realm::new(realm_base, 64 * 1024);

    let result = bootstrap(&mut realm, &mut mem).unwrap();
    proc.bootstrap(result.ns_var, result.core_ns);

    (proc, realm, mem)
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
    let (mut proc, mut realm, mut mem) = create_test_env();

    // 10 LOADINT instructions + HALT
    let chunk = create_loadint_chunk(10);
    proc.set_chunk(chunk);
    proc.reductions = 5; // Budget for only 5 instructions

    let result = Vm::run(&mut proc, &mut mem, &mut realm);
    assert!(matches!(result, RunResult::Yielded));
    assert_eq!(proc.ip, 5); // Should have executed 5 instructions
}

#[test]
fn vm_completes_with_sufficient_budget() {
    let (mut proc, mut realm, mut mem) = create_test_env();

    let chunk = create_loadint_chunk(5);
    proc.set_chunk(chunk);
    proc.reductions = 100;

    let result = Vm::run(&mut proc, &mut mem, &mut realm);
    assert!(matches!(result, RunResult::Completed(_)));
}

#[test]
fn vm_resumes_correctly() {
    let (mut proc, mut realm, mut mem) = create_test_env();

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

    proc.set_chunk(chunk);
    proc.reductions = 3; // First run: execute 3 instructions

    // First run - should yield after 3 instructions
    let result1 = Vm::run(&mut proc, &mut mem, &mut realm);
    assert!(matches!(result1, RunResult::Yielded));
    assert_eq!(proc.x_regs[1], Value::int(1));
    assert_eq!(proc.x_regs[2], Value::int(2));
    assert_eq!(proc.x_regs[3], Value::int(3));
    assert_eq!(proc.x_regs[4], Value::Nil); // Not yet executed

    // Resume with more budget
    proc.reductions = 100;
    let result2 = Vm::run(&mut proc, &mut mem, &mut realm);
    assert!(matches!(result2, RunResult::Completed(v) if v == Value::int(42)));
}

#[test]
fn reductions_are_consumed() {
    let (mut proc, mut realm, mut mem) = create_test_env();

    let chunk = create_loadint_chunk(3);
    proc.set_chunk(chunk);
    proc.reductions = 100;
    proc.total_reductions = 0;

    let _ = Vm::run(&mut proc, &mut mem, &mut realm);

    // 3 LOADINT (cost 1 each) + 1 HALT (cost 0, returns before consuming)
    // So 3 reductions should be consumed
    assert_eq!(proc.total_reductions, 3);
    assert_eq!(proc.reductions, 97);
}

// --- Call stack tests ---

#[test]
fn call_stack_push_pop() {
    let (mut proc, _, _) = create_test_env();

    // Set up initial chunk
    let chunk = create_loadint_chunk(1);
    proc.set_chunk(chunk);
    proc.ip = 42;

    // Push frame
    let result = proc.push_call_frame(Vaddr::new(0x1000));
    assert!(result.is_ok());
    assert_eq!(proc.call_depth(), 1);
    assert!(proc.chunk.is_none()); // Chunk moved to stack

    // Set callee state
    proc.chunk = Some(create_loadint_chunk(1));
    proc.ip = 0;

    // Pop frame
    assert!(proc.pop_call_frame());
    assert_eq!(proc.call_depth(), 0);
    assert_eq!(proc.ip, 42); // Restored
    assert!(proc.chunk.is_some()); // Chunk restored
}

#[test]
fn call_stack_overflow_detection() {
    let (mut proc, _, _) = create_test_env();

    // Fill call stack to max
    for _ in 0..MAX_CALL_DEPTH {
        proc.chunk = Some(create_loadint_chunk(1));
        assert!(proc.push_call_frame(Vaddr::new(0x1000)).is_ok());
    }

    // Next push should fail
    proc.chunk = Some(create_loadint_chunk(1));
    assert!(proc.push_call_frame(Vaddr::new(0x1000)).is_err());
}

#[test]
fn pop_at_top_level_returns_false() {
    let (mut proc, _, _) = create_test_env();
    assert!(proc.at_top_level());
    assert!(!proc.pop_call_frame());
}

// --- Integration tests for yield during function calls ---

#[test]
fn yield_during_simple_function_call() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // Compile a function call that will yield
    let result = crate::reader::read("((fn* [x] (+ x 1)) 5)", &mut proc, &mut mem)
        .ok()
        .flatten()
        .unwrap();
    let chunk = crate::compiler::compile(result, &mut proc, &mut mem, &mut realm).unwrap();
    proc.set_chunk(chunk);

    // Very small budget to force yield inside function
    proc.reductions = 2;

    let result1 = Vm::run(&mut proc, &mut mem, &mut realm);
    // May yield or complete depending on exact instruction count
    match result1 {
        RunResult::Yielded => {
            // Resume and complete
            proc.reductions = 100;
            let result2 = Vm::run(&mut proc, &mut mem, &mut realm);
            assert!(matches!(result2, RunResult::Completed(v) if v == Value::int(6)));
        }
        RunResult::Completed(v) => {
            assert_eq!(v, Value::int(6));
        }
        RunResult::Error(e) => panic!("Unexpected error: {e:?}"),
    }
}

#[test]
fn yield_and_resume_nested_calls() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

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
    let expr = crate::reader::read("(add4 10)", &mut proc, &mut mem)
        .ok()
        .flatten()
        .unwrap();
    let chunk = crate::compiler::compile(expr, &mut proc, &mut mem, &mut realm).unwrap();
    proc.set_chunk(chunk);

    // Small budget to force multiple yields
    proc.reductions = 3;
    let mut yield_count = 0;

    loop {
        match Vm::run(&mut proc, &mut mem, &mut realm) {
            RunResult::Completed(v) => {
                assert_eq!(v, Value::int(14));
                break;
            }
            RunResult::Yielded => {
                yield_count += 1;
                proc.reset_reductions();
                // Prevent infinite loop
                assert!(yield_count <= 100, "Too many yields");
            }
            RunResult::Error(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    // Should have yielded at least once with such a small budget
    assert!(yield_count > 0, "Expected at least one yield");
}

#[test]
fn run_result_methods() {
    assert!(RunResult::Completed(Value::Nil).is_terminal());
    assert!(RunResult::Error(crate::vm::RuntimeError::NoCode).is_terminal());
    assert!(!RunResult::Yielded.is_terminal());

    assert!(RunResult::Yielded.is_yielded());
    assert!(!RunResult::Completed(Value::Nil).is_yielded());
    assert!(!RunResult::Error(crate::vm::RuntimeError::NoCode).is_yielded());
}

#[test]
fn execute_handles_yielding_transparently() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();

    // The execute() function should handle yielding internally
    // and always run to completion
    // Note: + is binary, so we nest the additions
    let result = eval("(+ (+ (+ 1 2) 3) 4)", &mut proc, &mut realm, &mut mem);
    assert_eq!(result, Ok(Value::int(10)));
}

#[test]
fn reset_reductions_sets_max() {
    let (mut proc, _, _) = create_test_env();
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
    let (mut proc, mut realm, mut mem) = create_test_env();

    // Create a chunk with 1000 LOADINT instructions to simulate a long computation.
    // Each LOADINT costs 1 reduction, so with budget=100 we should yield ~10 times.
    let mut chunk = Chunk::new();
    for i in 0..STRESS_ITERATION_COUNT {
        // Use register 0 so the final value is 999 (the return value)
        chunk.emit(encode_abx(op::LOADINT, 0, i as u32));
    }
    chunk.emit(encode_abc(op::HALT, 0, 0, 0));

    proc.set_chunk(chunk);

    // Small budget to force many yields - reset to this value each time
    proc.reductions = STRESS_BUDGET_PER_SLICE;
    let mut yield_count = 0;

    loop {
        match Vm::run(&mut proc, &mut mem, &mut realm) {
            RunResult::Completed(v) => {
                // Final value should be the last iteration (999)
                assert_eq!(v, Value::int(999));
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
    let expr = crate::reader::read("(a1 0)", &mut proc, &mut mem)
        .ok()
        .flatten()
        .unwrap();
    let chunk = crate::compiler::compile(expr, &mut proc, &mut mem, &mut realm).unwrap();
    proc.set_chunk(chunk);

    // Very small budget to force yields during the deep call chain
    proc.reductions = 3;
    let mut yield_count = 0;

    loop {
        match Vm::run(&mut proc, &mut mem, &mut realm) {
            RunResult::Completed(v) => {
                // sum(1..10) = 55
                assert_eq!(v, Value::int(55));
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
    let expr = crate::reader::read("(f1 0)", &mut proc, &mut mem)
        .ok()
        .flatten()
        .unwrap();
    let chunk = crate::compiler::compile(expr, &mut proc, &mut mem, &mut realm).unwrap();
    proc.set_chunk(chunk);

    // Very small budget to force yields during nested calls
    proc.reductions = 2;
    let mut yield_count = 0;

    loop {
        match Vm::run(&mut proc, &mut mem, &mut realm) {
            RunResult::Completed(v) => {
                assert_eq!(v, Value::int(15));
                break;
            }
            RunResult::Yielded => {
                yield_count += 1;
                proc.reductions = 2; // Keep small budget
                assert!(yield_count <= 100, "Too many yields");
            }
            RunResult::Error(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    // Should have yielded multiple times with such a small budget
    assert!(yield_count > 0, "Expected at least one yield");
    // Call stack should be empty after completion
    assert!(proc.at_top_level(), "Call stack should be empty at end");
}
