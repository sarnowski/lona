// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Built-in intrinsic functions for the Lona VM.
//!
//! Intrinsics are operations implemented in Rust that are called from bytecode.
//! They use a fixed calling convention:
//! - Arguments in X1, X2, ..., X(argc)
//! - Result in X0
//!
//! See `docs/architecture/virtual-machine.md` for the full specification.

mod arithmetic;
mod collection;
mod meta;
mod sequence;
mod string;

/// Maximum length for `read-string` input.
///
/// Stack-buffered to release the immutable `MemorySpace` borrow before
/// calling the reader (which needs `&mut`). 1024 bytes accommodates
/// typical REPL and eval expressions.
const MAX_READ_STRING_LEN: usize = 1024;

#[cfg(test)]
mod arithmetic_test;
#[cfg(test)]
mod boolean_test;
#[cfg(test)]
mod cost_test;
#[cfg(test)]
mod keyword_intrinsic_test;
#[cfg(test)]
mod lookup_test;
#[cfg(test)]
mod meta_intrinsics_test;
#[cfg(test)]
mod predicate_test;
#[cfg(test)]
mod sequence_test;
#[cfg(test)]
mod string_test;
#[cfg(test)]
mod tuple_intrinsic_test;

use crate::platform::MemorySpace;
use crate::process::{Process, X_REG_COUNT};
use crate::realm::Realm;
use crate::term::Term;

/// Type alias for X register array (Term-based).
pub type XRegs = [Term; X_REG_COUNT];

use arithmetic::{
    intrinsic_add, intrinsic_div, intrinsic_eq, intrinsic_ge, intrinsic_gt, intrinsic_identical,
    intrinsic_le, intrinsic_lt, intrinsic_mod, intrinsic_mul, intrinsic_not, intrinsic_sub,
};
use collection::{
    intrinsic_contains, intrinsic_count, intrinsic_get, intrinsic_is_map, intrinsic_is_symbol,
    intrinsic_is_tuple, intrinsic_is_vector, intrinsic_keys, intrinsic_nth, intrinsic_put,
    intrinsic_vals,
};

// Re-export core functions for use by callable data structures in VM
pub use arithmetic::terms_equal;
pub use collection::{CoreCollectionError, core_contains, core_get, core_nth};
use meta::{
    intrinsic_create_ns, intrinsic_def_binding, intrinsic_def_meta, intrinsic_def_root,
    intrinsic_find_ns, intrinsic_intern, intrinsic_is_fn, intrinsic_is_namespace, intrinsic_is_var,
    intrinsic_meta, intrinsic_ns_map, intrinsic_ns_name, intrinsic_var_get, intrinsic_with_meta,
};
use sequence::{intrinsic_first, intrinsic_is_empty, intrinsic_rest};
use string::{
    intrinsic_is_keyword, intrinsic_keyword, intrinsic_name_fn, intrinsic_namespace, intrinsic_str,
};

