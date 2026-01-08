// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Bytecode virtual machine for Lona.
//!
//! The VM executes compiled bytecode chunks. It is stateless - all execution
//! state lives in the `Process` struct, which owns registers, heap, and IP.
//!
//! See `docs/architecture/virtual-machine.md` for the full specification.

#[cfg(test)]
mod vm_test;

use crate::bytecode::{decode_a, decode_b, decode_bx, decode_c, decode_opcode, decode_sbx, op};
use crate::intrinsics::{self, IntrinsicError};
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;

/// Maximum number of elements in a tuple literal.
const MAX_TUPLE_ELEMENTS: usize = 64;

/// Runtime error during VM execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    /// Invalid opcode encountered.
    InvalidOpcode(u8),
    /// Instruction pointer out of bounds.
    IpOutOfBounds,
    /// Constant pool index out of bounds.
    ConstantOutOfBounds(u32),
    /// Intrinsic execution failed.
    Intrinsic(IntrinsicError),
    /// No bytecode chunk to execute.
    NoCode,
    /// Out of memory during allocation.
    OutOfMemory,
}

impl From<IntrinsicError> for RuntimeError {
    fn from(e: IntrinsicError) -> Self {
        Self::Intrinsic(e)
    }
}

/// Stateless bytecode virtual machine.
///
/// The VM is a namespace for execution functions. All state lives in `Process`.
pub struct Vm;

impl Vm {
    /// Run bytecode from a Process until completion.
    ///
    /// The process must have a chunk set via `Process::set_chunk`.
    /// Execution state (ip, `x_regs`) is read from and written to the process.
    ///
    /// Returns the value in X0 when execution completes (HALT or RETURN).
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails.
    pub fn run<M: MemorySpace>(proc: &mut Process, mem: &mut M) -> Result<Value, RuntimeError> {
        loop {
            // Access chunk fresh each iteration to avoid borrow conflicts with intrinsics
            let Some(chunk) = proc.chunk.as_ref() else {
                return Err(RuntimeError::NoCode);
            };

            // Bounds check
            if proc.ip >= chunk.code.len() {
                return Err(RuntimeError::IpOutOfBounds);
            }

            // Fetch instruction
            let instr = chunk.code[proc.ip];
            proc.ip += 1;

            // Decode opcode
            let opcode = decode_opcode(instr);

            // For LOADK, get constant value before we release the chunk borrow
            let constant_value = if opcode == op::LOADK {
                let bx = decode_bx(instr);
                Some(
                    chunk
                        .constants
                        .get(bx as usize)
                        .copied()
                        .ok_or(RuntimeError::ConstantOutOfBounds(bx))?,
                )
            } else {
                None
            };

            // Dispatch - chunk borrow ends here, allowing mutable proc access
            match opcode {
                op::LOADNIL => {
                    let a = decode_a(instr) as usize;
                    proc.x_regs[a] = Value::Nil;
                }

                op::LOADBOOL => {
                    let a = decode_a(instr) as usize;
                    let bx = decode_bx(instr);
                    proc.x_regs[a] = Value::bool(bx != 0);
                }

                op::LOADINT => {
                    let a = decode_a(instr) as usize;
                    let sbx = decode_sbx(instr);
                    proc.x_regs[a] = Value::int(i64::from(sbx));
                }

                op::LOADK => {
                    let a = decode_a(instr) as usize;
                    // SAFETY: constant_value is always Some when opcode is LOADK
                    // (computed in the if block above)
                    if let Some(value) = constant_value {
                        proc.x_regs[a] = value;
                    }
                }

                op::MOVE => {
                    let a = decode_a(instr) as usize;
                    let b = decode_b(instr) as usize;
                    proc.x_regs[a] = proc.x_regs[b];
                }

                op::INTRINSIC => {
                    let intrinsic_id = decode_a(instr);
                    let argc = decode_b(instr) as u8;
                    intrinsics::call_intrinsic(intrinsic_id, argc, proc, mem)?;
                }

                op::RETURN | op::HALT => {
                    return Ok(proc.x_regs[0]);
                }

                op::BUILD_TUPLE => {
                    let a = decode_a(instr) as usize;
                    let b = decode_b(instr) as usize; // start register
                    let c = decode_c(instr) as usize; // count

                    // Collect elements from registers into temporary buffer
                    let elem_count = c.min(MAX_TUPLE_ELEMENTS);
                    let mut elements = [Value::Nil; MAX_TUPLE_ELEMENTS];
                    elements[..elem_count].copy_from_slice(&proc.x_regs[b..b + elem_count]);

                    // Allocate tuple
                    let tuple = proc
                        .alloc_tuple(mem, &elements[..elem_count])
                        .ok_or(RuntimeError::OutOfMemory)?;

                    proc.x_regs[a] = tuple;
                }

                _ => {
                    return Err(RuntimeError::InvalidOpcode(opcode));
                }
            }
        }
    }
}

/// Convenience function to execute a process's bytecode.
///
/// The process must have a chunk set via `Process::set_chunk`.
///
/// # Errors
///
/// Returns an error if execution fails.
pub fn execute<M: MemorySpace>(proc: &mut Process, mem: &mut M) -> Result<Value, RuntimeError> {
    Vm::run(proc, mem)
}
