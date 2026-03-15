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

mod pattern;
mod special_intrinsics;

use crate::Vaddr;
use crate::bytecode::{decode_a, decode_b, decode_bx, decode_c, decode_opcode, decode_sbx, op};
use crate::gc;
use crate::intrinsics::{self, IntrinsicError, XRegs, intrinsic_cost};
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::scheduler::{Scheduler, Worker};
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::{HeapClosure, HeapFun};
use crate::term::tag::object;

/// Maximum number of elements in a tuple literal.
const MAX_TUPLE_ELEMENTS: usize = 64;

/// Maximum number of key-value pairs in a map literal.
const MAX_MAP_PAIRS: usize = 64;

/// Build a tuple from registers and store in target register.
fn build_tuple<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    target: usize,
    start_reg: usize,
    count: usize,
) -> Result<(), RuntimeError> {
    let elem_count = count.min(MAX_TUPLE_ELEMENTS);
    let mut elements = [Term::NIL; MAX_TUPLE_ELEMENTS];
    elements[..elem_count].copy_from_slice(&x_regs[start_reg..start_reg + elem_count]);

    let tuple = proc
        .alloc_term_tuple(mem, &elements[..elem_count])
        .ok_or(RuntimeError::OutOfMemory)?;

    x_regs[target] = tuple;
    Ok(())
}

/// Build a vector from registers and store in target register.
fn build_vector<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    target: usize,
    start_reg: usize,
    count: usize,
) -> Result<(), RuntimeError> {
    let elem_count = count.min(MAX_TUPLE_ELEMENTS);
    let mut elements = [Term::NIL; MAX_TUPLE_ELEMENTS];
    elements[..elem_count].copy_from_slice(&x_regs[start_reg..start_reg + elem_count]);

    let vector = proc
        .alloc_term_vector(mem, &elements[..elem_count])
        .ok_or(RuntimeError::OutOfMemory)?;

    x_regs[target] = vector;
    Ok(())
}

/// Validate arity and set up variadic args for a user-defined function call.
///
/// This does NOT push a call frame — the caller must do that.
/// Arguments should already be in X1..X(argc).
///
/// For variadic functions, extra arguments are collected into a tuple
/// and placed in the rest parameter register (X(arity+1)).
fn prepare_user_fn<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    fn_addr: Vaddr,
    argc: u8,
) -> Result<(), RuntimeError> {
    // Read function header
    let header: HeapFun = mem.read(fn_addr);
    let fn_arity = header.fn_arity;
    let is_variadic = header.variadic != 0;

    // Check arity
    if is_variadic {
        // Variadic: must have at least `fn_arity` args
        if argc < fn_arity {
            return Err(RuntimeError::ArityMismatch {
                expected: fn_arity,
                got: argc,
                variadic: true,
            });
        }

        // Collect extra args into a tuple for the rest parameter
        let rest_start = fn_arity as usize + 1;
        let rest_count = (argc - fn_arity) as usize;
        let mut rest_elements = [Term::NIL; MAX_TUPLE_ELEMENTS];
        let rest_count = rest_count.min(MAX_TUPLE_ELEMENTS);
        rest_elements[..rest_count].copy_from_slice(&x_regs[rest_start..rest_start + rest_count]);

        let rest_tuple = proc
            .alloc_term_tuple(mem, &rest_elements[..rest_count])
            .ok_or(RuntimeError::OutOfMemory)?;

        // Place rest tuple in X(fn_arity+1)
        x_regs[fn_arity as usize + 1] = rest_tuple;
    } else {
        // Non-variadic: must have exactly `fn_arity` args
        if argc != fn_arity {
            return Err(RuntimeError::ArityMismatch {
                expected: fn_arity,
                got: argc,
                variadic: false,
            });
        }
    }

    Ok(())
}