/// Intrinsic function IDs.
///
/// These match the intrinsic dispatch table order.
pub mod id {
    /// Addition: `(+ a b)` -> `a + b`
    pub const ADD: u8 = 0;
    /// Subtraction: `(- a b)` -> `a - b`
    pub const SUB: u8 = 1;
    /// Multiplication: `(* a b)` -> `a * b`
    pub const MUL: u8 = 2;
    /// Division: `(/ a b)` -> `a / b`
    pub const DIV: u8 = 3;
    /// Modulo: `(mod a b)` -> `a % b`
    pub const MOD: u8 = 4;
    /// Equality: `(= a b)` -> `a == b`
    pub const EQ: u8 = 5;
    /// Less than: `(< a b)` -> `a < b`
    pub const LT: u8 = 6;
    /// Greater than: `(> a b)` -> `a > b`
    pub const GT: u8 = 7;
    /// Less or equal: `(<= a b)` -> `a <= b`
    pub const LE: u8 = 8;
    /// Greater or equal: `(>= a b)` -> `a >= b`
    pub const GE: u8 = 9;
    /// Boolean not: `(not x)` -> `!x`
    pub const NOT: u8 = 10;
    /// Nil predicate: `(nil? x)` -> `x == nil`
    pub const IS_NIL: u8 = 11;
    /// Integer predicate: `(integer? x)` -> is x an integer?
    pub const IS_INT: u8 = 12;
    /// String predicate: `(string? x)` -> is x a string?
    pub const IS_STR: u8 = 13;
    /// String concatenation: `(str a b ...)` -> concatenated string
    pub const STR: u8 = 14;
    /// Keyword predicate: `(keyword? x)` -> is x a keyword?
    pub const IS_KEYWORD: u8 = 15;
    /// Keyword constructor: `(keyword s)` -> :s
    pub const KEYWORD: u8 = 16;
    /// Get name: `(name x)` -> name string
    pub const NAME: u8 = 17;
    /// Get namespace: `(namespace x)` -> namespace string or nil
    pub const NAMESPACE: u8 = 18;
    /// Tuple predicate: `(tuple? x)` -> is x a tuple?
    pub const IS_TUPLE: u8 = 19;
    /// Get element at index: `(nth tuple index)` -> element
    pub const NTH: u8 = 20;
    /// Get count: `(count coll)` -> length
    pub const COUNT: u8 = 21;
    /// Symbol predicate: `(symbol? x)` -> is x a symbol?
    pub const IS_SYMBOL: u8 = 22;
    /// Map predicate: `(map? x)` -> is x a map?
    pub const IS_MAP: u8 = 23;
    /// Get value from map (2-arg): `(get m k)` -> value or nil
    pub const GET: u8 = 24;
    /// Put value into map (persistent): `(put m k v)` -> new map
    pub const PUT: u8 = 25;
    /// Get keys from map: `(keys m)` -> list of keys
    pub const KEYS: u8 = 26;
    /// Get values from map: `(vals m)` -> list of values
    pub const VALS: u8 = 27;
    /// Get metadata: `(meta obj)` -> metadata map or nil
    pub const META: u8 = 28;
    /// Attach metadata: `(with-meta obj m)` -> obj with metadata
    pub const WITH_META: u8 = 29;
    /// Namespace predicate: `(namespace? x)` -> is x a namespace?
    pub const IS_NAMESPACE: u8 = 30;
    /// Create namespace: `(create-ns sym)` -> namespace
    pub const CREATE_NS: u8 = 31;
    /// Find namespace: `(find-ns sym)` -> namespace or nil
    pub const FIND_NS: u8 = 32;
    /// Get namespace name: `(ns-name ns)` -> symbol
    pub const NS_NAME: u8 = 33;
    /// Get namespace mappings: `(ns-map ns)` -> map
    pub const NS_MAP: u8 = 34;
    /// Function predicate: `(fn? x)` -> is x callable?
    pub const IS_FN: u8 = 35;
    /// Var predicate: `(var? x)` -> is x a var?
    pub const IS_VAR: u8 = 36;
    /// Intern symbol in namespace: `(intern ns sym val)` -> var
    pub const INTERN: u8 = 37;
    /// Get var value: `(var-get var)` -> value
    pub const VAR_GET: u8 = 38;
    /// Define var root: `(def-root var value)` -> var (deep copies value to realm)
    pub const DEF_ROOT: u8 = 39;
    /// Define var binding: `(def-binding var value)` -> var (sets process binding)
    pub const DEF_BINDING: u8 = 40;
    /// Define var metadata: `(def-meta var meta)` -> var (stores metadata in realm)
    pub const DEF_META: u8 = 41;
    /// Vector predicate: `(vector? x)` -> is x a vector?
    pub const IS_VECTOR: u8 = 42;
    /// Get first element: `(first coll)` -> first element or nil
    pub const FIRST: u8 = 43;
    /// Get rest of sequence: `(rest coll)` -> list of remaining elements
    pub const REST: u8 = 44;
    /// Empty predicate: `(empty? coll)` -> is collection empty?
    pub const IS_EMPTY: u8 = 45;
    /// Reference identity: `(identical? a b)` -> same object?
    pub const IDENTICAL: u8 = 46;
    /// Map contains key: `(contains? m k)` -> is key in map?
    pub const CONTAINS: u8 = 47;
    /// Garbage collect: `(garbage-collect)` or `(garbage-collect :full)`.
    ///
    /// Handled directly in `Vm::run` (needs Worker/Realm access).
    pub const GARBAGE_COLLECT: u8 = 48;
    /// Process info: `(process-info)` -> map of process statistics.
    ///
    /// Handled directly in `Vm::run` (needs direct process access).
    pub const PROCESS_INFO: u8 = 49;
    /// Spawn a new process: `(spawn f)` -> pid.
    ///
    /// Handled directly in `Vm::run` (needs `ProcessTable` access).
    pub const SPAWN: u8 = 50;
    /// Get current process PID: `(self)` -> pid.
    pub const SELF: u8 = 51;
    /// Check if process is alive: `(alive? pid)` -> boolean.
    ///
    /// Handled directly in `Vm::run` (needs `ProcessTable` access).
    pub const ALIVE: u8 = 52;
    /// PID predicate: `(pid? x)` -> boolean.
    pub const IS_PID: u8 = 53;
    /// Parse string to form: `(read-string s)` -> form.
    pub const READ_STRING: u8 = 54;
    /// Evaluate a form: `(eval form)` -> result.
    ///
    /// Handled directly in `Vm::run` (uses eval trampoline).
    pub const EVAL: u8 = 55;
    /// Send a message: `(send pid msg)` -> `:ok`.
    ///
    /// Handled directly in `Vm::run` (needs `ProcessTable` access for delivery).
    pub const SEND: u8 = 56;
    /// Ref predicate: `(ref? x)` -> boolean.
    pub const IS_REF: u8 = 57;
    /// Link to another process: `(link pid)` -> `:ok`.
    ///
    /// Handled directly in `Vm::run` (needs `ProcessTable` access).
    pub const LINK: u8 = 58;
    /// Unlink from another process: `(unlink pid)` -> `:ok`.
    ///
    /// Handled directly in `Vm::run` (needs `ProcessTable` access).
    pub const UNLINK: u8 = 59;
    /// Set trap-exit flag: `(trap-exit bool)` -> `:ok`.
    ///
    /// Handled directly in `Vm::run` (needs direct process access).
    pub const TRAP_EXIT: u8 = 60;
    /// Monitor a process: `(monitor pid)` -> ref.
    ///
    /// Handled directly in `Vm::run` (needs `ProcessTable` + `Scheduler`).
    pub const MONITOR: u8 = 61;
    /// Remove a monitor: `(demonitor ref)` -> `:ok`.
    ///
    /// Handled directly in `Vm::run` (needs `ProcessTable`).
    pub const DEMONITOR: u8 = 62;
    /// Exit current process or send exit signal: `(exit reason)` or `(exit pid reason)`.
    ///
    /// 1-arg: terminates current process with reason.
    /// 2-arg: sends exit signal to target process.
    /// Handled directly in `Vm::run` (needs `ProcessTable`).
    pub const EXIT: u8 = 63;
    /// Spawn and link: `(spawn-link f)` -> pid.
    ///
    /// Atomically spawns and links before enqueue.
    /// Handled directly in `Vm::run` (needs `ProcessTable`).
    pub const SPAWN_LINK: u8 = 64;
    /// Spawn and monitor: `(spawn-monitor f)` -> `[pid ref]`.
    ///
    /// Atomically spawns and monitors before enqueue.
    /// Handled directly in `Vm::run` (needs `ProcessTable` + `Scheduler`).
    pub const SPAWN_MONITOR: u8 = 65;
}

