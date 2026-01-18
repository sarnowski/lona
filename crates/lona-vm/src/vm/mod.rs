// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Bytecode virtual machine for Lona.
//!
//! The VM executes compiled bytecode chunks. It is stateless - all execution
//! state lives in the `Process` struct, which owns registers, heap, and IP.
//!
//! The VM uses an explicit call stack for function calls instead of Rust recursion,
//! enabling cooperative scheduling via reduction counting. Long-running computations
//! can yield after exhausting their reduction budget and resume later.
//!
//! See `docs/architecture/virtual-machine.md` for the full specification.

#[cfg(test)]
mod vm_test;

use crate::Vaddr;
use crate::bytecode::{
    Chunk, decode_a, decode_b, decode_bx, decode_c, decode_opcode, decode_sbx, op,
};
use crate::intrinsics::{
    self, CoreCollectionError, IntrinsicError, core_get, core_nth, intrinsic_cost,
};
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::value::{HeapCompiledFn, Value};

/// Maximum number of elements in a tuple literal.
const MAX_TUPLE_ELEMENTS: usize = 64;

/// Maximum number of key-value pairs in a map literal.
const MAX_MAP_PAIRS: usize = 64;

/// Build a tuple from registers and store in target register.
fn build_tuple<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    target: usize,
    start_reg: usize,
    count: usize,
) -> Result<(), RuntimeError> {
    let elem_count = count.min(MAX_TUPLE_ELEMENTS);
    let mut elements = [Value::Nil; MAX_TUPLE_ELEMENTS];
    elements[..elem_count].copy_from_slice(&proc.x_regs[start_reg..start_reg + elem_count]);

    let tuple = proc
        .alloc_tuple(mem, &elements[..elem_count])
        .ok_or(RuntimeError::OutOfMemory)?;

    proc.x_regs[target] = tuple;
    Ok(())
}

/// Build a vector from registers and store in target register.
fn build_vector<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    target: usize,
    start_reg: usize,
    count: usize,
) -> Result<(), RuntimeError> {
    let elem_count = count.min(MAX_TUPLE_ELEMENTS);
    let mut elements = [Value::Nil; MAX_TUPLE_ELEMENTS];
    elements[..elem_count].copy_from_slice(&proc.x_regs[start_reg..start_reg + elem_count]);

    let vector = proc
        .alloc_vector(mem, &elements[..elem_count])
        .ok_or(RuntimeError::OutOfMemory)?;

    proc.x_regs[target] = vector;
    Ok(())
}

/// Prepare a user-defined function call (validate arity, build chunk).
///
/// This does NOT push a call frame - the caller must do that.
/// Arguments should already be in X1..X(argc).
///
/// For variadic functions, extra arguments are collected into a tuple
/// and placed in the rest parameter register (X(arity+1)).
fn prepare_user_fn<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    fn_addr: Vaddr,
    argc: u8,
) -> Result<Chunk, RuntimeError> {
    // Read function header
    let header: HeapCompiledFn = mem.read(fn_addr);

    // Check arity
    if header.variadic {
        // Variadic: must have at least `arity` args
        if argc < header.arity {
            return Err(RuntimeError::ArityMismatch {
                expected: header.arity,
                got: argc,
                variadic: true,
            });
        }

        // Collect extra args into a tuple for the rest parameter
        let rest_start = header.arity as usize + 1;
        let rest_count = (argc - header.arity) as usize;
        let mut rest_elements = [Value::Nil; MAX_TUPLE_ELEMENTS];
        let rest_count = rest_count.min(MAX_TUPLE_ELEMENTS);
        rest_elements[..rest_count]
            .copy_from_slice(&proc.x_regs[rest_start..rest_start + rest_count]);

        let rest_tuple = proc
            .alloc_tuple(mem, &rest_elements[..rest_count])
            .ok_or(RuntimeError::OutOfMemory)?;

        // Place rest tuple in X(arity+1)
        proc.x_regs[header.arity as usize + 1] = rest_tuple;
    } else {
        // Non-variadic: must have exactly `arity` args
        if argc != header.arity {
            return Err(RuntimeError::ArityMismatch {
                expected: header.arity,
                got: argc,
                variadic: false,
            });
        }
    }

    // Build chunk from function bytecode and constants
    let mut chunk = Chunk::new();

    // Read bytecode
    let code_addr = fn_addr.add(HeapCompiledFn::bytecode_offset() as u64);
    for i in 0..header.code_len as usize {
        let instr_addr = code_addr.add((i * core::mem::size_of::<u32>()) as u64);
        let instr: u32 = mem.read(instr_addr);
        chunk.emit(instr);
    }

    // Read constants
    let constants_addr =
        fn_addr.add(HeapCompiledFn::constants_offset(header.code_len as usize) as u64);
    for i in 0..header.constants_len as usize {
        let const_addr = constants_addr.add((i * core::mem::size_of::<Value>()) as u64);
        let constant: Value = mem.read(const_addr);
        chunk.add_constant(constant);
    }

    Ok(chunk)
}