/// Prepare a closure call (load captures, validate arity).
///
/// This does NOT push a call frame — the caller must do that.
/// Returns the function address for the closure's underlying function.
fn prepare_closure<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    closure_addr: Vaddr,
    argc: u8,
) -> Result<Vaddr, RuntimeError> {
    let closure: HeapClosure = mem.read(closure_addr);
    let capture_count = closure.capture_count();

    // Load captured values (Terms) into registers after regular args
    let captures_base = argc as usize + 1;
    let captures_offset = closure_addr.add(HeapClosure::PREFIX_SIZE as u64);

    for i in 0..capture_count {
        let capture_addr = captures_offset.add((i * 8) as u64);
        x_regs[captures_base + i] = mem.read(capture_addr);
    }

    // Extract function address from the closure's function Term
    let fn_addr = closure.function.to_vaddr();
    prepare_user_fn(x_regs, proc, mem, fn_addr, argc)?;
    Ok(fn_addr)
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
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    key: Term,
    argc: u8,
) -> Result<Term, RuntimeError> {
    check_callable_arity(argc)?;

    let map_term = x_regs[1];
    let default = if argc >= 2 { x_regs[2] } else { Term::NIL };

    // Get map entries
    let entries =
        proc.read_term_map_entries(mem, map_term)
            .ok_or(RuntimeError::CallableTypeError {
                callable: "keyword",
                arg: 0,
                expected: "map",
            })?;

    // Search for key in entries
    let result = map_get_term(proc, mem, entries, key, default);
    Ok(result)
}

/// Call a map as a function: `(map key [default])`.
fn call_map<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    map_term: Term,
    argc: u8,
) -> Result<Term, RuntimeError> {
    check_callable_arity(argc)?;

    let key = x_regs[1];
    let default = if argc >= 2 { x_regs[2] } else { Term::NIL };

    // Get map entries
    let entries =
        proc.read_term_map_entries(mem, map_term)
            .ok_or(RuntimeError::CallableTypeError {
                callable: "map",
                arg: 0,
                expected: "map",
            })?;

    let result = map_get_term(proc, mem, entries, key, default);
    Ok(result)
}

/// Search for a key in map entries and return value or default.
fn map_get_term<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    entries: Term,
    key: Term,
    default: Term,
) -> Term {
    let mut current = entries;
    while current.is_list() {
        if let Some((head, tail)) = proc.read_term_pair(mem, current) {
            // Each entry is a [key value] tuple
            if let Some(entry_key) = proc.read_term_tuple_element(mem, head, 0) {
                if terms_equal(proc, mem, entry_key, key) {
                    return proc
                        .read_term_tuple_element(mem, head, 1)
                        .unwrap_or(default);
                }
            }
            current = tail;
        } else {
            break;
        }
    }
    default
}

/// Compare two terms for equality (structural comparison).
///
/// Uses the intrinsics module's `terms_equal` for full structural comparison
/// with depth limiting. This ensures consistent equality semantics everywhere.
#[inline]
fn terms_equal<M: MemorySpace>(proc: &Process, mem: &M, a: Term, b: Term) -> bool {
    intrinsics::terms_equal(a, b, proc, mem)
}

/// Call a tuple as a function: `(tuple idx [default])`.
fn call_tuple<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    tuple_term: Term,
    argc: u8,
) -> Result<Term, RuntimeError> {
    check_callable_arity(argc)?;

    let idx_term = x_regs[1];
    let default = if argc >= 2 { Some(x_regs[2]) } else { None };

    // Index must be a small integer
    let idx = idx_term
        .as_small_int()
        .ok_or(RuntimeError::CallableTypeError {
            callable: "tuple",
            arg: 0,
            expected: "integer index",
        })?;

    if idx < 0 {
        return Ok(default.unwrap_or(Term::NIL));
    }

    let len = proc
        .read_term_tuple_len(mem, tuple_term)
        .ok_or(RuntimeError::CallableTypeError {
            callable: "tuple",
            arg: 0,
            expected: "tuple",
        })?;

    // Check bounds: negative indices or indices >= len are out of bounds
    let idx_usize = usize::try_from(idx).ok().filter(|&i| i < len);

    let Some(valid_idx) = idx_usize else {
        return default.map_or_else(
            || {
                Err(RuntimeError::Intrinsic(IntrinsicError::IndexOutOfBounds {
                    index: idx,
                    len,
                }))
            },
            Ok,
        );
    };

    proc.read_term_tuple_element(mem, tuple_term, valid_idx)
        .ok_or(RuntimeError::OutOfMemory)
}

