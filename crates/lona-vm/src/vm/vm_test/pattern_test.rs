// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for pattern matching instructions.
//!
//! Phase 1 instructions for the `match` special form:
//! - Type tests: `IS_NIL`, `IS_BOOL`, `IS_INT`, `IS_TUPLE`, `IS_VECTOR`, `IS_MAP`, `IS_KEYWORD`, `IS_STRING`
//! - Structure tests: `TEST_ARITY`, `TEST_VEC_LEN`
//! - Element extraction: `GET_TUPLE_ELEM`, `GET_VEC_ELEM`
//! - Comparison: `IS_EQ`
//! - Control flow: `JUMP`, `JUMP_IF_FALSE`

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::Vaddr;
use crate::bytecode::{Chunk, encode_abc, encode_abx, op};
use crate::platform::MockVSpace;
use crate::process::{Process, WorkerId};
use crate::realm::{Realm, bootstrap};
use crate::scheduler::Worker;
use crate::value::Value;
use crate::vm::{RunResult, Vm};

// =============================================================================
// Helper functions
// =============================================================================

/// Create a test environment (worker, process, realm, memory).
fn create_test_env() -> (Worker, Process, Realm, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let mut proc = Process::new(young_base, young_size, old_base, old_size);

    let realm_base = base.add(128 * 1024);
    let mut realm = Realm::new(realm_base, 64 * 1024);

    let result = bootstrap(&mut realm, &mut mem).unwrap();
    proc.bootstrap(result.ns_var, result.core_ns);

    let worker = Worker::new(WorkerId(0));
    (worker, proc, realm, mem)
}

/// Run a chunk and return the result.
fn run_chunk(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut MockVSpace,
    realm: &mut Realm,
    chunk: Chunk,
) -> RunResult {
    proc.set_chunk(chunk);
    proc.reset_reductions();
    Vm::run(worker, proc, mem, realm)
}

// =============================================================================
// IS_NIL tests
// =============================================================================

#[test]
fn is_nil_matches_nil() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // X0 = nil
    // IS_NIL X0, fail (if NOT nil, jump to fail)
    // LOADINT X0, 1 (success path)
    // HALT
    // fail: LOADINT X0, 0
    // HALT
    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADNIL, 0, 0)); // 0: X0 = nil
    chunk.emit(encode_abx(op::IS_NIL, 0, 4)); // 1: if X0 is NOT nil, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: X0 = 1 (success)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: X0 = 0 (fail)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_nil_rejects_integer() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // X0 = 42
    // IS_NIL X0, fail (if NOT nil, jump to fail)
    // LOADINT X0, 1 (success - should not reach)
    // HALT
    // fail: LOADINT X0, 0
    // HALT
    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42
    chunk.emit(encode_abx(op::IS_NIL, 0, 4)); // 1: if X0 is NOT nil, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: X0 = 1 (success)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: X0 = 0 (fail)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

#[test]
fn is_nil_rejects_false() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // X0 = false
    // IS_NIL X0, fail
    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADBOOL, 0, 0)); // 0: X0 = false
    chunk.emit(encode_abx(op::IS_NIL, 0, 4)); // 1: if NOT nil, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// IS_BOOL tests
// =============================================================================

#[test]
fn is_bool_matches_true() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADBOOL, 0, 1)); // 0: X0 = true
    chunk.emit(encode_abx(op::IS_BOOL, 0, 4)); // 1: if NOT bool, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_bool_matches_false() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADBOOL, 0, 0)); // 0: X0 = false
    chunk.emit(encode_abx(op::IS_BOOL, 0, 4)); // 1: if NOT bool, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_bool_rejects_nil() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADNIL, 0, 0)); // 0: X0 = nil
    chunk.emit(encode_abx(op::IS_BOOL, 0, 4)); // 1: if NOT bool, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// IS_INT tests
// =============================================================================

#[test]
fn is_int_matches_integer() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42
    chunk.emit(encode_abx(op::IS_INT, 0, 4)); // 1: if NOT int, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_int_rejects_bool() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADBOOL, 0, 1)); // 0: X0 = true
    chunk.emit(encode_abx(op::IS_INT, 0, 4)); // 1: if NOT int, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// IS_TUPLE tests
// =============================================================================