/// Prepare a closure call (load captures, validate arity, build chunk).
///
/// This does NOT push a call frame - the caller must do that.
/// Returns the function address and chunk for the closure's underlying function.
fn prepare_closure<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    closure_addr: Vaddr,
    argc: u8,
) -> Result<(Vaddr, Chunk), RuntimeError> {
    let closure: crate::value::HeapClosure = mem.read(closure_addr);

    // Load captured values into registers after regular args
    let captures_base = argc as usize + 1;
    let captures_offset = closure_addr.add(crate::value::HeapClosure::captures_offset() as u64);

    for i in 0..closure.captures_len as usize {
        let capture_addr = captures_offset.add((i * core::mem::size_of::<Value>()) as u64);
        proc.x_regs[captures_base + i] = mem.read(capture_addr);
    }

    let chunk = prepare_user_fn(proc, mem, closure.function, argc)?;
    Ok((closure.function, chunk))
}

/// Maximum number of captured variables in a closure.
const MAX_CLOSURE_CAPTURES: usize = 16;

/// Arity description for callable data structures (keywords, maps, tuples).
const CALLABLE_DATA_ARITY: &str = "1-2";

/// Valid arity range for callable data structures.
const CALLABLE_ARITY_RANGE: core::ops::RangeInclusive<u8> = 1..=2;

/// Check arity for callable data structures (keywords, maps, tuples).
fn check_callable_arity(argc: u8) -> Result<(), RuntimeError> {
    if !CALLABLE_ARITY_RANGE.contains(&argc) {
        return Err(RuntimeError::CallableArityError {
            expected: CALLABLE_DATA_ARITY,
            got: argc,
        });
    }
    Ok(())
}

/// Call a keyword as a function: `(:key map [default])`.
fn call_keyword<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    key: Value,
    argc: u8,
) -> Result<Value, RuntimeError> {
    check_callable_arity(argc)?;

    let map_val = proc.x_regs[1];
    let default = if argc >= 2 {
        proc.x_regs[2]
    } else {
        Value::Nil
    };

    core_get(proc, mem, map_val, key, default).map_err(|e| match e {
        CoreCollectionError::NotAMap => RuntimeError::CallableTypeError {
            callable: "keyword",
            arg: 0,
            expected: "map",
        },
        _ => RuntimeError::OutOfMemory,
    })
}

/// Call a map as a function: `(map key [default])`.
fn call_map<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    map_val: Value,
    argc: u8,
) -> Result<Value, RuntimeError> {
    check_callable_arity(argc)?;

    let key = proc.x_regs[1];
    let default = if argc >= 2 {
        proc.x_regs[2]
    } else {
        Value::Nil
    };

    // Note: NotAMap shouldn't happen here since we already know it's a map,
    // but we handle it for completeness.
    core_get(proc, mem, map_val, key, default).map_err(|e| match e {
        CoreCollectionError::NotAMap => RuntimeError::CallableTypeError {
            callable: "map",
            arg: 0,
            expected: "map",
        },
        _ => RuntimeError::OutOfMemory,
    })
}

/// Call a tuple as a function: `(tuple idx [default])`.
fn call_tuple<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    tuple_val: Value,
    argc: u8,
) -> Result<Value, RuntimeError> {
    check_callable_arity(argc)?;

    let idx = proc.x_regs[1];
    let default = if argc >= 2 {
        Some(proc.x_regs[2])
    } else {
        None
    };

    // Note: NotATuple shouldn't happen here since we already know it's a tuple,
    // but we handle it for completeness.
    core_nth(proc, mem, tuple_val, idx, default).map_err(|e| match e {
        CoreCollectionError::NotATuple => RuntimeError::CallableTypeError {
            callable: "tuple",
            arg: 0,
            expected: "tuple",
        },
        CoreCollectionError::InvalidIndex => RuntimeError::CallableTypeError {
            callable: "tuple",
            arg: 0,
            expected: "integer index",
        },
        CoreCollectionError::IndexOutOfBounds { index, len } => {
            RuntimeError::Intrinsic(IntrinsicError::IndexOutOfBounds { index, len })
        }
        _ => RuntimeError::OutOfMemory,
    })
}