/// Get the type name for a Term (for error messages).
fn term_type_name<M: MemorySpace>(mem: &M, term: Term) -> &'static str {
    if term.is_nil() {
        return "nil";
    }
    if term.is_immediate() {
        if term.is_small_int() {
            return "integer";
        }
        if term.is_boolean() {
            return "boolean";
        }
        if term.is_native_fn() {
            return "native-fn";
        }
        if term.is_symbol() {
            return "symbol";
        }
        if term.is_keyword() {
            return "keyword";
        }
        return "immediate";
    }
    if term.is_list() {
        return "list";
    }
    if term.is_boxed() {
        let header: Header = mem.read(term.to_vaddr());
        return match header.object_tag() {
            object::STRING => "string",
            object::TUPLE => "tuple",
            object::VECTOR => "vector",
            object::MAP => "map",
            object::FUN => "compiled-fn",
            object::CLOSURE => "closure",
            object::VAR => "var",
            object::NAMESPACE => "namespace",
            object::FLOAT => "float",
            object::PID => "pid",
            _ => "unknown",
        };
    }
    "unknown"
}

/// Handle CALL instruction without recursion.
///
/// For native functions, keywords, maps, tuples: execute immediately and return cost.
/// For compiled functions and closures: push call frame, set up callee's chunk/IP.
fn handle_call<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    fn_reg: usize,
    argc: u8,
) -> Result<u32, RuntimeError> {
    let fn_term = x_regs[fn_reg];

    // Check for native function (immediate)
    if let Some(id) = fn_term.as_native_fn_id() {
        let id_u8 = id as u8;
        intrinsics::call_intrinsic(id_u8, argc, x_regs, proc, mem, realm)?;
        return Ok(intrinsic_cost(id_u8));
    }

    // Check for keyword (immediate) - keywords are callable
    if fn_term.is_keyword() {
        x_regs[0] = call_keyword(x_regs, proc, mem, fn_term, argc)?;
        return Ok(2);
    }

    // Check for boxed callable types
    if fn_term.is_boxed() {
        let fn_addr = fn_term.to_vaddr();
        let header: Header = mem.read(fn_addr);

        match header.object_tag() {
            object::FUN => {
                prepare_user_fn(x_regs, proc, mem, fn_addr, argc)?;

                // Create stack frame for callee
                let return_ip = proc.ip;
                let caller_chunk_addr = proc.chunk_addr.unwrap_or(Vaddr::new(0));
                proc.allocate_frame(mem, return_ip, caller_chunk_addr)
                    .map_err(|_| RuntimeError::StackOverflow)?;

                // Set up callee's execution context (direct read from HeapFun)
                proc.chunk_addr = Some(fn_addr);
                proc.ip = 0;
                return Ok(1);
            }

            object::CLOSURE => {
                let fn_addr = prepare_closure(x_regs, proc, mem, fn_addr, argc)?;

                // Create stack frame for callee
                let return_ip = proc.ip;
                let caller_chunk_addr = proc.chunk_addr.unwrap_or(Vaddr::new(0));
                proc.allocate_frame(mem, return_ip, caller_chunk_addr)
                    .map_err(|_| RuntimeError::StackOverflow)?;

                // Set up callee's execution context (direct read from HeapFun)
                proc.chunk_addr = Some(fn_addr);
                proc.ip = 0;
                return Ok(1);
            }

            object::MAP => {
                x_regs[0] = call_map(x_regs, proc, mem, fn_term, argc)?;
                return Ok(2);
            }

            object::TUPLE => {
                x_regs[0] = call_tuple(x_regs, proc, mem, fn_term, argc)?;
                return Ok(2);
            }

            _ => {}
        }
    }

    Err(RuntimeError::NotCallable {
        type_name: term_type_name(mem, fn_term),
    })
}