#[test]
fn is_tuple_matches_tuple() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // Build a tuple first
    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abx(op::LOADINT, 2, 2)); // 1: X2 = 2
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 2)); // 2: X0 = [X1, X2]
    chunk.emit(encode_abx(op::IS_TUPLE, 0, 7)); // 3: if NOT tuple, jump to 7
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 4: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 7: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 8: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_tuple_rejects_vector() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // Build a vector
    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abc(op::BUILD_VECTOR, 0, 1, 1)); // 1: X0 = {X1}
    chunk.emit(encode_abx(op::IS_TUPLE, 0, 5)); // 2: if NOT tuple, jump to 5
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 5: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

#[test]
fn is_tuple_rejects_integer() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42
    chunk.emit(encode_abx(op::IS_TUPLE, 0, 4)); // 1: if NOT tuple, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// IS_VECTOR tests
// =============================================================================

#[test]
fn is_vector_matches_vector() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abc(op::BUILD_VECTOR, 0, 1, 1)); // 1: X0 = {X1}
    chunk.emit(encode_abx(op::IS_VECTOR, 0, 5)); // 2: if NOT vector, jump to 5
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 5: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_vector_rejects_tuple() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 1)); // 1: X0 = [X1]
    chunk.emit(encode_abx(op::IS_VECTOR, 0, 5)); // 2: if NOT vector, jump to 5
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 5: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// IS_MAP tests
// =============================================================================

#[test]
fn is_map_matches_map() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // Allocate a keyword for the map key
    let keyword = proc.alloc_keyword(&mut mem, "key").unwrap();
    worker.x_regs[1] = keyword;
    worker.x_regs[2] = Value::int(42);

    let mut chunk = Chunk::new();
    // Note: X1 and X2 are already set by test setup
    chunk.emit(encode_abc(op::BUILD_MAP, 0, 1, 1)); // 0: X0 = %{X1: X2}
    chunk.emit(encode_abx(op::IS_MAP, 0, 5)); // 1: if NOT map, jump to 5
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 5: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_map_rejects_tuple() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 1)); // 1: X0 = [X1]
    chunk.emit(encode_abx(op::IS_MAP, 0, 5)); // 2: if NOT map, jump to 5
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 5: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// IS_KEYWORD tests
// =============================================================================

#[test]
fn is_keyword_matches_keyword() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // Allocate a keyword
    let keyword = proc.alloc_keyword(&mut mem, "test").unwrap();
    worker.x_regs[0] = keyword;

    let mut chunk = Chunk::new();
    // X0 already contains keyword
    chunk.emit(encode_abx(op::IS_KEYWORD, 0, 4)); // 0: if NOT keyword, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 1: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 2: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_keyword_rejects_symbol() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // Allocate a symbol
    let symbol = proc.alloc_symbol(&mut mem, "test").unwrap();
    worker.x_regs[0] = symbol;

    let mut chunk = Chunk::new();
    // X0 already contains symbol
    chunk.emit(encode_abx(op::IS_KEYWORD, 0, 4)); // 0: if NOT keyword, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 1: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 2: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// IS_STRING tests
// =============================================================================

#[test]
fn is_string_matches_string() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // Allocate a string
    let string = proc.alloc_string(&mut mem, "hello").unwrap();
    worker.x_regs[0] = string;

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::IS_STRING, 0, 4)); // 0: if NOT string, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 1: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 2: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_string_rejects_integer() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42
    chunk.emit(encode_abx(op::IS_STRING, 0, 4)); // 1: if NOT string, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// TEST_ARITY tests (tuple length)
// =============================================================================

#[test]
fn test_arity_matches_correct_size() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abx(op::LOADINT, 2, 2)); // 1: X2 = 2
    chunk.emit(encode_abx(op::LOADINT, 3, 3)); // 2: X3 = 3
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3)); // 3: X0 = [1, 2, 3]
    chunk.emit(encode_abc(op::TEST_ARITY, 0, 3, 8)); // 4: if X0.len != 3, jump to 8
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 5: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 7: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 8: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 9: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn test_arity_rejects_wrong_size() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abx(op::LOADINT, 2, 2)); // 1: X2 = 2
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 2)); // 2: X0 = [1, 2]
    chunk.emit(encode_abc(op::TEST_ARITY, 0, 3, 7)); // 3: if X0.len != 3, jump to 7
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 4: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 7: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 8: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