/// Handle CALL instruction without recursion.
///
/// For native functions, keywords, maps, tuples: execute immediately and return cost.
/// For compiled functions and closures: push call frame, set up callee's chunk/IP.
///
/// Handles:
/// - `NativeFn(id)`: Native/intrinsic function call
/// - `CompiledFn(addr)`: User-defined function call (non-recursive)
/// - `Closure(addr)`: Closure call (function + captures, non-recursive)
/// - `Keyword(_)`: `(:key map [default])` → `(get map :key [default])`
/// - `Map(_)`: `(map key [default])` → `(get map key [default])`
/// - `Tuple(_)`: `(tuple idx [default])` → `(nth tuple idx [default])`
fn handle_call<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    fn_reg: usize,
    argc: u8,
) -> Result<u32, RuntimeError> {
    let fn_val = proc.x_regs[fn_reg];

    match fn_val {
        Value::NativeFn(id) => {
            intrinsics::call_intrinsic(id as u8, argc, proc, mem, realm)?;
            Ok(intrinsic_cost(id as u8))
        }

        Value::CompiledFn(fn_addr) => {
            let callee_chunk = prepare_user_fn(proc, mem, fn_addr, argc)?;
            proc.push_call_frame(fn_addr)
                .map_err(|_| RuntimeError::StackOverflow)?;
            proc.chunk = Some(callee_chunk);
            proc.ip = 0;
            Ok(1)
        }

        Value::Closure(closure_addr) => {
            let (fn_addr, callee_chunk) = prepare_closure(proc, mem, closure_addr, argc)?;
            proc.push_call_frame(fn_addr)
                .map_err(|_| RuntimeError::StackOverflow)?;
            proc.chunk = Some(callee_chunk);
            proc.ip = 0;
            Ok(1)
        }

        Value::Keyword(_) => {
            proc.x_regs[0] = call_keyword(proc, mem, fn_val, argc)?;
            Ok(2)
        }

        Value::Map(_) => {
            proc.x_regs[0] = call_map(proc, mem, fn_val, argc)?;
            Ok(2)
        }

        Value::Tuple(_) => {
            proc.x_regs[0] = call_tuple(proc, mem, fn_val, argc)?;
            Ok(2)
        }

        _ => Err(RuntimeError::NotCallable {
            type_name: fn_val.type_name(),
        }),
    }
}

/// Build a closure from a compiled function and captures tuple.
fn build_closure<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    target: usize,
    fn_reg: usize,
    captures_reg: usize,
) -> Result<(), RuntimeError> {
    // Get function address
    let fn_val = proc.x_regs[fn_reg];
    let Value::CompiledFn(fn_addr) = fn_val else {
        return Err(RuntimeError::NotCallable {
            type_name: fn_val.type_name(),
        });
    };

    // Get captures tuple
    let captures_val = proc.x_regs[captures_reg];
    let captures_len = if captures_val.is_nil() {
        0
    } else {
        proc.read_tuple_len(mem, captures_val)
            .ok_or(RuntimeError::OutOfMemory)?
    };

    // Read capture values from tuple
    let mut captures = [Value::Nil; MAX_CLOSURE_CAPTURES];
    for (i, capture) in captures
        .iter_mut()
        .enumerate()
        .take(captures_len.min(MAX_CLOSURE_CAPTURES))
    {
        *capture = proc
            .read_tuple_element(mem, captures_val, i)
            .ok_or(RuntimeError::OutOfMemory)?;
    }

    // Allocate closure
    let closure = proc
        .alloc_closure(mem, fn_addr, &captures[..captures_len])
        .ok_or(RuntimeError::OutOfMemory)?;

    proc.x_regs[target] = closure;
    Ok(())
}

