// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for receive compilation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::Vaddr;
use crate::bytecode::{decode_opcode, op};
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::reader::read;
use crate::realm::{Realm, bootstrap};
/// Create a test environment with bootstrapped realm and process.
fn setup() -> Option<(Process, Realm, MockVSpace)> {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(512 * 1024, base);

    let mut realm = Realm::new_for_test(base).unwrap();
    let (young_base, old_base) = realm.allocate_process_memory(64 * 1024, 16 * 1024)?;
    let mut proc = Process::new(young_base, 64 * 1024, old_base, 16 * 1024);

    let result = bootstrap(&mut realm, &mut mem)?;
    proc.bootstrap(result.ns_var, result.core_ns);

    Some((proc, realm, mem))
}

/// Parse and compile an expression, returning the opcodes.
fn compile_opcodes(src: &str) -> Vec<u8> {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let form = read(src, &mut proc, &mut realm, &mut mem).unwrap().unwrap();
    let chunk = compile(form, &mut proc, &mut mem, &mut realm).unwrap();
    chunk.code().iter().map(|&i| decode_opcode(i)).collect()
}

/// Try to compile an expression, return whether it succeeds.
fn try_compile(src: &str) -> Result<Vec<u8>, CompileError> {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let form = read(src, &mut proc, &mut realm, &mut mem).unwrap().unwrap();
    let chunk = compile(form, &mut proc, &mut mem, &mut realm)?;
    Ok(chunk.code().iter().map(|&i| decode_opcode(i)).collect())
}

#[test]
fn compile_receive_after_zero() {
    let opcodes = compile_opcodes("(receive :after 0 :timeout)");

    assert!(
        opcodes.contains(&op::RECV_TIMEOUT_INIT),
        "should emit RECV_TIMEOUT_INIT"
    );
    // Timeout-only (no clauses) uses the fast path: no RECV_PEEK
    assert!(opcodes.contains(&op::RECV_WAIT), "should emit RECV_WAIT");
}

#[test]
fn compile_receive_with_binding_pattern() {
    let opcodes = compile_opcodes("(receive x x :after 0 :timeout)");

    assert!(opcodes.contains(&op::RECV_PEEK), "should emit RECV_PEEK");
    assert!(
        opcodes.contains(&op::RECV_ACCEPT),
        "should emit RECV_ACCEPT"
    );
    assert!(opcodes.contains(&op::RECV_WAIT), "should emit RECV_WAIT");
    assert!(
        opcodes.contains(&op::RECV_NEXT),
        "should emit RECV_NEXT for no-match advance"
    );
}

#[test]
fn compile_receive_with_tuple_pattern() {
    let opcodes = compile_opcodes("(receive [:ok v] v :after 0 :timeout)");

    assert!(opcodes.contains(&op::RECV_PEEK));
    assert!(opcodes.contains(&op::IS_TUPLE), "should test tuple type");
    assert!(opcodes.contains(&op::TEST_ARITY), "should test arity");
    assert!(opcodes.contains(&op::RECV_ACCEPT));
}

#[test]
fn compile_receive_no_timeout() {
    let opcodes = compile_opcodes("(receive x x)");

    assert!(
        !opcodes.contains(&op::RECV_TIMEOUT_INIT),
        "should NOT emit RECV_TIMEOUT_INIT without :after"
    );
    assert!(opcodes.contains(&op::RECV_PEEK));
    assert!(opcodes.contains(&op::RECV_WAIT));
}

#[test]
fn compile_receive_multiple_clauses() {
    let opcodes = compile_opcodes("(receive :a 1 :b 2 :after 0 :timeout)");

    // Two clauses should generate two RECV_ACCEPT
    let mut accept_count = 0u32;
    for &o in &opcodes {
        if o == op::RECV_ACCEPT {
            accept_count += 1;
        }
    }
    assert_eq!(accept_count, 2, "should emit RECV_ACCEPT per clause");
}

#[test]
fn compile_receive_with_guard() {
    let opcodes = compile_opcodes("(receive x when (nil? x) x :after 0 :timeout)");

    assert!(opcodes.contains(&op::RECV_PEEK));
    assert!(
        opcodes.contains(&op::JUMP_IF_FALSE),
        "should emit JUMP_IF_FALSE for guard"
    );
    assert!(opcodes.contains(&op::RECV_ACCEPT));
}

#[test]
fn compile_receive_timeout_only_no_recv_peek() {
    // Timeout-only receive (no clauses) should NOT emit RECV_PEEK
    let opcodes = compile_opcodes("(receive :after 0 :done)");

    assert!(
        opcodes.contains(&op::RECV_TIMEOUT_INIT),
        "should emit RECV_TIMEOUT_INIT"
    );
    assert!(opcodes.contains(&op::RECV_WAIT), "should emit RECV_WAIT");
    assert!(
        !opcodes.contains(&op::RECV_PEEK),
        "timeout-only receive should NOT emit RECV_PEEK"
    );
}

#[test]
fn compile_receive_empty_is_error() {
    let result = try_compile("(receive)");
    assert!(result.is_err(), "empty receive should be compile error");
}