#[test]
fn test_arity_rejects_non_tuple() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42
    chunk.emit(encode_abc(op::TEST_ARITY, 0, 1, 5)); // 1: if X0.len != 1, jump to 5
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 5: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// TEST_VEC_LEN tests (vector length)
// =============================================================================

#[test]
fn test_vec_len_matches_correct_size() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abx(op::LOADINT, 2, 2)); // 1: X2 = 2
    chunk.emit(encode_abc(op::BUILD_VECTOR, 0, 1, 2)); // 2: X0 = {1, 2}
    chunk.emit(encode_abc(op::TEST_VEC_LEN, 0, 2, 7)); // 3: if X0.len != 2, jump to 7
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 4: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 7: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 8: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn test_vec_len_rejects_wrong_size() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abc(op::BUILD_VECTOR, 0, 1, 1)); // 1: X0 = {1}
    chunk.emit(encode_abc(op::TEST_VEC_LEN, 0, 2, 6)); // 2: if X0.len != 2, jump to 6
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 6: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 7: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

// =============================================================================
// GET_TUPLE_ELEM tests
// =============================================================================

#[test]
fn get_tuple_elem_extracts_first() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 10)); // 0: X1 = 10
    chunk.emit(encode_abx(op::LOADINT, 2, 20)); // 1: X2 = 20
    chunk.emit(encode_abx(op::LOADINT, 3, 30)); // 2: X3 = 30
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3)); // 3: X0 = [10, 20, 30]
    chunk.emit(encode_abc(op::GET_TUPLE_ELEM, 4, 0, 0)); // 4: X4 = X0[0]
    chunk.emit(encode_abc(op::MOVE, 0, 4, 0)); // 5: X0 = X4
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(10))));
}

#[test]
fn get_tuple_elem_extracts_middle() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 10)); // 0: X1 = 10
    chunk.emit(encode_abx(op::LOADINT, 2, 20)); // 1: X2 = 20
    chunk.emit(encode_abx(op::LOADINT, 3, 30)); // 2: X3 = 30
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3)); // 3: X0 = [10, 20, 30]
    chunk.emit(encode_abc(op::GET_TUPLE_ELEM, 4, 0, 1)); // 4: X4 = X0[1]
    chunk.emit(encode_abc(op::MOVE, 0, 4, 0)); // 5: X0 = X4
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(20))));
}

#[test]
fn get_tuple_elem_extracts_last() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 10)); // 0: X1 = 10
    chunk.emit(encode_abx(op::LOADINT, 2, 20)); // 1: X2 = 20
    chunk.emit(encode_abx(op::LOADINT, 3, 30)); // 2: X3 = 30
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3)); // 3: X0 = [10, 20, 30]
    chunk.emit(encode_abc(op::GET_TUPLE_ELEM, 4, 0, 2)); // 4: X4 = X0[2]
    chunk.emit(encode_abc(op::MOVE, 0, 4, 0)); // 5: X0 = X4
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(30))));
}

// =============================================================================
// GET_VEC_ELEM tests
// =============================================================================

#[test]
fn get_vec_elem_extracts_first() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 100)); // 0: X1 = 100
    chunk.emit(encode_abx(op::LOADINT, 2, 200)); // 1: X2 = 200
    chunk.emit(encode_abc(op::BUILD_VECTOR, 0, 1, 2)); // 2: X0 = {100, 200}
    chunk.emit(encode_abc(op::GET_VEC_ELEM, 3, 0, 0)); // 3: X3 = X0[0]
    chunk.emit(encode_abc(op::MOVE, 0, 3, 0)); // 4: X0 = X3
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(100))));
}

#[test]
fn get_vec_elem_extracts_last() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 1, 100)); // 0: X1 = 100
    chunk.emit(encode_abx(op::LOADINT, 2, 200)); // 1: X2 = 200
    chunk.emit(encode_abc(op::BUILD_VECTOR, 0, 1, 2)); // 2: X0 = {100, 200}
    chunk.emit(encode_abc(op::GET_VEC_ELEM, 3, 0, 1)); // 3: X3 = X0[1]
    chunk.emit(encode_abc(op::MOVE, 0, 3, 0)); // 4: X0 = X3
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(200))));
}

// =============================================================================
// IS_EQ tests
// =============================================================================