/// Build a closure from a compiled function and captures tuple.
fn build_closure<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    target: usize,
    fn_reg: usize,
    captures_reg: usize,
) -> Result<(), RuntimeError> {
    // Get function Term - verify it's a compiled function
    let fn_term = x_regs[fn_reg];
    if !fn_term.is_boxed() {
        return Err(RuntimeError::NotCallable {
            type_name: term_type_name(mem, fn_term),
        });
    }

    let fn_addr = fn_term.to_vaddr();
    let header: Header = mem.read(fn_addr);
    if header.object_tag() != object::FUN {
        return Err(RuntimeError::NotCallable {
            type_name: term_type_name(mem, fn_term),
        });
    }

    // Get captures tuple - use Term-based methods
    let captures_term = x_regs[captures_reg];
    let captures_len = if captures_term.is_nil() {
        0
    } else {
        proc.read_term_tuple_len(mem, captures_term)
            .ok_or(RuntimeError::OutOfMemory)?
    };

    // Read capture values from tuple as Terms
    let mut captures = [Term::NIL; MAX_CLOSURE_CAPTURES];
    for (i, capture) in captures
        .iter_mut()
        .enumerate()
        .take(captures_len.min(MAX_CLOSURE_CAPTURES))
    {
        *capture = proc
            .read_term_tuple_element(mem, captures_term, i)
            .ok_or(RuntimeError::OutOfMemory)?;
    }

    // Allocate closure using Term-based allocation
    let closure = proc
        .alloc_term_closure(mem, fn_term, &captures[..captures_len])
        .ok_or(RuntimeError::OutOfMemory)?;

    x_regs[target] = closure;
    Ok(())
}

/// Build a map from key-value pairs in registers and store in target register.
fn build_map<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    target: usize,
    start_reg: usize,
    pair_count: usize,
) -> Result<(), RuntimeError> {
    let pair_count = pair_count.min(MAX_MAP_PAIRS);

    // Build entries list from back to front using Term
    let mut entries = Term::NIL;
    for i in (0..pair_count).rev() {
        let key_reg = start_reg + i * 2;
        let val_reg = start_reg + i * 2 + 1;

        // Build [key value] tuple using Term-based allocation
        let kv_elements = [x_regs[key_reg], x_regs[val_reg]];
        let kv_tuple = proc
            .alloc_term_tuple(mem, &kv_elements)
            .ok_or(RuntimeError::OutOfMemory)?;

        // Prepend to entries list using Term-based allocation
        entries = proc
            .alloc_term_pair(mem, kv_tuple, entries)
            .ok_or(RuntimeError::OutOfMemory)?;
    }

    let map = proc
        .alloc_term_map(mem, entries, pair_count)
        .ok_or(RuntimeError::OutOfMemory)?;

    x_regs[target] = map;
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

    /// Y register index out of bounds.
    ///
    /// The Y register index exceeds the number of Y registers allocated
    /// in the current frame.
    YRegisterOutOfBounds {
        /// The index that was accessed.
        index: usize,
        /// Number of Y registers allocated.
        allocated: usize,
    },

    /// Frame Y register count mismatch.
    ///
    /// DEALLOCATE was called with a different count than ALLOCATE.
    FrameMismatch {
        /// Number of Y registers that were allocated.
        allocated: usize,
        /// Number of Y registers DEALLOCATE tried to release.
        deallocate_count: usize,
    },

    /// Pattern match failure - no clause matched the value.
    ///
    /// This causes the process to exit with reason `[:error :badmatch %{:value v}]`.
    Badmatch {
        /// The value that failed to match any pattern.
        value: Term,
    },

    /// Eval compilation failed (syntax error, undefined var, etc.).
    ///
    /// Distinct from `OutOfMemory` so GC retry is not triggered.
    EvalError,

    /// Process table is full — no more processes can be spawned.
    ///
    /// Distinct from `OutOfMemory` so GC retry is not triggered.
    ProcessLimitReached,
}