/// Build a map from key-value pairs in registers and store in target register.
fn build_map<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    target: usize,
    start_reg: usize,
    pair_count: usize,
) -> Result<(), RuntimeError> {
    let pair_count = pair_count.min(MAX_MAP_PAIRS);

    // Build entries list from back to front
    let mut entries = Value::Nil;
    for i in (0..pair_count).rev() {
        let key_reg = start_reg + i * 2;
        let val_reg = start_reg + i * 2 + 1;

        // Build [key value] tuple
        let kv_elements = [proc.x_regs[key_reg], proc.x_regs[val_reg]];
        let kv_tuple = proc
            .alloc_tuple(mem, &kv_elements)
            .ok_or(RuntimeError::OutOfMemory)?;

        // Prepend to entries list
        entries = proc
            .alloc_pair(mem, kv_tuple, entries)
            .ok_or(RuntimeError::OutOfMemory)?;
    }

    let map = proc
        .alloc_map(mem, entries)
        .ok_or(RuntimeError::OutOfMemory)?;

    proc.x_regs[target] = map;
    Ok(())
}

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
    /// Value is not callable.
    NotCallable {
        /// Name of the type that was used in function position.
        type_name: &'static str,
    },
    /// Wrong number of arguments in function call.
    ArityMismatch {
        /// Number of parameters the function expects.
        expected: u8,
        /// Number of arguments actually provided.
        got: u8,
        /// Whether the function accepts variadic arguments.
        variadic: bool,
    },
    /// Wrong number of arguments when calling a data structure.
    ///
    /// Used for callable keywords, maps, and tuples which accept 1-2 args.
    CallableArityError {
        /// Description of expected arity (e.g., "1-2").
        expected: &'static str,
        /// Number of arguments actually provided.
        got: u8,
    },
    /// Type error when calling a data structure (keyword, map, or tuple).
    ///
    /// Used when arguments to callable data structures have wrong types.
    CallableTypeError {
        /// What was being called (e.g., "keyword", "map", "tuple").
        callable: &'static str,
        /// Which argument had the wrong type (0-indexed).
        arg: u8,
        /// What type was expected.
        expected: &'static str,
    },
    /// Call stack overflow (too many nested calls).
    StackOverflow,
}

impl From<IntrinsicError> for RuntimeError {
    fn from(e: IntrinsicError) -> Self {
        Self::Intrinsic(e)
    }
}

/// Result of running a process for one time slice.
///
/// Used by the scheduler to determine what to do next with a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunResult {
    /// Process completed execution normally. Contains return value.
    Completed(Value),

    /// Process yielded due to exhausted reduction budget.
    /// The process can be resumed later by calling `Vm::run` again.
    Yielded,

    /// Process encountered a runtime error.
    Error(RuntimeError),
}

impl RunResult {
    /// Returns true if execution completed (success or error).
    ///
    /// Terminal results mean the process should not be resumed.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed(_) | Self::Error(_))
    }

    /// Returns true if execution can be resumed.
    #[must_use]
    pub const fn is_yielded(&self) -> bool {
        matches!(self, Self::Yielded)
    }
}

/// Execute a single instruction and return its reduction cost.
///
/// Returns `Ok(cost)` to continue execution, or `Err(result)` to terminate.
fn execute_instruction<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    instr: u32,
    opcode: u8,
    constant_value: Option<Value>,
) -> Result<u32, RunResult> {
    match opcode {
        op::LOADNIL => {
            proc.x_regs[decode_a(instr) as usize] = Value::Nil;
            Ok(1)
        }
        op::LOADBOOL => {
            proc.x_regs[decode_a(instr) as usize] = Value::bool(decode_bx(instr) != 0);
            Ok(1)
        }
        op::LOADINT => {
            proc.x_regs[decode_a(instr) as usize] = Value::int(i64::from(decode_sbx(instr)));
            Ok(1)
        }
        op::LOADK => {
            if let Some(value) = constant_value {
                proc.x_regs[decode_a(instr) as usize] = value;
            }
            Ok(1)
        }
        op::MOVE => {
            proc.x_regs[decode_a(instr) as usize] = proc.x_regs[decode_b(instr) as usize];
            Ok(1)
        }
        op::INTRINSIC => {
            let id = decode_a(instr);
            intrinsics::call_intrinsic(id, decode_b(instr) as u8, proc, mem, realm)
                .map_err(|e| RunResult::Error(e.into()))?;
            Ok(intrinsic_cost(id))
        }
        op::CALL => handle_call(
            proc,
            mem,
            realm,
            decode_a(instr) as usize,
            decode_b(instr) as u8,
        )
        .map_err(RunResult::Error),
        op::RETURN => {
            if proc.pop_call_frame() {
                Ok(1) // Continue in caller's context
            } else {
                Err(RunResult::Completed(proc.x_regs[0])) // Top-level return
            }
        }
        op::HALT => Err(RunResult::Completed(proc.x_regs[0])),
        op::BUILD_TUPLE => {
            let c = decode_c(instr) as usize;
            build_tuple(
                proc,
                mem,
                decode_a(instr) as usize,
                decode_b(instr) as usize,
                c,
            )
            .map_err(RunResult::Error)?;
            Ok(1 + (c / 8) as u32)
        }
        op::BUILD_VECTOR => {
            let c = decode_c(instr) as usize;
            build_vector(
                proc,
                mem,
                decode_a(instr) as usize,
                decode_b(instr) as usize,
                c,
            )
            .map_err(RunResult::Error)?;
            Ok(1 + (c / 8) as u32)
        }
        op::BUILD_MAP => {
            let c = decode_c(instr) as usize;
            build_map(
                proc,
                mem,
                decode_a(instr) as usize,
                decode_b(instr) as usize,
                c,
            )
            .map_err(RunResult::Error)?;
            Ok(1 + (c / 4) as u32)
        }
        op::BUILD_CLOSURE => {
            build_closure(
                proc,
                mem,
                decode_a(instr) as usize,
                decode_b(instr) as usize,
                decode_c(instr) as usize,
            )
            .map_err(RunResult::Error)?;
            Ok(2)
        }
        _ => Err(RunResult::Error(RuntimeError::InvalidOpcode(opcode))),
    }
}