#[test]
fn is_eq_matches_equal_integers() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42
    chunk.emit(encode_abx(op::LOADINT, 1, 42)); // 1: X1 = 42
    chunk.emit(encode_abc(op::IS_EQ, 0, 1, 6)); // 2: if X0 != X1, jump to 6
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 6: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 7: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_eq_rejects_different_integers() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42
    chunk.emit(encode_abx(op::LOADINT, 1, 99)); // 1: X1 = 99
    chunk.emit(encode_abc(op::IS_EQ, 0, 1, 6)); // 2: if X0 != X1, jump to 6
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 6: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 7: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

#[test]
fn is_eq_matches_nil() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADNIL, 0, 0)); // 0: X0 = nil
    chunk.emit(encode_abx(op::LOADNIL, 1, 0)); // 1: X1 = nil
    chunk.emit(encode_abc(op::IS_EQ, 0, 1, 6)); // 2: if X0 != X1, jump to 6
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 6: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 7: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn is_eq_rejects_different_types() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42 (int)
    chunk.emit(encode_abx(op::LOADNIL, 1, 0)); // 1: X1 = nil
    chunk.emit(encode_abc(op::IS_EQ, 0, 1, 6)); // 2: if X0 != X1, jump to 6
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 6: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 7: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

#[test]
fn is_eq_matches_same_keywords() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    // Allocate the same keyword twice (interned at realm level, so same address)
    let kw1 = realm.intern_keyword(&mut mem, "test").unwrap();
    let kw2 = realm.intern_keyword(&mut mem, "test").unwrap();
    worker.x_regs[0] = kw1;
    worker.x_regs[1] = kw2;

    let mut chunk = Chunk::new();
    chunk.emit(encode_abc(op::IS_EQ, 0, 1, 4)); // 0: if X0 != X1, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 1: success
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 2: halt
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

// =============================================================================
// JUMP tests
// =============================================================================

#[test]
fn jump_unconditional() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::JUMP, 0, 3)); // 0: jump to 3
    chunk.emit(encode_abx(op::LOADINT, 0, 999)); // 1: should be skipped
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 2: should be skipped
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 3: X0 = 42
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(42))));
}

#[test]
fn jump_forward_to_end() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42
    chunk.emit(encode_abx(op::JUMP, 0, 4)); // 1: jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 999)); // 2: should be skipped
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: should be skipped
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(42))));
}

// =============================================================================
// JUMP_IF_FALSE tests
// =============================================================================

#[test]
fn jump_if_false_on_nil() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADNIL, 0, 0)); // 0: X0 = nil
    chunk.emit(encode_abx(op::JUMP_IF_FALSE, 0, 4)); // 1: if X0 is falsy, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: not taken
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: X0 = 0 (jumped here)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

#[test]
fn jump_if_false_on_false() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADBOOL, 0, 0)); // 0: X0 = false
    chunk.emit(encode_abx(op::JUMP_IF_FALSE, 0, 4)); // 1: if X0 is falsy, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: not taken
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: X0 = 0 (jumped here)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(0))));
}