impl RuntimeError {
    /// Returns true if this error represents an out-of-memory condition,
    /// regardless of whether it originated from the VM or an intrinsic.
    #[must_use]
    pub const fn is_oom(&self) -> bool {
        matches!(
            self,
            Self::OutOfMemory | Self::Intrinsic(IntrinsicError::OutOfMemory)
        )
    }
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
    Completed(Term),

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

/// Handle RETURN instruction: deallocate frame and restore caller context.
fn handle_return(
    x_regs: &XRegs,
    proc: &mut Process,
    mem: &impl MemorySpace,
) -> Result<u32, RunResult> {
    match proc.deallocate_frame(mem) {
        Some((return_ip, chunk_addr)) => {
            // Restore caller's chunk_addr directly — no Vec allocation
            if chunk_addr.is_null() {
                proc.chunk_addr = None;
            } else {
                proc.chunk_addr = Some(chunk_addr);
            }
            proc.ip = return_ip;
            Ok(1)
        }
        None => Err(RunResult::Completed(x_regs[0])),
    }
}

/// Execute a load instruction.
fn execute_load_instruction(
    x_regs: &mut XRegs,
    instr: u32,
    opcode: u8,
    constant_term: Option<Term>,
) -> u32 {
    let a = decode_a(instr) as usize;
    x_regs[a] = match opcode {
        op::LOADNIL => Term::NIL,
        op::LOADBOOL => Term::bool(decode_bx(instr) != 0),
        op::LOADINT => {
            // small_int returns Option, unwrap since compiler ensures value fits
            match Term::small_int(i64::from(decode_sbx(instr))) {
                Some(term) => term,
                None => return 1, // Shouldn't happen for valid bytecode
            }
        }
        op::LOADK => match constant_term {
            Some(term) => term,
            None => return 1,
        },
        op::MOVE => x_regs[decode_b(instr) as usize],
        _ => return 1,
    };
    1
}

/// Execute a collection build instruction.
fn execute_build_instruction<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    instr: u32,
    opcode: u8,
) -> Result<u32, RuntimeError> {
    let a = decode_a(instr) as usize;
    let b = decode_b(instr) as usize;
    let c = decode_c(instr) as usize;

    match opcode {
        op::BUILD_TUPLE => {
            build_tuple(x_regs, proc, mem, a, b, c)?;
            Ok(1 + (c / 8) as u32)
        }
        op::BUILD_VECTOR => {
            build_vector(x_regs, proc, mem, a, b, c)?;
            Ok(1 + (c / 8) as u32)
        }
        op::BUILD_MAP => {
            build_map(x_regs, proc, mem, a, b, c)?;
            Ok(1 + (c / 4) as u32)
        }
        op::BUILD_CLOSURE => {
            build_closure(x_regs, proc, mem, a, b, c)?;
            Ok(2)
        }
        _ => Ok(1),
    }
}

/// Execute a Y register instruction.
///
/// Returns `Ok(cost)` for successful execution, or `Err` for errors.
fn execute_y_register_instruction<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    instr: u32,
    opcode: u8,
) -> Result<u32, RunResult> {
    match opcode {
        op::ALLOCATE => {
            let y_count = decode_a(instr) as usize;
            proc.extend_frame_y_regs(mem, y_count)
                .map_err(|_| RunResult::Error(RuntimeError::StackOverflow))?;
            Ok(1)
        }
        op::ALLOCATE_ZERO => {
            let y_count = decode_a(instr) as usize;
            proc.extend_frame_y_regs_zero(mem, y_count)
                .map_err(|_| RunResult::Error(RuntimeError::StackOverflow))?;
            Ok(1)
        }
        op::DEALLOCATE => {
            let y_count = decode_a(instr) as usize;
            if y_count != proc.current_y_count {
                return Err(RunResult::Error(RuntimeError::FrameMismatch {
                    allocated: proc.current_y_count,
                    deallocate_count: y_count,
                }));
            }
            proc.shrink_frame_y_regs(mem, y_count)
                .map_err(|_| RunResult::Error(RuntimeError::StackOverflow))?;
            Ok(1)
        }
        op::MOVE_XY => {
            let y_idx = decode_a(instr) as usize;
            let x_idx = decode_b(instr) as usize;
            if !proc.set_y(mem, y_idx, x_regs[x_idx]) {
                return Err(RunResult::Error(RuntimeError::YRegisterOutOfBounds {
                    index: y_idx,
                    allocated: proc.current_y_count,
                }));
            }
            Ok(1)
        }
        op::MOVE_YX => {
            let x_idx = decode_a(instr) as usize;
            let y_idx = decode_b(instr) as usize;
            match proc.get_y(mem, y_idx) {
                Some(term) => {
                    x_regs[x_idx] = term;
                    Ok(1)
                }
                None => Err(RunResult::Error(RuntimeError::YRegisterOutOfBounds {
                    index: y_idx,
                    allocated: proc.current_y_count,
                })),
            }
        }
        _ => Err(RunResult::Error(RuntimeError::InvalidOpcode(opcode))),
    }
}