/// Number of defined intrinsics.
pub const INTRINSIC_COUNT: usize = 66;

/// Intrinsic IDs that belong in the `lona.process` namespace.
///
/// These are auto-referred into `lona.core` during bootstrap so existing
/// code keeps working. They can also be accessed via qualified form:
/// `(lona.process/send pid msg)`.
pub const PROCESS_INTRINSIC_IDS: &[u8] = &[
    id::PROCESS_INFO,
    id::SPAWN,
    id::SELF,
    id::ALIVE,
    id::IS_PID,
    id::SEND,
    id::IS_REF,
    id::LINK,
    id::UNLINK,
    id::TRAP_EXIT,
    id::MONITOR,
    id::DEMONITOR,
    id::EXIT,
    id::SPAWN_LINK,
    id::SPAWN_MONITOR,
];

/// Intrinsic name lookup table.
///
/// This is the single source of truth for intrinsic names. The bootstrap
/// module imports this to register intrinsics in `lona.core`.
pub const INTRINSIC_NAMES: [&str; INTRINSIC_COUNT] = [
    "+",               // 0: ADD
    "-",               // 1: SUB
    "*",               // 2: MUL
    "/",               // 3: DIV
    "mod",             // 4: MOD
    "=",               // 5: EQ
    "<",               // 6: LT
    ">",               // 7: GT
    "<=",              // 8: LE
    ">=",              // 9: GE
    "not",             // 10: NOT
    "nil?",            // 11: IS_NIL
    "integer?",        // 12: IS_INT
    "string?",         // 13: IS_STR
    "str",             // 14: STR
    "keyword?",        // 15: IS_KEYWORD
    "keyword",         // 16: KEYWORD
    "name",            // 17: NAME
    "namespace",       // 18: NAMESPACE
    "tuple?",          // 19: IS_TUPLE
    "nth",             // 20: NTH
    "count",           // 21: COUNT
    "symbol?",         // 22: IS_SYMBOL
    "map?",            // 23: IS_MAP
    "get",             // 24: GET
    "put",             // 25: PUT
    "keys",            // 26: KEYS
    "vals",            // 27: VALS
    "meta",            // 28: META
    "with-meta",       // 29: WITH_META
    "namespace?",      // 30: IS_NAMESPACE
    "create-ns",       // 31: CREATE_NS
    "find-ns",         // 32: FIND_NS
    "ns-name",         // 33: NS_NAME
    "ns-map",          // 34: NS_MAP
    "fn?",             // 35: IS_FN
    "var?",            // 36: IS_VAR
    "intern",          // 37: INTERN
    "var-get",         // 38: VAR_GET
    "def-root",        // 39: DEF_ROOT
    "def-binding",     // 40: DEF_BINDING
    "def-meta",        // 41: DEF_META
    "vector?",         // 42: IS_VECTOR
    "first",           // 43: FIRST
    "rest",            // 44: REST
    "empty?",          // 45: IS_EMPTY
    "identical?",      // 46: IDENTICAL
    "contains?",       // 47: CONTAINS
    "garbage-collect", // 48: GARBAGE_COLLECT
    "process-info",    // 49: PROCESS_INFO
    "spawn",           // 50: SPAWN
    "self",            // 51: SELF
    "alive?",          // 52: ALIVE
    "pid?",            // 53: IS_PID
    "read-string",     // 54: READ_STRING
    "eval",            // 55: EVAL
    "send",            // 56: SEND
    "ref?",            // 57: IS_REF
    "link",            // 58: LINK
    "unlink",          // 59: UNLINK
    "trap-exit",       // 60: TRAP_EXIT
    "monitor",         // 61: MONITOR
    "demonitor",       // 62: DEMONITOR
    "exit",            // 63: EXIT
    "spawn-link",      // 64: SPAWN_LINK
    "spawn-monitor",   // 65: SPAWN_MONITOR
];

