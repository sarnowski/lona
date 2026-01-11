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
mod string;

#[cfg(test)]
mod arithmetic_test;
#[cfg(test)]
mod boolean_test;
#[cfg(test)]
mod keyword_intrinsic_test;
#[cfg(test)]
mod lookup_test;
#[cfg(test)]
mod meta_intrinsics_test;
#[cfg(test)]
mod predicate_test;
#[cfg(test)]
mod string_test;
#[cfg(test)]
mod tuple_intrinsic_test;

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::value::Value;

use arithmetic::{
    intrinsic_add, intrinsic_div, intrinsic_eq, intrinsic_ge, intrinsic_gt, intrinsic_le,
    intrinsic_lt, intrinsic_mod, intrinsic_mul, intrinsic_not, intrinsic_sub,
};
use collection::{
    intrinsic_count, intrinsic_get, intrinsic_is_map, intrinsic_is_symbol, intrinsic_is_tuple,
    intrinsic_keys, intrinsic_nth, intrinsic_put, intrinsic_vals,
};

// Re-export core functions for use by callable data structures in VM
pub use collection::{CoreCollectionError, core_get, core_nth};
use meta::{
    intrinsic_create_ns, intrinsic_def_binding, intrinsic_def_meta, intrinsic_def_root,
    intrinsic_find_ns, intrinsic_intern, intrinsic_is_fn, intrinsic_is_namespace, intrinsic_is_var,
    intrinsic_meta, intrinsic_ns_map, intrinsic_ns_name, intrinsic_var_get, intrinsic_with_meta,
};
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
}

/// Number of defined intrinsics.
pub const INTRINSIC_COUNT: usize = 42;