#[test]
fn jump_if_false_not_taken_on_true() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADBOOL, 0, 1)); // 0: X0 = true
    chunk.emit(encode_abx(op::JUMP_IF_FALSE, 0, 4)); // 1: if X0 is falsy, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: taken
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: not reached
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn jump_if_false_not_taken_on_integer() {
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42 (truthy)
    chunk.emit(encode_abx(op::JUMP_IF_FALSE, 0, 4)); // 1: if X0 is falsy, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: taken
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: not reached
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn jump_if_false_not_taken_on_zero() {
    // Zero is truthy in Lonala (only nil and false are falsy)
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 0: X0 = 0 (truthy!)
    chunk.emit(encode_abx(op::JUMP_IF_FALSE, 0, 4)); // 1: if X0 is falsy, jump to 4
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 2: taken (0 is truthy)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 3: halt
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 4: not reached
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

// =============================================================================
// Integration tests: combined pattern matching sequence
// =============================================================================

#[test]
fn pattern_match_literal_integer() {
    // Simulate: (match 42 42 :yes _ :no)
    // Should return 1 (representing :yes)
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    // Setup: value to match
    chunk.emit(encode_abx(op::LOADINT, 0, 42)); // 0: X0 = 42

    // Clause 1: pattern = 42
    chunk.emit(encode_abx(op::LOADINT, 1, 42)); // 1: X1 = 42 (pattern)
    chunk.emit(encode_abc(op::IS_EQ, 0, 1, 6)); // 2: if X0 != X1, jump to clause 2
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 3: success, return 1
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt

    // Clause 2: pattern = _ (wildcard - always matches)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 6: return 0
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 7: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn pattern_match_falls_through_to_wildcard() {
    // Simulate: (match 99 42 :no _ :yes)
    // Value 99 doesn't match 42, falls through to wildcard
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    // Setup: value to match
    chunk.emit(encode_abx(op::LOADINT, 0, 99)); // 0: X0 = 99

    // Clause 1: pattern = 42
    chunk.emit(encode_abx(op::LOADINT, 1, 42)); // 1: X1 = 42 (pattern)
    chunk.emit(encode_abc(op::IS_EQ, 0, 1, 6)); // 2: if X0 != X1, jump to clause 2
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 3: should not reach
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 4: halt

    // Clause 2: pattern = _ (wildcard)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 5: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 1)); // 6: success (wildcard matched)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 7: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(1))));
}

#[test]
fn pattern_match_tuple_destructure() {
    // Simulate: (match [1 2 3] [a b c] (+ a c))
    // Returns a + c = 1 + 3 = 4
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    // Setup: build tuple [1, 2, 3] in X0
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abx(op::LOADINT, 2, 2)); // 1: X2 = 2
    chunk.emit(encode_abx(op::LOADINT, 3, 3)); // 2: X3 = 3
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3)); // 3: X0 = [1, 2, 3]

    // Pattern: [a b c]
    // Test 1: is it a tuple?
    chunk.emit(encode_abx(op::IS_TUPLE, 0, 14)); // 4: if NOT tuple, jump to fail

    // Test 2: does it have arity 3?
    chunk.emit(encode_abc(op::TEST_ARITY, 0, 3, 14)); // 5: if arity != 3, jump to fail

    // Extract elements
    chunk.emit(encode_abc(op::GET_TUPLE_ELEM, 4, 0, 0)); // 6: X4 = X0[0] (a)
    chunk.emit(encode_abc(op::GET_TUPLE_ELEM, 5, 0, 1)); // 7: X5 = X0[1] (b)
    chunk.emit(encode_abc(op::GET_TUPLE_ELEM, 6, 0, 2)); // 8: X6 = X0[2] (c)

    // Compute a + c (using intrinsic ADD = 0)
    chunk.emit(encode_abc(op::MOVE, 1, 4, 0)); // 9: X1 = a
    chunk.emit(encode_abc(op::MOVE, 2, 6, 0)); // 10: X2 = c
    chunk.emit(encode_abc(op::INTRINSIC, 0, 2, 0)); // 11: X0 = add(X1, X2)
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 12: halt

    // Fail label
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 13: (placeholder)
    chunk.emit(encode_abx(op::LOADINT, 0, 0)); // 14: fail
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 15: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(4))));
}

// =============================================================================
// TUPLE_SLICE tests
// =============================================================================

#[test]
fn tuple_slice_from_middle() {
    // TUPLE_SLICE [1 2 3] starting at index 1 -> [2 3]
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    // Build tuple [1, 2, 3] in X0
    chunk.emit(encode_abx(op::LOADINT, 1, 1)); // 0: X1 = 1
    chunk.emit(encode_abx(op::LOADINT, 2, 2)); // 1: X2 = 2
    chunk.emit(encode_abx(op::LOADINT, 3, 3)); // 2: X3 = 3
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3)); // 3: X0 = [1, 2, 3]

    // TUPLE_SLICE X4 = X0[1..]
    chunk.emit(encode_abc(op::TUPLE_SLICE, 4, 0, 1)); // 4: X4 = [2, 3]

    // Verify: extract first element of slice (should be 2)
    chunk.emit(encode_abc(op::GET_TUPLE_ELEM, 0, 4, 0)); // 5: X0 = X4[0]
    chunk.emit(encode_abc(op::HALT, 0, 0, 0)); // 6: halt

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(2))));
}