/// Execute a single instruction and return its reduction cost.
///
/// Returns `Ok(cost)` to continue execution, or `Err(result)` to terminate.
fn execute_instruction<M: MemorySpace>(
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    instr: u32,
    opcode: u8,
    constant_term: Option<Term>,
) -> Result<u32, RunResult> {
    match opcode {
        // Load and move instructions
        op::LOADNIL | op::LOADBOOL | op::LOADINT | op::LOADK | op::MOVE => Ok(
            execute_load_instruction(x_regs, instr, opcode, constant_term),
        ),

        op::INTRINSIC => {
            let id = decode_a(instr);
            intrinsics::call_intrinsic(id, decode_b(instr) as u8, x_regs, proc, mem, realm)
                .map_err(|e| RunResult::Error(e.into()))?;
            Ok(intrinsic_cost(id))
        }

        op::CALL => handle_call(
            x_regs,
            proc,
            mem,
            realm,
            decode_a(instr) as usize,
            decode_b(instr) as u8,
        )
        .map_err(RunResult::Error),

        op::RETURN => handle_return(x_regs, proc, mem),
        op::HALT => {
            // Check eval stack: if non-empty, restore caller's context
            if proc.eval_depth > 0 {
                proc.eval_depth -= 1;
                let frame = proc.eval_stack[proc.eval_depth];
                proc.ip = frame.saved_ip;
                proc.chunk_addr = frame.saved_chunk_addr;
                proc.frame_base = frame.saved_frame_base;
                proc.current_y_count = frame.saved_y_count;
                proc.stop = frame.saved_stop;
                // Result of eval is already in x_regs[0]
                Ok(1)
            } else {
                Err(RunResult::Completed(x_regs[0]))
            }
        }

        // Build instructions
        op::BUILD_TUPLE | op::BUILD_VECTOR | op::BUILD_MAP | op::BUILD_CLOSURE => {
            execute_build_instruction(x_regs, proc, mem, instr, opcode).map_err(RunResult::Error)
        }

        // Y register instructions
        op::ALLOCATE | op::ALLOCATE_ZERO | op::DEALLOCATE | op::MOVE_XY | op::MOVE_YX => {
            execute_y_register_instruction(x_regs, proc, mem, instr, opcode)
        }

        // Pattern matching instructions (read-only)
        op::IS_NIL
        | op::IS_BOOL
        | op::IS_INT
        | op::IS_TUPLE
        | op::IS_VECTOR
        | op::IS_MAP
        | op::IS_KEYWORD
        | op::IS_STRING
        | op::TEST_ARITY
        | op::TEST_VEC_LEN
        | op::TEST_ARITY_GE
        | op::GET_TUPLE_ELEM
        | op::GET_VEC_ELEM
        | op::IS_EQ
        | op::JUMP
        | op::JUMP_IF_FALSE
        | op::BADMATCH => pattern::execute(x_regs, proc, mem, instr, opcode),

        // Pattern matching instructions (allocating)
        op::TUPLE_SLICE => pattern::execute_tuple_slice(x_regs, proc, mem, instr),

        _ => Err(RunResult::Error(RuntimeError::InvalidOpcode(opcode))),
    }
}

/// Attempt GC recovery after an OOM error.
///
/// Tries the following sequence:
/// 1. Minor GC
/// 2. If still insufficient space: grow young heap
/// 3. If grow failed: major GC
/// 4. If still insufficient: grow young heap again
///
/// GC updates `chunk_addr` via root tracking, so no reload is needed.
///
/// Returns `true` if recovery succeeded and free space is available.
fn handle_oom_with_gc<M: MemorySpace>(
    proc: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
) -> bool {
    // Step 1: Try minor GC
    if let Ok(stats) = gc::minor_gc(proc, worker, mem) {
        if proc.free_space() > 0 {
            return true;
        }
        // If minor GC reclaimed nothing, everything is live — skip to heap growth
        if stats.reclaimed_bytes == 0 {
            return try_grow_heap(proc, worker, realm, mem);
        }
    }

    // Step 2: Try major GC (minor GC reclaimed something but not enough)
    if gc::major_gc(proc, worker, realm.pool_mut(), mem).is_ok() && proc.free_space() > 0 {
        return true;
    }

    // Step 3: Last resort - grow the heap
    try_grow_heap(proc, worker, realm, mem)
}

/// Try growing the young heap as a last resort after GC.
fn try_grow_heap<M: MemorySpace>(
    proc: &mut Process,
    worker: &mut Worker,
    realm: &mut Realm,
    mem: &mut M,
) -> bool {
    let required = proc.heap_used().saturating_mul(2).max(1024);
    if gc::grow_young_heap_with_gc(proc, worker, realm.pool_mut(), mem, required).is_ok()
        && proc.free_space() > 0
    {
        return true;
    }
    false
}