/// Intrinsic name lookup table.
const INTRINSIC_NAMES: [&str; INTRINSIC_COUNT] = [
    "+",           // 0: ADD
    "-",           // 1: SUB
    "*",           // 2: MUL
    "/",           // 3: DIV
    "mod",         // 4: MOD
    "=",           // 5: EQ
    "<",           // 6: LT
    ">",           // 7: GT
    "<=",          // 8: LE
    ">=",          // 9: GE
    "not",         // 10: NOT
    "nil?",        // 11: IS_NIL
    "integer?",    // 12: IS_INT
    "string?",     // 13: IS_STR
    "str",         // 14: STR
    "keyword?",    // 15: IS_KEYWORD
    "keyword",     // 16: KEYWORD
    "name",        // 17: NAME
    "namespace",   // 18: NAMESPACE
    "tuple?",      // 19: IS_TUPLE
    "nth",         // 20: NTH
    "count",       // 21: COUNT
    "symbol?",     // 22: IS_SYMBOL
    "map?",        // 23: IS_MAP
    "get",         // 24: GET
    "put",         // 25: PUT
    "keys",        // 26: KEYS
    "vals",        // 27: VALS
    "meta",        // 28: META
    "with-meta",   // 29: WITH_META
    "namespace?",  // 30: IS_NAMESPACE
    "create-ns",   // 31: CREATE_NS
    "find-ns",     // 32: FIND_NS
    "ns-name",     // 33: NS_NAME
    "ns-map",      // 34: NS_MAP
    "fn?",         // 35: IS_FN
    "var?",        // 36: IS_VAR
    "intern",      // 37: INTERN
    "var-get",     // 38: VAR_GET
    "def-root",    // 39: DEF_ROOT
    "def-binding", // 40: DEF_BINDING
    "def-meta",    // 41: DEF_META
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
/// * `proc` - Process containing registers and heap
/// * `mem` - Memory space
/// * `realm` - Realm for intrinsics that need it (`DEF_ROOT`, `DEF_BINDING`)
///
/// # Errors
/// Returns an error if the intrinsic fails (type error, division by zero, etc.)
pub fn call_intrinsic<M: MemorySpace>(
    intrinsic_id: u8,
    argc: u8,
    proc: &mut Process,
    mem: &mut M,
    realm: &mut Realm,
) -> Result<(), IntrinsicError> {
    let result = match intrinsic_id {
        id::ADD => intrinsic_add(proc, intrinsic_id)?,
        id::SUB => intrinsic_sub(proc, intrinsic_id)?,
        id::MUL => intrinsic_mul(proc, intrinsic_id)?,
        id::DIV => intrinsic_div(proc, intrinsic_id)?,
        id::MOD => intrinsic_mod(proc, intrinsic_id)?,
        id::EQ => intrinsic_eq(proc, mem),
        id::LT => intrinsic_lt(proc, intrinsic_id)?,
        id::GT => intrinsic_gt(proc, intrinsic_id)?,
        id::LE => intrinsic_le(proc, intrinsic_id)?,
        id::GE => intrinsic_ge(proc, intrinsic_id)?,
        id::NOT => intrinsic_not(proc),
        id::IS_NIL => intrinsic_is_nil(proc),
        id::IS_INT => intrinsic_is_int(proc),
        id::IS_STR => intrinsic_is_str(proc),
        id::STR => intrinsic_str(proc, argc, mem)?,
        id::IS_KEYWORD => intrinsic_is_keyword(proc),
        id::KEYWORD => intrinsic_keyword(proc, mem, intrinsic_id)?,
        id::NAME => intrinsic_name_fn(proc, mem, intrinsic_id)?,
        id::NAMESPACE => intrinsic_namespace(proc, mem, intrinsic_id)?,
        id::IS_TUPLE => intrinsic_is_tuple(proc),
        id::NTH => intrinsic_nth(proc, argc, mem, intrinsic_id)?,
        id::COUNT => intrinsic_count(proc, mem, intrinsic_id)?,
        id::IS_SYMBOL => intrinsic_is_symbol(proc),
        id::IS_MAP => intrinsic_is_map(proc),
        id::GET => intrinsic_get(proc, argc, mem, intrinsic_id)?,
        id::PUT => intrinsic_put(proc, mem, intrinsic_id)?,
        id::KEYS => intrinsic_keys(proc, mem, intrinsic_id)?,
        id::VALS => intrinsic_vals(proc, mem, intrinsic_id)?,
        id::META => intrinsic_meta(proc, realm, mem),
        id::WITH_META => intrinsic_with_meta(proc, mem, intrinsic_id)?,
        id::IS_NAMESPACE => intrinsic_is_namespace(proc),
        id::CREATE_NS => intrinsic_create_ns(proc, mem, intrinsic_id)?,
        id::FIND_NS => intrinsic_find_ns(proc, mem, intrinsic_id)?,
        id::NS_NAME => intrinsic_ns_name(proc, mem, intrinsic_id)?,
        id::NS_MAP => intrinsic_ns_map(proc, mem, intrinsic_id)?,
        id::IS_FN => intrinsic_is_fn(proc),
        id::IS_VAR => intrinsic_is_var(proc),
        id::INTERN => intrinsic_intern(proc, mem, intrinsic_id)?,
        id::VAR_GET => intrinsic_var_get(proc, mem, intrinsic_id)?,
        id::DEF_ROOT => intrinsic_def_root(proc, realm, mem, intrinsic_id)?,
        id::DEF_BINDING => intrinsic_def_binding(proc, mem, intrinsic_id)?,
        id::DEF_META => intrinsic_def_meta(proc, realm, mem, intrinsic_id)?,
        _ => return Err(IntrinsicError::UnknownIntrinsic(intrinsic_id)),
    };
    proc.x_regs[0] = result;
    Ok(())
}

/// Extract an integer from a register, returning a type error if not an int.
pub(crate) const fn expect_int(
    proc: &Process,
    reg: usize,
    intrinsic: u8,
    arg: u8,
) -> Result<i64, IntrinsicError> {
    match proc.x_regs[reg] {
        Value::Int(n) => Ok(n),
        _ => Err(IntrinsicError::TypeError {
            intrinsic,
            arg,
            expected: "integer",
        }),
    }
}

// --- Type predicate intrinsics (kept in mod.rs as they're very small) ---

const fn intrinsic_is_nil(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_nil())
}

const fn intrinsic_is_int(proc: &Process) -> Value {
    Value::bool(matches!(proc.x_regs[1], Value::Int(_)))
}

const fn intrinsic_is_str(proc: &Process) -> Value {
    Value::bool(matches!(proc.x_regs[1], Value::String(_)))
}
