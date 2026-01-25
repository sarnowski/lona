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
use crate::intrinsics::XRegs;
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::term::Term;
use crate::vm::{RunResult, RuntimeError};

/// Execute a type test instruction.
///
/// Jump to fail label if the value does not match the expected type.
pub fn execute_type_test<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &mut Process,
    mem: &M,
    instr: u32,
    opcode: u8,
) -> u32 {
    let reg = decode_a(instr) as usize;
    let fail_label = decode_bx(instr) as usize;
    let term = x_regs[reg];

    // Type tests using Term directly
    let matches = match opcode {
        op::IS_NIL => term.is_nil(),
        op::IS_BOOL => term.is_boolean(),
        op::IS_INT => term.is_small_int(),
        op::IS_TUPLE => proc.is_term_tuple(mem, term),
        op::IS_VECTOR => proc.is_term_vector(mem, term),
        op::IS_MAP => proc.is_term_map(mem, term),
        op::IS_KEYWORD => proc.is_term_keyword(term),
        op::IS_STRING => proc.is_term_string(mem, term),
        _ => false,
    };

    if !matches {
        proc.ip = fail_label;
    }
    1
}

/// Execute a structure test instruction.
pub fn execute_structure_test<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &mut Process,
    mem: &M,
    instr: u32,
    opcode: u8,
) -> u32 {
    let reg = decode_a(instr) as usize;
    let expected_len = decode_b(instr) as usize;
    let fail_label = decode_c(instr) as usize;
    let term = x_regs[reg];

    match opcode {
        op::TEST_ARITY => {
            let actual_len = proc.read_term_tuple_len(mem, term).unwrap_or(usize::MAX);
            if actual_len == usize::MAX || actual_len != expected_len {
                proc.ip = fail_label;
            }
        }
        op::TEST_VEC_LEN => {
            let actual_len = proc.read_term_vector_len(mem, term).unwrap_or(usize::MAX);
            if actual_len == usize::MAX || actual_len != expected_len {
                proc.ip = fail_label;
            }
        }
        op::TEST_ARITY_GE => {
            let actual_len = proc.read_term_tuple_len(mem, term).unwrap_or(0);
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
    x_regs: &mut XRegs,
    proc: &Process,
    mem: &M,
    instr: u32,
    opcode: u8,
) -> Result<u32, RunResult> {
    let dest = decode_a(instr) as usize;
    let src_reg = decode_b(instr) as usize;
    let index = decode_c(instr) as usize;
    let src_term = x_regs[src_reg];

    let elem = match opcode {
        op::GET_TUPLE_ELEM => proc.read_term_tuple_element(mem, src_term, index),
        op::GET_VEC_ELEM => proc.read_term_vector_element(mem, src_term, index),
        _ => None,
    };

    x_regs[dest] = elem.ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
    Ok(1)
}

/// Maximum number of elements for tuple slice.
const MAX_SLICE_ELEMENTS: usize = 64;

/// Execute a tuple slice instruction.
///
/// Creates a new tuple containing elements from index C to end of source tuple.
pub fn execute_tuple_slice<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    instr: u32,
) -> Result<u32, RunResult> {
    let dest = decode_a(instr) as usize;
    let src_reg = decode_b(instr) as usize;
    let start_index = decode_c(instr) as usize;
    let src_term = x_regs[src_reg];

    let len = proc
        .read_term_tuple_len(mem, src_term)
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

    // Calculate slice length
    let slice_len = len.saturating_sub(start_index);
    let slice_len = slice_len.min(MAX_SLICE_ELEMENTS);

    if slice_len == 0 {
        // Empty tuple
        let empty = proc
            .alloc_term_tuple(mem, &[])
            .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
        x_regs[dest] = empty;
        return Ok(1);
    }

    // Collect elements from start_index to end
    let mut elements = [Term::NIL; MAX_SLICE_ELEMENTS];
    for (i, elem) in elements.iter_mut().enumerate().take(slice_len) {
        *elem = proc
            .read_term_tuple_element(mem, src_term, start_index + i)
            .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;
    }

    // Allocate new tuple using Term-based allocation
    let new_tuple = proc
        .alloc_term_tuple(mem, &elements[..slice_len])
        .ok_or(RunResult::Error(RuntimeError::OutOfMemory))?;

    x_regs[dest] = new_tuple;
    Ok(1)
}

/// Execute a pattern matching instruction.
///
/// Handles type tests, structure tests, element extraction, equality, and control flow.
pub fn execute<M: MemorySpace>(
    x_regs: &mut XRegs,
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
        | op::IS_STRING => Ok(execute_type_test(x_regs, proc, mem, instr, opcode)),

        // Structure test instructions
        op::TEST_ARITY | op::TEST_VEC_LEN | op::TEST_ARITY_GE => {
            Ok(execute_structure_test(x_regs, proc, mem, instr, opcode))
        }

        // Element extraction instructions
        op::GET_TUPLE_ELEM | op::GET_VEC_ELEM => {
            execute_element_extraction(x_regs, proc, mem, instr, opcode)
        }

        // Equality test - Term supports direct comparison via Eq
        op::IS_EQ => {
            let reg_a = decode_a(instr) as usize;
            let reg_b = decode_b(instr) as usize;
            let fail_label = decode_c(instr) as usize;
            if x_regs[reg_a] != x_regs[reg_b] {
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
            let term = x_regs[reg];
            // Falsy: nil or false - Term provides these checks directly
            if !term.is_truthy() {
                proc.ip = target;
            }
            Ok(1)
        }

        // Badmatch error - use Term directly in the error
        op::BADMATCH => {
            let reg = decode_a(instr) as usize;
            let term = x_regs[reg];
            Err(RunResult::Error(RuntimeError::Badmatch { value: term }))
        }

        _ => Err(RunResult::Error(RuntimeError::InvalidOpcode(opcode))),
    }
}