/// Look up an intrinsic ID by name.
///
/// Returns `Some(id)` if the name matches a known intrinsic, `None` otherwise.
#[must_use]
pub fn lookup_intrinsic(name: &str) -> Option<u8> {
    INTRINSIC_NAMES
        .iter()
        .position(|&n| n == name)
        .map(|i| i as u8)
}

/// Get the name of an intrinsic by ID.
///
/// Returns `Some(name)` if the ID is valid, `None` otherwise.
#[must_use]
pub fn intrinsic_name(id: u8) -> Option<&'static str> {
    INTRINSIC_NAMES.get(id as usize).copied()
}

/// Get reduction cost for an intrinsic.
///
/// Returns the number of reductions to charge for executing this intrinsic.
/// Costs are approximate and grouped by operation complexity:
/// - Arithmetic, comparison, predicates: 1
/// - Simple collection ops (count, first, rest, empty): 2
/// - Collection access (get, nth, keys, vals): 3
/// - String operations: 3
/// - Collection mutation (put): 5
/// - Namespace/var operations: 10
#[must_use]
pub const fn intrinsic_cost(id: u8) -> u32 {
    match id {
        // Arithmetic, comparison, boolean logic, type predicates: cost 1
        id::ADD
        | id::SUB
        | id::MUL
        | id::DIV
        | id::MOD
        | id::EQ
        | id::LT
        | id::GT
        | id::LE
        | id::GE
        | id::NOT
        | id::IDENTICAL
        | id::IS_NIL
        | id::IS_INT
        | id::IS_STR
        | id::IS_KEYWORD
        | id::IS_SYMBOL
        | id::IS_TUPLE
        | id::IS_MAP
        | id::IS_VECTOR
        | id::IS_NAMESPACE
        | id::IS_FN
        | id::IS_VAR
        | id::SELF
        | id::IS_PID
        | id::IS_REF => 1,

        // Simple collection ops: cost 2
        id::COUNT | id::FIRST | id::IS_EMPTY => 2,

        // Collection access, string ops, metadata: cost 3
        id::NTH
        | id::GET
        | id::CONTAINS
        | id::KEYS
        | id::VALS
        | id::REST
        | id::KEYWORD
        | id::NAME
        | id::NAMESPACE
        | id::STR
        | id::META
        | id::WITH_META => 3,

        // Namespace/var operations, GC, and process info: cost 10
        id::CREATE_NS
        | id::FIND_NS
        | id::NS_NAME
        | id::NS_MAP
        | id::INTERN
        | id::VAR_GET
        | id::DEF_ROOT
        | id::DEF_BINDING
        | id::DEF_META
        | id::GARBAGE_COLLECT
        | id::PROCESS_INFO
        | id::SPAWN
        | id::ALIVE
        | id::READ_STRING
        | id::EVAL
        | id::SEND
        | id::LINK
        | id::UNLINK
        | id::TRAP_EXIT
        | id::MONITOR
        | id::DEMONITOR
        | id::EXIT
        | id::SPAWN_LINK
        | id::SPAWN_MONITOR => 10,

        // Unknown and PUT: default cost 5
        _ => 5,
    }
}