/// Saturating conversion from `u64` to `i64` (clamps at `i64::MAX`).
fn u64_to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

/// Saturating conversion from `usize` to `i64` (clamps at `i64::MAX`).
fn usize_to_i64(v: usize) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

/// Build a process-info map from the current process state.
///
/// Returns a map containing process statistics as specified in
/// `docs/lonala/lona.process.md`. Returns `None` on OOM.
fn build_process_info<M: MemorySpace>(
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
) -> Option<Term> {
    // Intern all the keyword keys we need
    let k_status = realm.intern_keyword(mem, "status")?;
    let k_heap_size = realm.intern_keyword(mem, "heap-size")?;
    let k_heap_used = realm.intern_keyword(mem, "heap-used")?;
    let k_old_heap_size = realm.intern_keyword(mem, "old-heap-size")?;
    let k_old_heap_used = realm.intern_keyword(mem, "old-heap-used")?;
    let k_stack_size = realm.intern_keyword(mem, "stack-size")?;
    let k_minor_gc_count = realm.intern_keyword(mem, "minor-gc-count")?;
    let k_major_gc_count = realm.intern_keyword(mem, "major-gc-count")?;
    let k_total_reclaimed = realm.intern_keyword(mem, "total-reclaimed")?;
    let k_reductions = realm.intern_keyword(mem, "reductions")?;

    // Build value terms — sizes and counters are clamped to i64::MAX (cannot be negative)
    let v_status = realm.intern_keyword(mem, "running")?;
    let young_size = proc.hend.as_u64().saturating_sub(proc.heap.as_u64());
    let v_heap_size = Term::small_int(u64_to_i64(young_size))?;
    let v_heap_used = Term::small_int(usize_to_i64(proc.heap_used()))?;
    let old_size = proc
        .old_hend
        .as_u64()
        .saturating_sub(proc.old_heap.as_u64());
    let v_old_heap_size = Term::small_int(u64_to_i64(old_size))?;
    let old_used = proc
        .old_htop
        .as_u64()
        .saturating_sub(proc.old_heap.as_u64());
    let v_old_heap_used = Term::small_int(u64_to_i64(old_used))?;
    let v_stack_size = Term::small_int(usize_to_i64(proc.stack_used()))?;
    let v_minor_gc = Term::small_int(u64_to_i64(proc.minor_gc_count))?;
    let v_major_gc = Term::small_int(u64_to_i64(proc.major_gc_count))?;
    let v_total_reclaimed = Term::small_int(u64_to_i64(proc.total_reclaimed))?;
    let v_reductions = Term::small_int(u64_to_i64(proc.total_reductions))?;

    // Build entries list (key-value pairs as tuples in a list)
    // Build from back to front to get correct order
    let pairs: [(Term, Term); 10] = [
        (k_status, v_status),
        (k_heap_size, v_heap_size),
        (k_heap_used, v_heap_used),
        (k_old_heap_size, v_old_heap_size),
        (k_old_heap_used, v_old_heap_used),
        (k_stack_size, v_stack_size),
        (k_minor_gc_count, v_minor_gc),
        (k_major_gc_count, v_major_gc),
        (k_total_reclaimed, v_total_reclaimed),
        (k_reductions, v_reductions),
    ];

    let mut entries = Term::NIL;
    for &(key, val) in pairs.iter().rev() {
        let kv = proc.alloc_term_tuple(mem, &<[Term; 2]>::from((key, val)))?;
        entries = proc.alloc_term_pair(mem, kv, entries)?;
    }

    proc.alloc_term_map(mem, entries, pairs.len())
}

/// Stateless bytecode virtual machine.
///
/// The VM is a namespace for execution functions. All state lives in `Process`
/// (heap, IP, call stack) and `Worker` (X registers).
pub struct Vm;