/// Stateless bytecode virtual machine.
///
/// The VM is a namespace for execution functions. All state lives in `Process`.
pub struct Vm;

impl Vm {
    /// Run bytecode until completion, yield, or error.
    ///
    /// The process must have a chunk set via `Process::set_chunk`.
    /// Execution state (ip, `x_regs`, call stack) is read from and written to the process.
    ///
    /// This implementation is non-recursive: function calls use the Process's call stack
    /// instead of Rust stack recursion, enabling the VM to yield and resume at any call depth.
    ///
    /// Returns:
    /// - `RunResult::Completed(value)` when execution finishes (HALT or top-level RETURN)
    /// - `RunResult::Yielded` when the reduction budget is exhausted (can be resumed)
    /// - `RunResult::Error(e)` when a runtime error occurs
    pub fn run<M: MemorySpace>(proc: &mut Process, mem: &mut M, realm: &mut Realm) -> RunResult {
        loop {
            // Check reduction budget
            if proc.should_yield() {
                return RunResult::Yielded;
            }

            // Get current chunk
            let Some(chunk) = proc.chunk.as_ref() else {
                return RunResult::Error(RuntimeError::NoCode);
            };

            // Bounds check and fetch
            if proc.ip >= chunk.code.len() {
                return RunResult::Error(RuntimeError::IpOutOfBounds);
            }
            let instr = chunk.code[proc.ip];
            proc.ip += 1;

            // Decode and pre-fetch constant for LOADK
            let opcode = decode_opcode(instr);
            let constant_value = if opcode == op::LOADK {
                let bx = decode_bx(instr);
                match chunk.constants.get(bx as usize).copied() {
                    Some(v) => Some(v),
                    None => return RunResult::Error(RuntimeError::ConstantOutOfBounds(bx)),
                }
            } else {
                None
            };

            // Execute instruction
            match execute_instruction(proc, mem, realm, instr, opcode, constant_value) {
                Ok(cost) => {
                    proc.consume_reductions(cost);
                }
                Err(result) => return result,
            }
        }
    }
}

/// Convenience function to execute a process's bytecode to completion.
///
/// The process must have a chunk set via `Process::set_chunk`.
/// Automatically handles yielding by resetting the reduction budget and resuming.
///
/// Use this when you want to run a computation to completion without worrying
/// about cooperative scheduling. For proper multi-process scheduling, use
/// `Vm::run` directly and handle `RunResult::Yielded` appropriately.
///
/// # Errors
///
/// Returns an error if execution fails.
pub fn execute<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
) -> Result<Value, RuntimeError> {
    // Initialize reduction budget
    proc.reset_reductions();

    loop {
        match Vm::run(proc, mem, realm) {
            RunResult::Completed(value) => return Ok(value),
            RunResult::Yielded => {
                // Single-threaded execution: just continue with fresh budget
                proc.reset_reductions();
            }
            RunResult::Error(e) => return Err(e),
        }
    }
}
