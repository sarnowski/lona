// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the bytecode VM.

mod arithmetic_test;
mod callable_test;
mod function_test;
mod gc_integration_test;
mod integration_test;
mod keyword_test;
mod literal_test;
mod map_test;
mod metadata_test;
mod pattern_test;
mod run_result_test;
mod tuple_test;
mod y_register_test;
mod yield_test;

use super::*;
use crate::Vaddr;
use crate::compiler::compile;
use crate::platform::MockVSpace;
use crate::process::{Process, WorkerId};
use crate::reader::read;
use crate::realm::{Realm, bootstrap};
use crate::scheduler::Worker;
use crate::term::Term;

/// Create a test environment with bootstrapped realm and process.
///
/// Returns `None` if bootstrap fails (should not happen in tests).
pub fn setup() -> Option<(Process, Realm, MockVSpace)> {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(512 * 1024, base);
    let mut realm = Realm::new_for_test(base)?;

    // Increased heap sizes for tests with multiple function definitions
    let (young_base, old_base) = realm.allocate_process_memory(128 * 1024, 32 * 1024)?;
    let mut proc = Process::new(young_base, 128 * 1024, old_base, 32 * 1024);

    let result = bootstrap(&mut realm, &mut mem)?;
    proc.bootstrap(result.ns_var, result.core_ns);

    Some((proc, realm, mem))
}

/// Parse, compile, and execute an expression.
///
/// Returns `Err(RuntimeError::NoCode)` if parsing or compilation fails.
pub fn eval(
    src: &str,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut MockVSpace,
) -> Result<Term, RuntimeError> {
    let expr = read(src, proc, realm, mem)
        .ok()
        .flatten()
        .ok_or(RuntimeError::NoCode)?;
    let chunk = compile(expr, proc, mem, realm).map_err(|_| RuntimeError::NoCode)?;
    if !proc.write_chunk_to_heap(mem, &chunk) {
        return Err(RuntimeError::OutOfMemory);
    }
    let mut worker = Worker::new(WorkerId(0));
    let result = execute(&mut worker, proc, mem, realm);
    worker.reset_x_regs();
    proc.reset();
    result
}