impl Vm {
    /// Run bytecode until completion, yield, or error.
    ///
    /// The process must have `chunk_addr` set (pointing to a `HeapFun` on the heap).
    /// Execution state is split between Process (ip, call stack) and Worker (`x_regs`).
    ///
    /// This implementation is non-recursive: function calls use the Process's call stack
    /// instead of Rust stack recursion, enabling the VM to yield and resume at any call depth.
    ///
    /// Returns:
    /// - `RunResult::Completed(value)` when execution finishes (HALT or top-level RETURN)
    /// - `RunResult::Yielded` when the reduction budget is exhausted (can be resumed)
    /// - `RunResult::Error(e)` when a runtime error occurs
    pub fn run<M: MemorySpace>(
        worker: &mut Worker,
        proc: &mut Process,
        mem: &mut M,
        realm: &mut Realm,
        scheduler: Option<&Scheduler>,
    ) -> RunResult {
        // Track whether the last instruction already triggered a GC retry.
        // This prevents infinite retry loops when GC frees some bytes but not
        // enough for the allocation that failed.
        let mut gc_retry_pending = false;

        loop {
            // Check reduction budget
            if proc.should_yield() {
                return RunResult::Yielded;
            }

            // Get current chunk address
            let Some(chunk_addr) = proc.chunk_addr else {
                return RunResult::Error(RuntimeError::NoCode);
            };

            // Bounds check and fetch instruction directly from HeapFun
            let instr_count = HeapFun::instruction_count(mem, chunk_addr);
            if proc.ip >= instr_count {
                return RunResult::Error(RuntimeError::IpOutOfBounds);
            }
            let instr = HeapFun::read_instruction(mem, chunk_addr, proc.ip);
            proc.ip += 1;

            // Decode and pre-fetch constant for LOADK
            let opcode = decode_opcode(instr);
            let constant_term = if opcode == op::LOADK {
                let bx = decode_bx(instr);
                let const_count = HeapFun::read_const_count(mem, chunk_addr);
                if bx >= u32::from(const_count) {
                    return RunResult::Error(RuntimeError::ConstantOutOfBounds(bx));
                }
                let code_len = HeapFun::read_code_len(mem, chunk_addr);
                Some(HeapFun::read_constant(
                    mem,
                    chunk_addr,
                    code_len,
                    bx as usize,
                ))
            } else {
                None
            };

            // Special intrinsics need Worker/Realm/Scheduler access.
            if opcode == op::INTRINSIC {
                let id = decode_a(instr);
                if let Some(handled) = special_intrinsics::dispatch(
                    id,
                    decode_b(instr) as u8,
                    worker,
                    proc,
                    mem,
                    realm,
                    scheduler,
                ) {
                    match handled {
                        Ok(()) => {
                            proc.consume_reductions(intrinsic_cost(id));
                            gc_retry_pending = false;
                            continue;
                        }
                        Err(result) => return result,
                    }
                }
            }

            // Execute instruction
            match execute_instruction(
                &mut worker.x_regs,
                proc,
                mem,
                realm,
                instr,
                opcode,
                constant_term,
            ) {
                Ok(cost) => {
                    proc.consume_reductions(cost);
                    gc_retry_pending = false;
                }
                Err(RunResult::Error(e)) if e.is_oom() => {
                    if !gc_retry_pending && handle_oom_with_gc(proc, worker, realm, mem) {
                        // Rewind IP to retry the failed instruction (once)
                        proc.ip -= 1;
                        gc_retry_pending = true;
                        continue;
                    }
                    return RunResult::Error(RuntimeError::OutOfMemory);
                }
                Err(result) => return result,
            }
        }
    }
}

/// Convenience function to execute a process's bytecode to completion.
///
/// The process must have `chunk_addr` set (pointing to a `HeapFun` on the heap).
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
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
) -> Result<Term, RuntimeError> {
    execute_with_scheduler(worker, proc, mem, realm, None)
}

/// Run a process to completion with access to a scheduler.
///
/// Like `execute`, but passes a `Scheduler` so `spawn` and `alive?`
/// intrinsics work during execution.
///
/// # Errors
///
/// Returns an error if execution fails.
pub fn execute_with_scheduler<M: MemorySpace>(
    worker: &mut Worker,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
    scheduler: Option<&Scheduler>,
) -> Result<Term, RuntimeError> {
    // Initialize reduction budget
    proc.reset_reductions();

    loop {
        match Vm::run(worker, proc, mem, realm, scheduler) {
            RunResult::Completed(term) => return Ok(term),
            RunResult::Yielded => {
                // Single-threaded execution: just continue with fresh budget
                proc.reset_reductions();
            }
            RunResult::Error(e) => return Err(e),
        }
    }
}