/// Runtime error from intrinsic execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntrinsicError {
    /// Type error: expected a specific type.
    TypeError {
        /// Which intrinsic was called.
        intrinsic: u8,
        /// Which argument (0-indexed).
        arg: u8,
        /// What type was expected.
        expected: &'static str,
    },
    /// Division by zero.
    DivisionByZero,
    /// Integer overflow.
    Overflow,
    /// Unknown intrinsic ID.
    UnknownIntrinsic(u8),
    /// Out of memory during string allocation.
    OutOfMemory,
    /// Index out of bounds.
    IndexOutOfBounds {
        /// The index that was requested.
        index: i64,
        /// The length of the collection.
        len: usize,
    },
}

/// Execute an intrinsic function.
///
/// # Arguments
/// * `intrinsic_id` - The intrinsic to call
/// * `argc` - Number of arguments
/// * `x_regs` - X registers (from Worker) - args in X1..X(argc), result written to X0
/// * `proc` - Process for heap operations
/// * `mem` - Memory space
/// * `realm` - Realm for intrinsics that need it (`DEF_ROOT`, `DEF_BINDING`)
///
/// # Errors
/// Returns an error if the intrinsic fails (type error, division by zero, etc.)
pub fn call_intrinsic<M: MemorySpace>(
    intrinsic_id: u8,
    argc: u8,
    x_regs: &mut XRegs,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
) -> Result<(), IntrinsicError> {
    x_regs[0] = match intrinsic_id {
        // Arithmetic
        id::ADD => intrinsic_add(x_regs, intrinsic_id)?,
        id::SUB => intrinsic_sub(x_regs, intrinsic_id)?,
        id::MUL => intrinsic_mul(x_regs, intrinsic_id)?,
        id::DIV => intrinsic_div(x_regs, intrinsic_id)?,
        id::MOD => intrinsic_mod(x_regs, intrinsic_id)?,

        // Comparison
        id::EQ => intrinsic_eq(x_regs, proc, mem),
        id::LT => intrinsic_lt(x_regs, intrinsic_id)?,
        id::GT => intrinsic_gt(x_regs, intrinsic_id)?,
        id::LE => intrinsic_le(x_regs, intrinsic_id)?,
        id::GE => intrinsic_ge(x_regs, intrinsic_id)?,
        id::IDENTICAL => intrinsic_identical(x_regs),

        // Boolean
        id::NOT => intrinsic_not(x_regs),

        // Type predicates
        id::IS_NIL => Term::bool(x_regs[1].is_nil()),
        id::IS_INT => Term::bool(x_regs[1].is_small_int()),
        id::IS_STR => Term::bool(proc.is_term_string(mem, x_regs[1])),
        id::IS_KEYWORD => intrinsic_is_keyword(x_regs, proc),
        id::IS_SYMBOL => intrinsic_is_symbol(x_regs, proc),
        id::IS_TUPLE => intrinsic_is_tuple(x_regs, proc, mem),
        id::IS_VECTOR => intrinsic_is_vector(x_regs, proc, mem),
        id::IS_MAP => intrinsic_is_map(x_regs, proc, mem),
        id::IS_NAMESPACE => intrinsic_is_namespace(x_regs, proc, mem),
        id::IS_FN => intrinsic_is_fn(x_regs, proc, mem),
        id::IS_VAR => intrinsic_is_var(x_regs, proc, mem),

        // String operations
        id::STR => intrinsic_str(x_regs, argc, proc, realm, mem)?,
        id::KEYWORD => intrinsic_keyword(x_regs, proc, realm, mem, intrinsic_id)?,
        id::NAME => intrinsic_name_fn(x_regs, proc, realm, mem, intrinsic_id)?,
        id::NAMESPACE => intrinsic_namespace(x_regs, proc, realm, mem, intrinsic_id)?,

        // Collection operations
        id::NTH => intrinsic_nth(x_regs, argc, proc, mem, intrinsic_id)?,
        id::COUNT => intrinsic_count(x_regs, proc, mem, intrinsic_id)?,
        id::GET => intrinsic_get(x_regs, argc, proc, mem, intrinsic_id)?,
        id::PUT => intrinsic_put(x_regs, proc, mem, intrinsic_id)?,
        id::KEYS => intrinsic_keys(x_regs, proc, mem, intrinsic_id)?,
        id::VALS => intrinsic_vals(x_regs, proc, mem, intrinsic_id)?,
        id::CONTAINS => intrinsic_contains(x_regs, proc, mem, intrinsic_id)?,

        // Sequence operations
        id::FIRST => intrinsic_first(x_regs, proc, mem, intrinsic_id)?,
        id::REST => intrinsic_rest(x_regs, proc, mem, intrinsic_id)?,
        id::IS_EMPTY => intrinsic_is_empty(x_regs, proc, mem, intrinsic_id)?,

        // Metadata operations
        id::META => intrinsic_meta(x_regs, realm, mem),
        id::WITH_META => intrinsic_with_meta(x_regs, realm, mem, intrinsic_id)?,

        // Namespace operations
        id::CREATE_NS => intrinsic_create_ns(x_regs, realm, mem, intrinsic_id)?,
        id::FIND_NS => intrinsic_find_ns(x_regs, realm, intrinsic_id)?,
        id::NS_NAME => intrinsic_ns_name(x_regs, proc, mem, intrinsic_id)?,
        id::NS_MAP => intrinsic_ns_map(x_regs, proc, mem, intrinsic_id)?,

        // Var operations
        id::INTERN => intrinsic_intern(x_regs, proc, mem, intrinsic_id)?,
        id::VAR_GET => intrinsic_var_get(x_regs, proc, mem, intrinsic_id)?,
        id::DEF_ROOT => intrinsic_def_root(x_regs, proc, realm, mem, intrinsic_id)?,
        id::DEF_BINDING => intrinsic_def_binding(x_regs, proc, mem, intrinsic_id)?,
        id::DEF_META => intrinsic_def_meta(x_regs, proc, realm, mem, intrinsic_id)?,

        // These intrinsics are handled directly in Vm::run before reaching
        // this dispatch (they need Worker, ProcessTable, or eval trampoline).
        // If we get here, return a no-op.
        id::GARBAGE_COLLECT
        | id::PROCESS_INFO
        | id::SPAWN
        | id::ALIVE
        | id::EVAL
        | id::SEND
        | id::LINK
        | id::UNLINK
        | id::TRAP_EXIT
        | id::MONITOR
        | id::DEMONITOR
        | id::EXIT
        | id::SPAWN_LINK
        | id::SPAWN_MONITOR => {
            return Ok(());
        }

        // Regular process intrinsics
        id::SELF => {
            x_regs[0] = proc.pid_term.unwrap_or(Term::NIL);
            return Ok(());
        }
        id::IS_PID => Term::bool(proc.is_term_pid(mem, x_regs[1])),
        id::IS_REF => Term::bool(proc.is_term_ref(mem, x_regs[1])),
        id::READ_STRING => {
            // Read string content into a stack buffer to release the immutable borrow
            // on `mem` before calling the reader (which needs `&mut mem`).
            let s = proc
                .read_term_string(mem, x_regs[1])
                .ok_or(IntrinsicError::TypeError {
                    intrinsic: id::READ_STRING,
                    arg: 0,
                    expected: "string",
                })?;
            if s.len() > MAX_READ_STRING_LEN {
                return Err(IntrinsicError::OutOfMemory);
            }
            let mut buf = [0u8; MAX_READ_STRING_LEN];
            let len = s.len();
            buf[..len].copy_from_slice(s.as_bytes());
            // Now mem borrow is released
            let str_ref = core::str::from_utf8(&buf[..len]).unwrap_or("");
            crate::reader::read(str_ref, proc, realm, mem)
                .map_err(|_| IntrinsicError::OutOfMemory)?
                .unwrap_or(Term::NIL)
        }

        _ => return Err(IntrinsicError::UnknownIntrinsic(intrinsic_id)),
    };
    Ok(())
}

/// Extract an integer from a Term register, returning a type error if not an int.
pub(crate) const fn expect_int(
    x_regs: &XRegs,
    reg: usize,
    intrinsic: u8,
    arg: u8,
) -> Result<i64, IntrinsicError> {
    match x_regs[reg].as_small_int() {
        Some(n) => Ok(n),
        None => Err(IntrinsicError::TypeError {
            intrinsic,
            arg,
            expected: "integer",
        }),
    }
}
