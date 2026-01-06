// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Bytecode virtual machine for Lona.
//!
//! The VM executes compiled bytecode chunks. It uses a register-based
//! architecture with 256 X registers for temporaries.
//!
//! See `docs/architecture/virtual-machine.md` for the full specification.

#[cfg(test)]
mod vm_test;

use crate::bytecode::{Chunk, decode_a, decode_b, decode_bx, decode_opcode, decode_sbx, op};
use crate::heap::Heap;
use crate::intrinsics::{self, IntrinsicError};
use crate::platform::MemorySpace;
use crate::value::Value;

/// Number of X registers.
const X_REG_COUNT: usize = 256;

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
}

impl From<IntrinsicError> for RuntimeError {
    fn from(e: IntrinsicError) -> Self {
        Self::Intrinsic(e)
    }
}

/// Bytecode virtual machine.
///
/// Executes compiled bytecode chunks using a register-based interpreter.
pub struct Vm<'a, M: MemorySpace> {
    /// The bytecode chunk being executed.
    chunk: &'a Chunk,
    /// Instruction pointer (index into code array).
    ip: usize,
    /// X registers (temporaries).
    x_regs: [Value; X_REG_COUNT],
    /// Reference to the heap.
    heap: &'a mut Heap,
    /// Reference to the memory space.
    mem: &'a mut M,
}

impl<'a, M: MemorySpace> Vm<'a, M> {
    /// Create a new VM to execute a chunk.
    #[must_use]
    pub const fn new(chunk: &'a Chunk, heap: &'a mut Heap, mem: &'a mut M) -> Self {
        Self {
            chunk,
            ip: 0,
            x_regs: [Value::Nil; X_REG_COUNT],
            heap,
            mem,
        }
    }

    /// Run the VM until completion.
    ///
    /// Returns the value in X0 when execution completes (HALT or RETURN).
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails.
    pub fn run(&mut self) -> Result<Value, RuntimeError> {
        loop {
            // Bounds check
            if self.ip >= self.chunk.code.len() {
                return Err(RuntimeError::IpOutOfBounds);
            }

            // Fetch instruction
            let instr = self.chunk.code[self.ip];
            self.ip += 1;

            // Decode opcode
            let opcode = decode_opcode(instr);

            // Dispatch
            match opcode {
                op::LOADNIL => {
                    let a = decode_a(instr) as usize;
                    self.x_regs[a] = Value::Nil;
                }

                op::LOADBOOL => {
                    let a = decode_a(instr) as usize;
                    let bx = decode_bx(instr);
                    self.x_regs[a] = Value::bool(bx != 0);
                }

                op::LOADINT => {
                    let a = decode_a(instr) as usize;
                    let sbx = decode_sbx(instr);
                    self.x_regs[a] = Value::int(i64::from(sbx));
                }

                op::LOADK => {
                    let a = decode_a(instr) as usize;
                    let bx = decode_bx(instr);
                    let value = self
                        .chunk
                        .constants
                        .get(bx as usize)
                        .copied()
                        .ok_or(RuntimeError::ConstantOutOfBounds(bx))?;
                    self.x_regs[a] = value;
                }

                op::MOVE => {
                    let a = decode_a(instr) as usize;
                    let b = decode_b(instr) as usize;
                    self.x_regs[a] = self.x_regs[b];
                }

                op::INTRINSIC => {
                    let intrinsic_id = decode_a(instr);
                    let argc = decode_b(instr) as u8;
                    intrinsics::call_intrinsic(
                        intrinsic_id,
                        argc,
                        &mut self.x_regs,
                        self.heap,
                        self.mem,
                    )?;
                }

                op::RETURN | op::HALT => {
                    return Ok(self.x_regs[0]);
                }

                _ => {
                    return Err(RuntimeError::InvalidOpcode(opcode));
                }
            }
        }
    }
}

/// Convenience function to execute a chunk.
///
/// # Errors
///
/// Returns an error if execution fails.
pub fn execute<M: MemorySpace>(
    chunk: &Chunk,
    heap: &mut Heap,
    mem: &mut M,
) -> Result<Value, RuntimeError> {
    Vm::new(chunk, heap, mem).run()
}