#[test]
fn tuple_slice_from_end() {
    // TUPLE_SLICE [1 2 3] starting at index 2 -> [3]
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    // Build tuple [1, 2, 3] in X0
    chunk.emit(encode_abx(op::LOADINT, 1, 1));
    chunk.emit(encode_abx(op::LOADINT, 2, 2));
    chunk.emit(encode_abx(op::LOADINT, 3, 3));
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3));

    // TUPLE_SLICE X4 = X0[2..]
    chunk.emit(encode_abc(op::TUPLE_SLICE, 4, 0, 2));

    // Verify: extract first element of slice (should be 3)
    chunk.emit(encode_abc(op::GET_TUPLE_ELEM, 0, 4, 0));
    chunk.emit(encode_abc(op::HALT, 0, 0, 0));

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    assert!(matches!(result, RunResult::Completed(Value::Int(3))));
}

#[test]
fn tuple_slice_empty_result() {
    // TUPLE_SLICE [1 2 3] starting at index 3 -> []
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    // Build tuple [1, 2, 3] in X0
    chunk.emit(encode_abx(op::LOADINT, 1, 1));
    chunk.emit(encode_abx(op::LOADINT, 2, 2));
    chunk.emit(encode_abx(op::LOADINT, 3, 3));
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3));

    // TUPLE_SLICE X4 = X0[3..]
    chunk.emit(encode_abc(op::TUPLE_SLICE, 4, 0, 3));

    // Check it's a tuple and move to X0 for return
    chunk.emit(encode_abc(op::MOVE, 0, 4, 0));
    chunk.emit(encode_abc(op::HALT, 0, 0, 0));

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    // Should be an empty tuple
    if let RunResult::Completed(val) = result {
        assert!(val.is_tuple());
        let len = proc.read_tuple_len(&mem, val).unwrap();
        assert_eq!(len, 0);
    } else {
        panic!("Expected Completed result");
    }
}

#[test]
fn tuple_slice_full_copy() {
    // TUPLE_SLICE [1 2 3] starting at index 0 -> [1 2 3]
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    // Build tuple [1, 2, 3] in X0
    chunk.emit(encode_abx(op::LOADINT, 1, 1));
    chunk.emit(encode_abx(op::LOADINT, 2, 2));
    chunk.emit(encode_abx(op::LOADINT, 3, 3));
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3));

    // TUPLE_SLICE X4 = X0[0..]
    chunk.emit(encode_abc(op::TUPLE_SLICE, 4, 0, 0));

    // Move to X0 for return
    chunk.emit(encode_abc(op::MOVE, 0, 4, 0));
    chunk.emit(encode_abc(op::HALT, 0, 0, 0));

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    if let RunResult::Completed(val) = result {
        assert!(val.is_tuple());
        let len = proc.read_tuple_len(&mem, val).unwrap();
        assert_eq!(len, 3);
        let elem0 = proc.read_tuple_element(&mem, val, 0).unwrap();
        let elem1 = proc.read_tuple_element(&mem, val, 1).unwrap();
        let elem2 = proc.read_tuple_element(&mem, val, 2).unwrap();
        assert_eq!(elem0, Value::Int(1));
        assert_eq!(elem1, Value::Int(2));
        assert_eq!(elem2, Value::Int(3));
    } else {
        panic!("Expected Completed result");
    }
}

#[test]
fn tuple_slice_beyond_length() {
    // TUPLE_SLICE [1 2 3] starting at index 5 -> []
    let (mut worker, mut proc, mut realm, mut mem) = create_test_env();

    let mut chunk = Chunk::new();
    // Build tuple [1, 2, 3] in X0
    chunk.emit(encode_abx(op::LOADINT, 1, 1));
    chunk.emit(encode_abx(op::LOADINT, 2, 2));
    chunk.emit(encode_abx(op::LOADINT, 3, 3));
    chunk.emit(encode_abc(op::BUILD_TUPLE, 0, 1, 3));

    // TUPLE_SLICE X4 = X0[5..] (beyond length)
    chunk.emit(encode_abc(op::TUPLE_SLICE, 4, 0, 5));

    // Move to X0 for return
    chunk.emit(encode_abc(op::MOVE, 0, 4, 0));
    chunk.emit(encode_abc(op::HALT, 0, 0, 0));

    let result = run_chunk(&mut worker, &mut proc, &mut mem, &mut realm, chunk);
    if let RunResult::Completed(val) = result {
        assert!(val.is_tuple());
        let len = proc.read_tuple_len(&mem, val).unwrap();
        assert_eq!(len, 0);
    } else {
        panic!("Expected Completed result");
    }
}
