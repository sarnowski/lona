// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the bytecode VM.

mod arithmetic_test;
mod callable_test;
mod function_test;
mod integration_test;
mod keyword_test;
mod literal_test;
mod map_test;
mod metadata_test;
mod tuple_test;

use super::*;
use crate::Vaddr;
use crate::compiler::compile;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::reader::read;
use crate::realm::{Realm, bootstrap};

/// Create a test environment with bootstrapped realm and process.
///
/// Returns `None` if bootstrap fails (should not happen in tests).
pub fn setup() -> Option<(Process, Realm, MockVSpace)> {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let mut proc = Process::new(1, young_base, young_size, old_base, old_size);

    // Create realm at a higher address
    let realm_base = base.add(128 * 1024);
    let mut realm = Realm::new(realm_base, 64 * 1024);

    // Bootstrap realm and process
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
) -> Result<Value, RuntimeError> {
    let expr = read(src, proc, mem)
        .ok()
        .flatten()
        .ok_or(RuntimeError::NoCode)?;
    let chunk = compile(expr, proc, mem, realm).map_err(|_| RuntimeError::NoCode)?;
    proc.set_chunk(chunk);
    let result = execute(proc, mem, realm);
    proc.reset();
    result
}
