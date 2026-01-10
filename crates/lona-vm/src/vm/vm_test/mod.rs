// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the bytecode VM.

#![allow(clippy::unwrap_used, clippy::expect_used)]

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

/// Create a test environment.
pub fn setup() -> (Process, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(128 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let proc = Process::new(1, young_base, young_size, old_base, old_size);
    (proc, mem)
}

/// Parse, compile, and execute an expression.
pub fn eval(src: &str, proc: &mut Process, mem: &mut MockVSpace) -> Result<Value, RuntimeError> {
    let expr = read(src, proc, mem)
        .expect("parse error")
        .expect("empty input");
    let chunk = compile(expr, proc, mem).expect("compile error");
    proc.set_chunk(chunk);
    let result = execute(proc, mem);
    proc.reset();
    result
}
