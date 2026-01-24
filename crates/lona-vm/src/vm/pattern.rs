// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Pattern matching instruction handlers for the VM.
//!
//! This module implements the bytecode instructions used for pattern matching:
//! - Type tests: `IS_NIL`, `IS_BOOL`, `IS_INT`, `IS_TUPLE`, `IS_VECTOR`, `IS_MAP`, `IS_KEYWORD`, `IS_STRING`
//! - Structure tests: `TEST_ARITY`, `TEST_VEC_LEN`
//! - Element extraction: `GET_TUPLE_ELEM`, `GET_VEC_ELEM`
//! - Comparison: `IS_EQ`
//! - Control flow: `JUMP`, `JUMP_IF_FALSE`

use crate::bytecode::{decode_a, decode_b, decode_bx, decode_c, op};
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;
use crate::vm::{RunResult, RuntimeError};

/// Execute a type test instruction.
///
/// Jump to fail label if the value does not match the expected type.
pub const fn execute_type_test(proc: &mut Process, instr: u32, opcode: u8) -> u32 {
    let reg = decode_a(instr) as usize;
    let fail_label = decode_bx(instr) as usize;
    let val = proc.x_regs[reg];

    let matches = match opcode {
        op::IS_NIL => val.is_nil(),
        op::IS_BOOL => matches!(val, Value::Bool(_)),
        op::IS_INT => matches!(val, Value::Int(_)),
        op::IS_TUPLE => val.is_tuple(),
        op::IS_VECTOR => val.is_vector(),
        op::IS_MAP => val.is_map(),
        op::IS_KEYWORD => val.is_keyword(),
        op::IS_STRING => val.is_string(),
        _ => false,
    };

    if !matches {
        proc.ip = fail_label;
    }
    1
}

/// Execute a structure test instruction.
pub fn execute_structure_test<M: MemorySpace>(
    proc: &mut Process,
    mem: &M,
    instr: u32,
    opcode: u8,
) -> u32 {
    let reg = decode_a(instr) as usize;
    let expected_len = decode_b(instr) as usize;
    let fail_label = decode_c(instr) as usize;
    let val = proc.x_regs[reg];

    match opcode {
        op::TEST_ARITY => {
            if !val.is_tuple() {
                proc.ip = fail_label;
                return 1;
            }
            let actual_len = proc.read_tuple_len(mem, val).unwrap_or(usize::MAX);
            if actual_len != expected_len {
                proc.ip = fail_label;
            }
        }
        op::TEST_VEC_LEN => {
            if !val.is_vector() {
                proc.ip = fail_label;
                return 1;
            }
            let actual_len = proc.read_vector_len(mem, val).unwrap_or(usize::MAX);
            if actual_len != expected_len {
                proc.ip = fail_label;
            }
        }
        op::TEST_ARITY_GE => {
            if !val.is_tuple() {
                proc.ip = fail_label;
                return 1;
            }
            let actual_len = proc.read_tuple_len(mem, val).unwrap_or(0);
            if actual_len < expected_len {
                proc.ip = fail_label;
            }
        }
        _ => {}
    }
    1
}

/// Execute an element extraction instruction.
pub fn execute_element_extraction<M: MemorySpace>(
    proc: &mut Process,
    mem: &M,
    instr: u32,
    opcode: u8,
) -> Result<u32, RunResult> {
    let dest = decode_a(instr) as usize;
    let src_reg = decode_b(instr) as usize;
    let index = decode_c(instr) as usize;
    let src_val = proc.x_regs[src_reg];

    let elem = match opcode {
        op::GET_TUPLE_ELEM => proc.read_tuple_element(mem, src_val, index),
        op::GET_VEC_ELEM => proc.read_vector_element(mem, src_val, index),
        _ => None,
    };

    proc.x_regs[dest] = elem.ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
    Ok(1)
}

/// Maximum number of elements for tuple slice.
const MAX_SLICE_ELEMENTS: usize = 64;

/// Execute a tuple slice instruction.
///
/// Creates a new tuple containing elements from index C to end of source tuple.
pub fn execute_tuple_slice<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    instr: u32,
) -> Result<u32, RunResult> {
    let dest = decode_a(instr) as usize;
    let src_reg = decode_b(instr) as usize;
    let start_index = decode_c(instr) as usize;
    let src_val = proc.x_regs[src_reg];

    let len = proc
        .read_tuple_len(mem, src_val)
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

    // Calculate slice length
    let slice_len = len.saturating_sub(start_index);
    let slice_len = slice_len.min(MAX_SLICE_ELEMENTS);

    if slice_len == 0 {
        // Empty tuple
        let empty = proc
            .alloc_tuple(mem, &[])
            .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
        proc.x_regs[dest] = empty;
        return Ok(1);
    }

    // Collect elements from start_index to end
    let mut elements = [Value::Nil; MAX_SLICE_ELEMENTS];
    for (i, elem) in elements.iter_mut().enumerate().take(slice_len) {
        *elem = proc
            .read_tuple_element(mem, src_val, start_index + i)
            .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
    }

    // Allocate new tuple
    let new_tuple = proc
        .alloc_tuple(mem, &elements[..slice_len])
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

    proc.x_regs[dest] = new_tuple;
    Ok(1)
}

/// Execute a pattern matching instruction.
///
/// Handles type tests, structure tests, element extraction, equality, and control flow.
pub fn execute<M: MemorySpace>(
    proc: &mut Process,
    mem: &M,
    instr: u32,
    opcode: u8,
) -> Result<u32, RunResult> {
    match opcode {
        // Type test instructions
        op::IS_NIL
        | op::IS_BOOL
        | op::IS_INT
        | op::IS_TUPLE
        | op::IS_VECTOR
        | op::IS_MAP
        | op::IS_KEYWORD
        | op::IS_STRING => Ok(execute_type_test(proc, instr, opcode)),

        // Structure test instructions
        op::TEST_ARITY | op::TEST_VEC_LEN | op::TEST_ARITY_GE => {
            Ok(execute_structure_test(proc, mem, instr, opcode))
        }

        // Element extraction instructions
        op::GET_TUPLE_ELEM | op::GET_VEC_ELEM => {
            execute_element_extraction(proc, mem, instr, opcode)
        }

        // Equality test
        op::IS_EQ => {
            let reg_a = decode_a(instr) as usize;
            let reg_b = decode_b(instr) as usize;
            let fail_label = decode_c(instr) as usize;
            if proc.x_regs[reg_a] != proc.x_regs[reg_b] {
                proc.ip = fail_label;
            }
            Ok(1)
        }

        // Control flow
        op::JUMP => {
            proc.ip = decode_bx(instr) as usize;
            Ok(1)
        }
        op::JUMP_IF_FALSE => {
            let reg = decode_a(instr) as usize;
            let target = decode_bx(instr) as usize;
            let val = proc.x_regs[reg];
            // Falsy: nil or false
            if val.is_nil() || matches!(val, Value::Bool(false)) {
                proc.ip = target;
            }
            Ok(1)
        }

        // Badmatch error
        op::BADMATCH => {
            let reg = decode_a(instr) as usize;
            let value = proc.x_regs[reg];
            Err(RunResult::Error(RuntimeError::Badmatch { value }))
        }

        _ => Err(RunResult::Error(RuntimeError::InvalidOpcode(opcode))),
    }
}
